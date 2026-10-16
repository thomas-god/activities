//! Command line interface.
//!
//! Invoked as `app admin <command>` or `app backup run`. These commands operate directly on the
//! application's data, so they work even when the server is not running and do not require a
//! session or the full server configuration.

use std::{ffi::OsStr, ffi::OsString, path::PathBuf};

use anyhow::anyhow;
use clap::{Args, Parser, Subcommand};

use crate::{
    backup::BackupService,
    clock::Clock,
    config::{BackupConfig, StdEnvironment, data_path_from_env},
    inbound::{
        auth::multi_user::{
            EmailAddress, Password,
            admin::{
                AdminActor, AdminService, BootstrapAdminMode, BootstrapAdminOutcome,
                PromoteAdminOutcome, RevokeAdminOutcome,
            },
        },
        http::SqliteUsersRepository,
    },
};

#[derive(Debug, Parser)]
#[command(name = "app", about = "Self-hosted sport training journal")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Admin {
        #[command(subcommand)]
        command: AdminCommand,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
}

#[derive(Debug, PartialEq)]
pub enum CliCommand {
    Admin(AdminCommand),
    Backup(BackupCommand),
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum AdminCommand {
    Bootstrap(BootstrapArgs),
    Promote(PromoteArgs),
    Revoke(RevokeArgs),
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum BackupCommand {
    Run,
}

#[derive(Debug, PartialEq, Args)]
pub struct BootstrapArgs {
    /// Email address of the account to create or promote.
    #[arg(long)]
    pub email: String,
    /// Password to set. Visible in the process list; prefer `--password-file`.
    #[arg(
        long,
        conflicts_with = "password_file",
        required_unless_present = "password_file"
    )]
    pub password: Option<String>,
    /// File containing the password (e.g. a Docker secret).
    #[arg(
        long,
        conflicts_with = "password",
        required_unless_present = "password"
    )]
    pub password_file: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Args)]
pub struct PromoteArgs {
    /// Email address of the admin to promote.
    #[arg(long)]
    pub email: String,
}

#[derive(Debug, PartialEq, Args)]
pub struct RevokeArgs {
    /// Email address of the admin to demote.
    #[arg(long)]
    pub email: String,
}

/// Parses the process arguments, returning `None` when they do not request a command.
pub fn parse_env() -> Option<Result<CliCommand, clap::Error>> {
    match std::env::args_os().nth(1).as_deref() {
        Some(command) if command == OsStr::new("admin") || command == OsStr::new("backup") => {
            Some(parse_from(std::env::args_os()))
        }
        _ => None,
    }
}

/// Parses any `app <command> ...` invocation from the given arguments.
pub fn parse_from<I, T>(args: I) -> Result<CliCommand, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Cli::try_parse_from(args).map(|cli| match cli.command {
        Command::Admin { command } => CliCommand::Admin(command),
        Command::Backup { command } => CliCommand::Backup(command),
    })
}

/// Parses `app admin <command> ...` from the given arguments.
pub fn parse_admin_from<I, T>(args: I) -> Result<AdminCommand, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    parse_from(args).map(|command| match command {
        CliCommand::Admin(command) => command,
        CliCommand::Backup(_) => unreachable!("expected an admin command"),
    })
}

/// Runs a command line command.
pub async fn run(command: CliCommand) -> anyhow::Result<()> {
    match command {
        CliCommand::Admin(command) => run_admin(command).await,
        CliCommand::Backup(command) => run_backup(command).await,
    }
}

/// Runs an admin command against the users database.
async fn run_admin(command: AdminCommand) -> anyhow::Result<()> {
    let data_path = data_path_from_env(&StdEnvironment {}).map_err(|err| anyhow!(err))?;
    let repository = open_users_repository(&data_path).await?;
    let service = AdminService::new(repository, Clock::new());

    match command {
        AdminCommand::Bootstrap(args) => bootstrap(&service, args).await,
        AdminCommand::Promote(args) => promote(&service, args).await,
        AdminCommand::Revoke(args) => revoke(&service, args).await,
    }
}

/// Runs a backup command against the databases.
async fn run_backup(command: BackupCommand) -> anyhow::Result<()> {
    let BackupCommand::Run = command;

    let config = BackupConfig::from_env(&StdEnvironment {}).map_err(|err| anyhow!(err))?;
    let service = BackupService::new(
        config,
        Clock::new(),
        tokio_util::sync::CancellationToken::new(),
    );
    let started = std::time::Instant::now();
    let archive = service.run_once().await?;

    println!(
        "Backup written to {} in {:.2?}.",
        archive.display(),
        started.elapsed()
    );

    Ok(())
}

async fn open_users_repository(data_path: &str) -> anyhow::Result<SqliteUsersRepository> {
    let db_dir = PathBuf::from(data_path).join("db");
    if !db_dir.exists() {
        tokio::fs::create_dir_all(&db_dir).await?;
    }

    let user_db = db_dir.join("user.db");
    Ok(SqliteUsersRepository::new(&format!("sqlite:{}", user_db.to_string_lossy())).await?)
}

async fn bootstrap(
    service: &AdminService<SqliteUsersRepository, Clock>,
    args: BootstrapArgs,
) -> anyhow::Result<()> {
    let email = parse_email(&args.email)?;
    let password = read_password(&args)?;

    match service
        .bootstrap_admin(&AdminActor::Cli, email, password, BootstrapAdminMode::Force)
        .await?
    {
        BootstrapAdminOutcome::Created => println!("Created admin `{}`.", args.email),
        BootstrapAdminOutcome::Promoted => println!("Promoted `{}` to admin.", args.email),
        BootstrapAdminOutcome::PasswordUpdated => {
            println!("Updated the password of admin `{}`.", args.email)
        }
        BootstrapAdminOutcome::SkippedAdminExists => {
            println!(
                "An admin already exists; `{}` was not promoted.",
                args.email
            )
        }
    }

    Ok(())
}

async fn promote(
    service: &AdminService<SqliteUsersRepository, Clock>,
    args: PromoteArgs,
) -> anyhow::Result<()> {
    let email = parse_email(&args.email)?;

    match service.promote_admin(&AdminActor::Cli, email).await? {
        PromoteAdminOutcome::Promoted => println!("Promoted admin `{}`.", args.email),
        PromoteAdminOutcome::AlreadyAdmin => {
            println!("`{}` is already an admin, nothing to do.", args.email)
        }
        PromoteAdminOutcome::UnknownUser => anyhow::bail!("no user with email `{}`", args.email),
    }

    Ok(())
}

async fn revoke(
    service: &AdminService<SqliteUsersRepository, Clock>,
    args: RevokeArgs,
) -> anyhow::Result<()> {
    let email = parse_email(&args.email)?;

    match service.revoke_admin(&AdminActor::Cli, email).await? {
        RevokeAdminOutcome::Revoked => println!("Revoked admin `{}`.", args.email),
        RevokeAdminOutcome::NotAdmin => {
            println!("`{}` is not an admin, nothing to do.", args.email)
        }
        RevokeAdminOutcome::LastAdmin => {
            anyhow::bail!("refusing to revoke `{}`: it is the last admin", args.email)
        }
        RevokeAdminOutcome::UnknownUser => anyhow::bail!("no user with email `{}`", args.email),
    }

    Ok(())
}

fn parse_email(value: &str) -> anyhow::Result<EmailAddress> {
    EmailAddress::try_from(value).map_err(|()| anyhow!("invalid email address: `{value}`"))
}

fn read_password(args: &BootstrapArgs) -> anyhow::Result<Password> {
    let password = match (&args.password, &args.password_file) {
        (Some(password), None) => password.clone(),
        (None, Some(path)) => std::fs::read_to_string(path)
            .map_err(|err| anyhow!("unable to read password file `{}`: {err}", path.display()))?
            .trim_end_matches(['\n', '\r'])
            .to_string(),
        (None, None) => {
            return Err(anyhow!(
                "a password is required: pass `--password` or `--password-file`"
            ));
        }
        // `conflicts_with` already rejects supplying both.
        (Some(_), Some(_)) => unreachable!("clap rejects two password sources"),
    };

    Password::try_from(password).map_err(|()| anyhow!("the password must not be empty"))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn parse(args: &[&str]) -> Result<AdminCommand, clap::Error> {
        parse_admin_from(args.iter().copied())
    }

    fn bootstrap_args(password: Option<&str>, password_file: Option<&str>) -> BootstrapArgs {
        BootstrapArgs {
            email: "admin@example.com".to_string(),
            password: password.map(str::to_string),
            password_file: password_file.map(PathBuf::from),
        }
    }

    fn assert_password_is(password: Password, expected: &str) {
        assert!(
            password
                .as_hash()
                .unwrap()
                .verify_password(&Password::try_from(expected.to_string()).unwrap())
        );
    }

    #[test]
    fn test_parse_bootstrap_with_password() {
        let command = parse(&[
            "app",
            "admin",
            "bootstrap",
            "--email",
            "admin@example.com",
            "--password",
            "s3cret",
        ])
        .unwrap();

        assert_eq!(
            command,
            AdminCommand::Bootstrap(bootstrap_args(Some("s3cret"), None))
        );
    }

    #[test]
    fn test_parse_bootstrap_with_password_file() {
        let command = parse(&[
            "app",
            "admin",
            "bootstrap",
            "--email",
            "admin@example.com",
            "--password-file",
            "/run/secrets/admin",
        ])
        .unwrap();

        assert_eq!(
            command,
            AdminCommand::Bootstrap(bootstrap_args(None, Some("/run/secrets/admin")))
        );
    }

    #[test]
    fn test_parse_bootstrap_requires_email() {
        let err = parse(&["app", "admin", "bootstrap", "--password", "s3cret"]).unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_parse_bootstrap_requires_a_password_source() {
        let err =
            parse(&["app", "admin", "bootstrap", "--email", "admin@example.com"]).unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_parse_bootstrap_rejects_two_password_sources() {
        let err = parse(&[
            "app",
            "admin",
            "bootstrap",
            "--email",
            "admin@example.com",
            "--password",
            "s3cret",
            "--password-file",
            "/run/secrets/admin",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn test_parse_promote() {
        let command = parse(&["app", "admin", "promote", "--email", "admin@example.com"]).unwrap();

        assert_eq!(
            command,
            AdminCommand::Promote(PromoteArgs {
                email: "admin@example.com".to_string()
            })
        );
    }

    #[test]
    fn test_parse_promote_requires_email() {
        let err = parse(&["app", "admin", "promote"]).unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_parse_revoke() {
        let command = parse(&["app", "admin", "revoke", "--email", "admin@example.com"]).unwrap();

        assert_eq!(
            command,
            AdminCommand::Revoke(RevokeArgs {
                email: "admin@example.com".to_string()
            })
        );
    }

    #[test]
    fn test_parse_unknown_command_errors() {
        assert!(parse(&["app", "admin", "nope"]).is_err());
    }

    #[test]
    fn test_parse_backup_run() {
        assert_eq!(
            parse_from(["app", "backup", "run"]).unwrap(),
            CliCommand::Backup(BackupCommand::Run)
        );
    }

    #[test]
    fn test_parse_backup_requires_a_subcommand() {
        assert!(parse_from(["app", "backup"]).is_err());
    }

    #[test]
    fn test_parse_unknown_backup_subcommand_errors() {
        assert!(parse_from(["app", "backup", "nope"]).is_err());
    }

    #[test]
    fn test_parse_admin_wraps_admin_command() {
        assert_eq!(
            parse_from(["app", "admin", "promote", "--email", "admin@example.com"]).unwrap(),
            CliCommand::Admin(AdminCommand::Promote(PromoteArgs {
                email: "admin@example.com".to_string()
            }))
        );
    }

    #[test]
    fn test_read_password_from_argument() {
        assert_password_is(
            read_password(&bootstrap_args(Some("s3cret"), None)).unwrap(),
            "s3cret",
        );
    }

    #[test]
    fn test_read_password_from_file_trims_trailing_newline() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "file-secret").unwrap();

        let password =
            read_password(&bootstrap_args(None, Some(file.path().to_str().unwrap()))).unwrap();

        assert_password_is(password, "file-secret");
    }

    #[test]
    fn test_read_password_missing_errors() {
        assert!(read_password(&bootstrap_args(None, None)).is_err());
    }

    #[test]
    fn test_read_password_empty_errors() {
        assert!(read_password(&bootstrap_args(Some(""), None)).is_err());
    }
}
