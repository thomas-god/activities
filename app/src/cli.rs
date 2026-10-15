//! Break-glass admin command line interface.
//!
//! Invoked as `app admin <command>`. It operates directly on the users database, so it works even
//! when the server is not running and does not require a session or the full server configuration.

use std::{ffi::OsStr, ffi::OsString, path::PathBuf};

use anyhow::anyhow;
use clap::{Args, Parser, Subcommand};

use crate::{
    clock::Clock,
    config::{StdEnvironment, data_path_from_env},
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
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum AdminCommand {
    Bootstrap(BootstrapArgs),
    Promote(PromoteArgs),
    Revoke(RevokeArgs),
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

/// Parses the process arguments, returning `None` when they do not request an admin command.
pub fn parse_env() -> Option<Result<AdminCommand, clap::Error>> {
    if std::env::args_os().nth(1).as_deref() != Some(OsStr::new("admin")) {
        return None;
    }

    Some(parse_admin_from(std::env::args_os()))
}

/// Parses `app admin <command> ...` from the given arguments.
pub fn parse_admin_from<I, T>(args: I) -> Result<AdminCommand, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Cli::try_parse_from(args).map(|cli| match cli.command {
        Command::Admin { command } => command,
    })
}

/// Runs an admin command against the users database.
pub async fn run(command: AdminCommand) -> anyhow::Result<()> {
    let data_path = data_path_from_env(&StdEnvironment {}).map_err(|err| anyhow!(err))?;
    let repository = open_users_repository(&data_path).await?;
    let service = AdminService::new(repository, Clock::new());

    match command {
        AdminCommand::Bootstrap(args) => bootstrap(&service, args).await,
        AdminCommand::Promote(args) => promote(&service, args).await,
        AdminCommand::Revoke(args) => revoke(&service, args).await,
    }
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
