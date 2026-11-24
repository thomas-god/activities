use std::str::FromStr;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use sqlx::{
    ConnectOptions, Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::{
    domain::models::UserId,
    inbound::auth::multi_user::{
        EmailAddress, EmailChangeFinalizeResult, HashedAuthLink, HashedAuthToken,
        HashedEmailChangeToken, HashedPassword, HashedSession, HashedSessionToken, UserInfo,
        admin::{
            AdminAction, AdminAuditContext, AdminAuditEntry, AdminRepository, AdminUser,
            BootstrapAdminMode, BootstrapAdminOutcome, CreateUserOutcome, PromoteAdminOutcome,
            RevokeAdminOutcome, SetAdminOutcome, SetUserEmailOutcome, SetUserPasswordOutcome,
        },
        service::UserRepository,
        session::SessionRepository,
    },
};

/// SQLite repository backed by the user database (`user.db`).
///
/// It implements the [`UserRepository`], and [`SessionRepository`] domain traits.
#[derive(Debug, Clone)]
pub struct SqliteUsersRepository {
    writer: SqlitePool,
    readers: SqlitePool,
}

impl SqliteUsersRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let writer_options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .log_slow_statements(
                log::LevelFilter::Warn,
                std::time::Duration::from_millis(100),
            )
            .journal_mode(SqliteJournalMode::Wal);

        let writer = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(writer_options)
            .await?;

        // Run migrations
        sqlx::migrate!("migrations/users").run(&writer).await?;

        let readers_options = SqliteConnectOptions::from_str(url)?
            .journal_mode(SqliteJournalMode::Wal)
            .read_only(true);
        let readers = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(readers_options)
            .await?;

        Ok(Self { writer, readers })
    }
}

impl SqliteUsersRepository {
    /// Deletes the pending email change token, returning the target email carried by the deleted
    /// row (or `None` if token does not exist). Err if the token's email cannot be parsed.
    #[tracing::instrument(skip_all, err(Debug))]
    async fn delete_email_change_token(
        tx: &mut Transaction<'_, Sqlite>,
        user: &UserId,
        hash: &HashedAuthToken,
    ) -> Result<Option<EmailAddress>, ()> {
        let deleted: Option<String> = sqlx::query_scalar(
            r#"
        DELETE FROM t_email_change_tokens
        WHERE
            token_hash = ?1
            AND user = (SELECT rowid FROM t_users WHERE user_id = ?2)
        RETURNING new_email
        ;"#,
        )
        .bind(hash.to_string())
        .bind(user.to_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to consume email change token in database: {}", err);
        })?;

        match deleted {
            None => Ok(None),
            Some(email) => match EmailAddress::try_from(email) {
                Ok(email) => Ok(Some(email)),
                Err(err) => {
                    tracing::warn!(
                        "Consumed email change token carries an invalid target email: {}",
                        err
                    );
                    Err(())
                }
            },
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn is_email_unique(
        tx: &mut Transaction<'_, Sqlite>,
        user: &UserId,
        email: &str,
    ) -> Result<bool, sqlx::Error> {
        let in_use: i64 = sqlx::query_scalar(
            r#"
        SELECT COUNT(*)
        FROM t_emails e
        WHERE e.email = ?1 AND e.user != (SELECT rowid FROM t_users WHERE user_id = ?2)
        ;"#,
        )
        .bind(email)
        .bind(user.to_string())
        .fetch_one(&mut **tx)
        .await?;
        Ok(in_use == 0)
    }

    /// Swaps the user's email and marks it verified.
    #[tracing::instrument(skip_all, err)]
    async fn set_user_email_verified(
        tx: &mut Transaction<'_, Sqlite>,
        user: &UserId,
        email: &str,
    ) -> Result<(), anyhow::Error> {
        let res = sqlx::query(
            r#"
        UPDATE t_emails
        SET email = ?1, verified = TRUE
        WHERE user = (SELECT rowid FROM t_users WHERE user_id = ?2)
        ;"#,
        )
        .bind(email)
        .bind(user.to_string())
        .execute(&mut **tx)
        .await
        .map_err(|err| anyhow!(err))?;
        if res.rows_affected() != 1 {
            return Err(anyhow!(
                "Not exactly one row affected: {}",
                res.rows_affected()
            ));
        }
        Ok(())
    }

    /// Inserts one admin audit row within the caller's transaction, so a mutation and its audit
    /// entry commit (or roll back) together.
    #[tracing::instrument(skip_all, err(Debug))]
    async fn insert_admin_audit(
        tx: &mut Transaction<'_, Sqlite>,
        entry: &AdminAuditEntry,
    ) -> Result<(), ()> {
        sqlx::query(
            r#"
        INSERT INTO t_admin_audit_log (
            occurred_at, origin, actor_user_id, action, target_user_id, target_email, outcome, details
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);
        "#,
        )
        .bind(entry.occurred_at().to_rfc3339())
        .bind(entry.origin())
        .bind(entry.actor_user_id().map(|user| user.to_string()))
        .bind(entry.action().as_str())
        .bind(entry.target_user_id().map(|user| user.to_string()))
        .bind(entry.target_email().map(|email| email.to_string()))
        .bind(entry.outcome())
        .bind(entry.details().map(|details| details.to_string()))
        .execute(&mut **tx)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to insert admin audit log entry: {}", err);
        })?;
        Ok(())
    }
}

impl AdminRepository for SqliteUsersRepository {
    #[tracing::instrument(skip_all, err(Debug))]
    async fn create_or_promote_admin(
        &self,
        audit: &AdminAuditContext,
        email: &EmailAddress,
        password_hash: HashedPassword,
        mode: BootstrapAdminMode,
    ) -> Result<BootstrapAdminOutcome, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!("Unable to start transaction to bootstrap admin: {}", err);
        })?;

        let mode_str = match mode {
            BootstrapAdminMode::IfNoAdmin => "if_no_admin",
            BootstrapAdminMode::Force => "force",
        };

        if mode == BootstrapAdminMode::IfNoAdmin {
            let admins_exist: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM t_users WHERE is_admin = TRUE);")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|err| {
                        tracing::warn!("Unable to check for an existing admin: {}", err);
                    })?;
            if admins_exist {
                Self::insert_admin_audit(
                    &mut tx,
                    &audit.entry(
                        AdminAction::BootstrapAdmin,
                        None,
                        Some(email.clone()),
                        "skipped_admin_exists",
                        Some(serde_json::json!({ "mode": mode_str })),
                    ),
                )
                .await?;
                tx.commit().await.map_err(|err| {
                    tracing::warn!("Unable to commit admin bootstrap transaction: {}", err);
                })?;
                return Ok(BootstrapAdminOutcome::SkippedAdminExists);
            }
        }

        let existing: Option<(i64, String, bool)> = sqlx::query_as(
            r#"
        SELECT u.rowid, u.user_id, u.is_admin
        FROM t_users u
        JOIN t_emails e ON e.user = u.rowid
        WHERE e.email = ?1
        ;"#,
        )
        .bind(email.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| {
            tracing::warn!(
                "Unable to look up user by email to bootstrap admin: {}",
                err
            );
        })?;

        let (outcome, target_user_id) = match existing {
            None => {
                let user = UserId::new();
                sqlx::query(
                    "INSERT INTO t_users (user_id, password_hash, is_admin) VALUES (?1, ?2, TRUE);",
                )
                .bind(user.to_string())
                .bind(password_hash.to_string())
                .execute(&mut *tx)
                .await
                .map_err(|err| {
                    tracing::warn!("Unable to insert admin user: {}", err);
                })?;

                sqlx::query(
                    r#"
                INSERT INTO t_emails (email, user, verified) VALUES (
                    ?1, (SELECT rowid FROM t_users WHERE user_id = ?2), TRUE
                );"#,
                )
                .bind(email.to_string())
                .bind(user.to_string())
                .execute(&mut *tx)
                .await
                .map_err(|err| {
                    tracing::warn!("Unable to insert admin user email: {}", err);
                })?;

                (BootstrapAdminOutcome::Created, Some(user))
            }
            Some((rowid, user_id, was_admin)) => {
                sqlx::query(
                    "UPDATE t_users SET is_admin = TRUE, password_hash = ?1 WHERE rowid = ?2;",
                )
                .bind(password_hash.to_string())
                .bind(rowid)
                .execute(&mut *tx)
                .await
                .map_err(|err| {
                    tracing::warn!("Unable to promote user to admin: {}", err);
                })?;

                (
                    if was_admin {
                        BootstrapAdminOutcome::PasswordUpdated
                    } else {
                        BootstrapAdminOutcome::Promoted
                    },
                    Some(UserId::from(user_id)),
                )
            }
        };

        let outcome_str = match outcome {
            BootstrapAdminOutcome::Created => "created",
            BootstrapAdminOutcome::Promoted => "promoted",
            BootstrapAdminOutcome::PasswordUpdated => "password_updated",
            BootstrapAdminOutcome::SkippedAdminExists => "skipped_admin_exists",
        };
        Self::insert_admin_audit(
            &mut tx,
            &audit.entry(
                AdminAction::BootstrapAdmin,
                target_user_id,
                Some(email.clone()),
                outcome_str,
                Some(serde_json::json!({ "mode": mode_str })),
            ),
        )
        .await?;

        tx.commit().await.map_err(|err| {
            tracing::warn!("Unable to commit admin bootstrap transaction: {}", err);
        })?;

        Ok(outcome)
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn promote_admin(
        &self,
        audit: &AdminAuditContext,
        email: &EmailAddress,
    ) -> Result<PromoteAdminOutcome, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!("Unable to start transaction to promote admin: {}", err);
        })?;

        let existing: Option<(i64, String, bool)> = sqlx::query_as(
            r#"
        SELECT u.rowid, u.user_id, u.is_admin
        FROM t_users u
        JOIN t_emails e ON e.user = u.rowid
        WHERE e.email = ?1
        ;"#,
        )
        .bind(email.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to look up user by email to promote admin: {}", err);
        })?;

        let Some((rowid, user_id, is_admin)) = existing else {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::PromoteAdmin,
                    None,
                    Some(email.clone()),
                    "unknown_user",
                    None,
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit admin promote transaction: {}", err);
            })?;
            return Ok(PromoteAdminOutcome::UnknownUser);
        };
        let target_user_id = UserId::from(user_id);
        if is_admin {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::PromoteAdmin,
                    Some(target_user_id),
                    Some(email.clone()),
                    "already_admin",
                    None,
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit admin promote transaction: {}", err);
            })?;
            return Ok(PromoteAdminOutcome::AlreadyAdmin);
        }

        sqlx::query("UPDATE t_users SET is_admin = TRUE WHERE rowid = ?1;")
            .bind(rowid)
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                tracing::warn!("Unable to promote admin: {}", err);
            })?;

        Self::insert_admin_audit(
            &mut tx,
            &audit.entry(
                AdminAction::PromoteAdmin,
                Some(target_user_id),
                Some(email.clone()),
                "promoted",
                None,
            ),
        )
        .await?;

        tx.commit().await.map_err(|err| {
            tracing::warn!("Unable to commit admin promote transaction: {}", err);
        })?;

        Ok(PromoteAdminOutcome::Promoted)
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn revoke_admin(
        &self,
        audit: &AdminAuditContext,
        email: &EmailAddress,
    ) -> Result<RevokeAdminOutcome, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!("Unable to start transaction to revoke admin: {}", err);
        })?;

        let existing: Option<(i64, String, bool)> = sqlx::query_as(
            r#"
        SELECT u.rowid, u.user_id, u.is_admin
        FROM t_users u
        JOIN t_emails e ON e.user = u.rowid
        WHERE e.email = ?1
        ;"#,
        )
        .bind(email.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to look up user by email to revoke admin: {}", err);
        })?;

        let Some((rowid, user_id, is_admin)) = existing else {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::RevokeAdmin,
                    None,
                    Some(email.clone()),
                    "unknown_user",
                    None,
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit admin revoke transaction: {}", err);
            })?;
            return Ok(RevokeAdminOutcome::UnknownUser);
        };
        let target_user_id = UserId::from(user_id);
        if !is_admin {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::RevokeAdmin,
                    Some(target_user_id),
                    Some(email.clone()),
                    "not_admin",
                    None,
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit admin revoke transaction: {}", err);
            })?;
            return Ok(RevokeAdminOutcome::NotAdmin);
        }

        let admin_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM t_users WHERE is_admin = TRUE;")
                .fetch_one(&mut *tx)
                .await
                .map_err(|err| {
                    tracing::warn!("Unable to count admins before revoking: {}", err);
                })?;
        if admin_count <= 1 {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::RevokeAdmin,
                    Some(target_user_id),
                    Some(email.clone()),
                    "last_admin",
                    None,
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit admin revoke transaction: {}", err);
            })?;
            return Ok(RevokeAdminOutcome::LastAdmin);
        }

        sqlx::query("UPDATE t_users SET is_admin = FALSE WHERE rowid = ?1;")
            .bind(rowid)
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                tracing::warn!("Unable to revoke admin: {}", err);
            })?;

        Self::insert_admin_audit(
            &mut tx,
            &audit.entry(
                AdminAction::RevokeAdmin,
                Some(target_user_id),
                Some(email.clone()),
                "revoked",
                None,
            ),
        )
        .await?;

        tx.commit().await.map_err(|err| {
            tracing::warn!("Unable to commit admin revoke transaction: {}", err);
        })?;

        Ok(RevokeAdminOutcome::Revoked)
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn is_admin(&self, user: &UserId) -> Result<bool, ()> {
        let res: Result<Option<bool>, sqlx::Error> =
            sqlx::query_scalar("SELECT is_admin FROM t_users WHERE user_id = ?1;")
                .bind(user.to_string())
                .fetch_optional(&self.readers)
                .await;
        match res {
            Ok(Some(is_admin)) => Ok(is_admin),
            Ok(None) => Ok(false),
            Err(err) => {
                tracing::warn!("Cannot check admin status from database: {}", err);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn list_users(&self) -> Result<Vec<AdminUser>, ()> {
        type UserRow = (String, Option<String>, bool, bool, bool);
        let rows: Result<Vec<UserRow>, sqlx::Error> = sqlx::query_as(
            r#"
        SELECT u.user_id, e.email, COALESCE(e.verified, false), u.password_hash IS NOT NULL, u.is_admin
        FROM t_users u
        LEFT JOIN t_emails e ON e.user = u.rowid
        ORDER BY e.email
        ;"#,
        )
        .fetch_all(&self.readers)
        .await;

        let rows = match rows {
            Ok(rows) => rows,
            Err(err) => {
                tracing::warn!("Cannot list users from database: {}", err);
                return Err(());
            }
        };

        let mut users = Vec::with_capacity(rows.len());
        for (user_id, email, verified, has_password, is_admin) in rows {
            let email_valid = email
                .as_ref()
                .map(|e| EmailAddress::try_from(e.as_str()).is_ok())
                .unwrap_or_default();
            if !email_valid {
                tracing::warn!(%user_id, "Stored user email is not a valid email address");
            }
            users.push(AdminUser::new(
                UserId::from(user_id),
                email,
                email_valid,
                verified,
                has_password,
                is_admin,
            ));
        }

        Ok(users)
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn create_user(
        &self,
        audit: &AdminAuditContext,
        email: &EmailAddress,
        password_hash: HashedPassword,
    ) -> Result<CreateUserOutcome, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!("Unable to start transaction to create user: {}", err);
        })?;

        let email_in_use: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM t_emails WHERE email = ?1);")
                .bind(email.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(|err| {
                    tracing::warn!("Unable to check email availability: {}", err);
                })?;
        if email_in_use {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::CreateUser,
                    None,
                    Some(email.clone()),
                    "email_already_used",
                    None,
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit create user transaction: {}", err);
            })?;
            return Ok(CreateUserOutcome::EmailAlreadyUsed);
        }

        let user = UserId::new();
        sqlx::query("INSERT INTO t_users (user_id, password_hash) VALUES (?1, ?2);")
            .bind(user.to_string())
            .bind(password_hash.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                tracing::warn!("Unable to insert user: {}", err);
            })?;

        sqlx::query(
            r#"
        INSERT INTO t_emails (email, user) VALUES (
            ?1, (SELECT rowid FROM t_users WHERE user_id = ?2)
        );"#,
        )
        .bind(email.to_string())
        .bind(user.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to insert user email: {}", err);
        })?;

        Self::insert_admin_audit(
            &mut tx,
            &audit.entry(
                AdminAction::CreateUser,
                Some(user.clone()),
                Some(email.clone()),
                "created",
                None,
            ),
        )
        .await?;

        tx.commit().await.map_err(|err| {
            tracing::warn!("Unable to commit create user transaction: {}", err);
        })?;

        Ok(CreateUserOutcome::Created(user))
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn set_user_password(
        &self,
        audit: &AdminAuditContext,
        user: &UserId,
        password_hash: HashedPassword,
    ) -> Result<SetUserPasswordOutcome, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!("Unable to start transaction to set user password: {}", err);
        })?;

        let res = sqlx::query("UPDATE t_users SET password_hash = ?1 WHERE user_id = ?2;")
            .bind(password_hash.to_string())
            .bind(user.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                tracing::warn!("Unable to set user password: {}", err);
            })?;

        let outcome = if res.rows_affected() == 0 {
            SetUserPasswordOutcome::UnknownUser
        } else {
            SetUserPasswordOutcome::Updated
        };
        let outcome_str = match outcome {
            SetUserPasswordOutcome::Updated => "updated",
            SetUserPasswordOutcome::UnknownUser => "unknown_user",
        };

        Self::insert_admin_audit(
            &mut tx,
            &audit.entry(
                AdminAction::SetUserPassword,
                Some(user.clone()),
                None,
                outcome_str,
                None,
            ),
        )
        .await?;

        tx.commit().await.map_err(|err| {
            tracing::warn!("Unable to commit set user password transaction: {}", err);
        })?;

        Ok(outcome)
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn set_user_email(
        &self,
        audit: &AdminAuditContext,
        user: &UserId,
        email: EmailAddress,
    ) -> Result<SetUserEmailOutcome, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!("Unable to start transaction to update user email: {}", err);
        })?;

        let user_row: Option<i64> =
            sqlx::query_scalar("SELECT rowid FROM t_users WHERE user_id = ?1;")
                .bind(user.to_string())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|err| {
                    tracing::warn!("Unable to look up user to set email: {}", err);
                })?;

        let Some(rowid) = user_row else {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::SetUserEmail,
                    Some(user.clone()),
                    Some(email.clone()),
                    "unknown_user",
                    None,
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit update user email transaction: {}", err);
            })?;
            return Ok(SetUserEmailOutcome::UnknownUser);
        };

        let email_in_use_by_other: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM t_emails WHERE email = ?1 AND user != ?2);",
        )
        .bind(email.to_string())
        .bind(rowid)
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to check email availability: {}", err);
        })?;

        if email_in_use_by_other {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::SetUserEmail,
                    Some(user.clone()),
                    Some(email.clone()),
                    "email_address_taken",
                    None,
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit update user email transaction: {}", err);
            })?;
            return Ok(SetUserEmailOutcome::EmailAddressTaken);
        }

        let updated = sqlx::query("UPDATE t_emails SET email = ?1 WHERE user = ?2;")
            .bind(email.value())
            .bind(rowid)
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                tracing::warn!("Unable to update user email: {}", err);
            })?;

        if updated.rows_affected() == 0 {
            sqlx::query("INSERT INTO t_emails (user, email) VALUES (?1, ?2);")
                .bind(rowid)
                .bind(email.value())
                .execute(&mut *tx)
                .await
                .map_err(|err| {
                    tracing::warn!("Unable to insert user email: {}", err);
                })?;
        }

        Self::insert_admin_audit(
            &mut tx,
            &audit.entry(
                AdminAction::SetUserEmail,
                Some(user.clone()),
                Some(email),
                "updated",
                None,
            ),
        )
        .await?;

        tx.commit().await.map_err(|err| {
            tracing::warn!("Unable to commit update user email transaction: {}", err);
        })?;

        Ok(SetUserEmailOutcome::Updated)
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn set_admin(
        &self,
        audit: &AdminAuditContext,
        user: &UserId,
        is_admin: bool,
    ) -> Result<SetAdminOutcome, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!("Unable to start transaction to set admin flag: {}", err);
        })?;

        let current: Option<bool> =
            sqlx::query_scalar("SELECT is_admin FROM t_users WHERE user_id = ?1;")
                .bind(user.to_string())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|err| {
                    tracing::warn!("Unable to look up user to set admin flag: {}", err);
                })?;

        let Some(current) = current else {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::SetAdmin,
                    Some(user.clone()),
                    None,
                    "unknown_user",
                    Some(serde_json::json!({ "is_admin": is_admin })),
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit set admin transaction: {}", err);
            })?;
            return Ok(SetAdminOutcome::UnknownUser);
        };
        if current == is_admin {
            Self::insert_admin_audit(
                &mut tx,
                &audit.entry(
                    AdminAction::SetAdmin,
                    Some(user.clone()),
                    None,
                    "updated",
                    Some(serde_json::json!({ "is_admin": is_admin })),
                ),
            )
            .await?;
            tx.commit().await.map_err(|err| {
                tracing::warn!("Unable to commit set admin transaction: {}", err);
            })?;
            return Ok(SetAdminOutcome::Updated);
        }

        if !is_admin {
            let admin_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM t_users WHERE is_admin = TRUE;")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|err| {
                        tracing::warn!("Unable to count admins before revoking: {}", err);
                    })?;
            if admin_count <= 1 {
                Self::insert_admin_audit(
                    &mut tx,
                    &audit.entry(
                        AdminAction::SetAdmin,
                        Some(user.clone()),
                        None,
                        "last_admin",
                        Some(serde_json::json!({ "is_admin": is_admin })),
                    ),
                )
                .await?;
                tx.commit().await.map_err(|err| {
                    tracing::warn!("Unable to commit set admin transaction: {}", err);
                })?;
                return Ok(SetAdminOutcome::LastAdmin);
            }
        }

        sqlx::query("UPDATE t_users SET is_admin = ?1 WHERE user_id = ?2;")
            .bind(is_admin)
            .bind(user.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                tracing::warn!("Unable to set admin flag: {}", err);
            })?;

        Self::insert_admin_audit(
            &mut tx,
            &audit.entry(
                AdminAction::SetAdmin,
                Some(user.clone()),
                None,
                "updated",
                Some(serde_json::json!({ "is_admin": is_admin })),
            ),
        )
        .await?;

        tx.commit().await.map_err(|err| {
            tracing::warn!("Unable to commit set admin transaction: {}", err);
        })?;

        Ok(SetAdminOutcome::Updated)
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn revoke_user_sessions(
        &self,
        audit: &AdminAuditContext,
        user: &UserId,
    ) -> Result<u64, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!(
                "Unable to start transaction to revoke user sessions: {}",
                err
            );
        })?;

        let revoked = sqlx::query(
            r#"
        DELETE FROM t_sessions
        WHERE user = (SELECT rowid FROM t_users WHERE user_id = ?1)
        ;"#,
        )
        .bind(user.to_string())
        .execute(&mut *tx)
        .await
        .map(|res| res.rows_affected())
        .map_err(|err| {
            tracing::warn!("Unable to revoke user sessions: {}", err);
        })?;

        Self::insert_admin_audit(
            &mut tx,
            &audit.entry(
                AdminAction::RevokeUserSessions,
                Some(user.clone()),
                None,
                "revoked",
                Some(serde_json::json!({ "revoked": revoked })),
            ),
        )
        .await?;

        tx.commit().await.map_err(|err| {
            tracing::warn!("Unable to commit revoke user sessions transaction: {}", err);
        })?;

        Ok(revoked)
    }
}

#[cfg(test)]
impl SqliteUsersRepository {
    /// Test-only accessor for the number of admins, kept out of the production port.
    pub(crate) async fn test_count_admins(&self) -> u64 {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM t_users WHERE is_admin = TRUE;")
            .fetch_one(&self.readers)
            .await
            .unwrap();
        count.max(0) as u64
    }

    /// Test-only accessor for whether the user with the given email is an admin.
    pub(crate) async fn test_is_admin(&self, email: &EmailAddress) -> bool {
        sqlx::query_scalar(
            r#"
        SELECT u.is_admin
        FROM t_users u
        JOIN t_emails e ON e.user = u.rowid
        WHERE e.email = ?1
        ;"#,
        )
        .bind(email.to_string())
        .fetch_optional(&self.readers)
        .await
        .unwrap()
        .unwrap_or(false)
    }
}

impl UserRepository for SqliteUsersRepository {
    #[tracing::instrument(skip_all, err(Debug))]
    async fn store_user(
        &self,
        user: &UserId,
        email: &EmailAddress,
        password_hash: Option<HashedPassword>,
    ) -> Result<(), ()> {
        let mut tx = match self.writer.begin().await {
            Ok(tx) => tx,
            Err(err) => {
                tracing::warn!("Unable to start transaction to store user");
                tracing::warn!("{}", err);
                return Err(());
            }
        };

        if let Err(err) =
            sqlx::query("INSERT INTO t_users (user_id, password_hash) VALUES (?1, ?2);")
                .bind(user.to_string())
                .bind(password_hash.map(|hash| hash.to_string()))
                .execute(&mut *tx)
                .await
        {
            tracing::warn!("Unable to insert user into database");
            tracing::warn!("{}", err);
            return Err(());
        }

        if let Err(err) = sqlx::query(
            r#"
        INSERT INTO t_emails (email, user) VALUES (
            ?1, (SELECT rowid FROM t_users WHERE user_id = ?2)
        );"#,
        )
        .bind(email.to_string())
        .bind(user.to_string())
        .execute(&mut *tx)
        .await
        {
            tracing::warn!("Unable to insert user email into database");
            tracing::warn!("{}", err);
            return Err(());
        }

        match tx.commit().await {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::warn!("Unable to commit user storing transaction");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn get_user_by_email(
        &self,
        email: &EmailAddress,
    ) -> Result<Option<(UserId, Option<HashedPassword>)>, ()> {
        let res: Result<(String, Option<String>), sqlx::Error> = sqlx::query_as(
            r#"
        SELECT u.user_id, u.password_hash
        FROM t_users u
        JOIN t_emails e ON e.user = u.rowid
        WHERE e.email = ?1
        ;"#,
        )
        .bind(email.to_string())
        .fetch_one(&self.readers)
        .await;
        match res {
            Ok((user_id, password_hash)) => Ok(Some((
                UserId::from(user_id),
                password_hash.map(HashedPassword::new),
            ))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => {
                tracing::warn!("Cannot fetch users from database");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn get_user_password_hash(&self, user: &UserId) -> Result<Option<HashedPassword>, ()> {
        // Double Option to avoid parsing NULL into Some("")
        let res: Result<Option<Option<String>>, sqlx::Error> =
            sqlx::query_scalar("SELECT password_hash FROM t_users WHERE user_id = ?1;")
                .bind(user.to_string())
                .fetch_optional(&self.readers)
                .await;
        match res {
            Ok(maybe_hash) => Ok(maybe_hash.flatten().map(HashedPassword::new)),
            Err(err) => {
                tracing::warn!("Cannot fetch user password from database");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn count_users_without_password(&self) -> Result<u64, ()> {
        let res: Result<i64, sqlx::Error> =
            sqlx::query_scalar("SELECT COUNT(*) FROM t_users WHERE password_hash IS NULL;")
                .fetch_one(&self.readers)
                .await;
        match res {
            Ok(count) => Ok(count.max(0) as u64),
            Err(err) => {
                tracing::warn!("Cannot count users without a password in database");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }

    /// Compare-and-set to avoid concurrent operations overwriting each other.
    #[tracing::instrument(skip_all, err(Debug))]
    async fn set_user_password_if_absent(
        &self,
        user: &UserId,
        password_hash: &HashedPassword,
    ) -> Result<bool, ()> {
        let res = sqlx::query(
            r#"
        UPDATE t_users SET password_hash = ?2
        WHERE user_id = ?1 AND password_hash IS NULL
        ;"#,
        )
        .bind(user.to_string())
        .bind(password_hash.to_string())
        .execute(&self.writer)
        .await;
        match res {
            Ok(res) => Ok(res.rows_affected() > 0),
            Err(err) => {
                tracing::warn!("Unable to store user password in database");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }

    /// Compare-and-set to avoid concurrent operations overwriting each other.
    #[tracing::instrument(skip_all, err(Debug))]
    async fn replace_user_password_if_matches(
        &self,
        user: &UserId,
        current_hash: &HashedPassword,
        new_hash: &HashedPassword,
    ) -> Result<bool, ()> {
        let res = sqlx::query(
            r#"
        UPDATE t_users SET password_hash = ?3
        WHERE user_id = ?1 AND password_hash = ?2
        ;"#,
        )
        .bind(user.to_string())
        .bind(current_hash.to_string())
        .bind(new_hash.to_string())
        .execute(&self.writer)
        .await;
        match res {
            Ok(res) => Ok(res.rows_affected() > 0),
            Err(err) => {
                tracing::warn!("Unable to update user password in database");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn store_auth_link(&self, link: &HashedAuthLink) -> Result<(), anyhow::Error> {
        let res = sqlx::query(
            r#"
           INSERT INTO t_auth_links (user, email, token_hash, expire_at)
           SELECT rowid, ?2, ?3, ?4 FROM t_users WHERE user_id = ?1
           ;"#,
        )
        .bind(link.user().to_string())
        .bind(link.email().map(|email| email.to_string()))
        .bind(link.hash().to_string())
        .bind(link.expire_at())
        .execute(&self.writer)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to insert auth link into database");
            tracing::warn!("{}", err);
            anyhow!(err)
        })?;

        if res.rows_affected() == 0 {
            tracing::warn!("Unable to insert auth link into database: unknown user");
            return Err(anyhow!(
                "Unable to insert auth link into database: unknown user"
            ));
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn get_all_auth_links(&self) -> Vec<HashedAuthLink> {
        let res: Vec<(String, Option<String>, String, DateTime<Utc>)> = match sqlx::query_as(
            r#"
           SELECT u.user_id, a.email, a.token_hash, a.expire_at
           FROM t_auth_links a
           JOIN t_users u ON u.rowid = a.user
           ;"#,
        )
        .fetch_all(&self.readers)
        .await
        {
            Ok(res) => res,
            Err(err) => {
                tracing::warn!("Cannot fetch auth links from database");
                tracing::warn!("{}", err);
                return Vec::new();
            }
        };
        res.iter()
            .map(|(user, email, token, expire_at)| {
                let email = match email {
                    None => None,
                    Some(email) => match EmailAddress::try_from(email.clone()) {
                        Ok(email) => Some(email),
                        Err(err) => {
                            tracing::warn!(
                                "Stored auth link has an invalid target email ({}): it will not \
                                 verify any address",
                                err
                            );
                            None
                        }
                    },
                };
                HashedAuthLink::new(
                    UserId::from(user.clone()),
                    email,
                    HashedAuthToken::new(token.clone()),
                    *expire_at,
                )
            })
            .collect()
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_auth_link_by_hash(&self, hash: &HashedAuthToken) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM t_auth_links WHERE token_hash = ?1;")
            .bind(hash.to_string())
            .execute(&self.writer)
            .await
            .map(|_| ())
            .map_err(|err| anyhow!(err))
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_expired_auth_links(
        &self,
        reference: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM t_auth_links WHERE expire_at <= ?1;")
            .bind(reference)
            .execute(&self.writer)
            .await
            .map(|_| ())
            .map_err(|err| anyhow!(err))
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn get_user_email(&self, user: &UserId) -> Result<Option<EmailAddress>, ()> {
        let res: Result<Option<String>, sqlx::Error> = sqlx::query_scalar(
            r#"
        SELECT e.email
        FROM t_emails e
        JOIN t_users u ON u.rowid = e.user
        WHERE u.user_id = ?1
        ;"#,
        )
        .bind(user.to_string())
        .fetch_optional(&self.readers)
        .await;
        match res {
            Ok(email) => Ok(match email {
                Some(email) => match EmailAddress::try_from(email) {
                    Ok(email) => Some(email),
                    Err(err) => {
                        tracing::warn!("Stored user email is not a valid email address: {}", err);
                        return Err(());
                    }
                },
                None => None,
            }),
            Err(err) => {
                tracing::warn!("Cannot fetch user email from database: {}", err);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn update_user_email(
        &self,
        user: &UserId,
        new_email: &EmailAddress,
        verified: bool,
    ) -> Result<(), ()> {
        let res = sqlx::query(
            r#"
        UPDATE t_emails
        SET email = ?2, verified = ?3
        WHERE user = (SELECT rowid FROM t_users WHERE user_id = ?1)
        ;"#,
        )
        .bind(user.to_string())
        .bind(new_email.to_string())
        .bind(verified)
        .execute(&self.writer)
        .await;
        match res {
            Ok(res) if res.rows_affected() == 1 => Ok(()),
            Ok(res) => {
                tracing::warn!(
                    "Unable to update user email in database: not exactly one row affected ({})",
                    res.rows_affected()
                );
                Err(())
            }
            Err(err) => {
                tracing::warn!("Unable to update user email in database: {}", err);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn consume_auth_link(&self, user: &UserId, hash: &HashedAuthToken) -> Result<bool, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!("Unable to start transaction to consume auth link: {}", err);
        })?;

        let deleted: Option<Option<String>> = sqlx::query_scalar(
            r#"
        DELETE FROM t_auth_links
        WHERE
            token_hash = ?1
            AND user = (SELECT rowid FROM t_users WHERE user_id = ?2)
        RETURNING email
        ;"#,
        )
        .bind(hash.to_string())
        .bind(user.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to consume auth link in database: {}", err);
        })?;

        let Some(email) = deleted else {
            // Already consumed concurrently (or unknown): nothing to verify.
            tx.rollback().await.map_err(|err| {
                tracing::warn!("Unable to roll back auth link consumption: {}", err);
            })?;
            return Ok(false);
        };

        match email {
            None => {}
            Some(email) => match EmailAddress::try_from(email) {
                Ok(email) => {
                    sqlx::query(
                        r#"
                    UPDATE t_emails SET verified = TRUE
                    WHERE
                        user = (SELECT rowid FROM t_users WHERE user_id = ?1)
                        AND email = ?2
                    ;"#,
                    )
                    .bind(user.to_string())
                    .bind(email.to_string())
                    .execute(&mut *tx)
                    .await
                    .map_err(|err| {
                        tracing::warn!("Unable to mark auth link email verified: {}", err);
                    })?;
                }
                Err(err) => {
                    tracing::warn!(
                        "Consumed auth link carries an invalid target email ({}): it verifies no \
                         address",
                        err
                    );
                }
            },
        }

        tx.commit().await.map_err(|err| {
            tracing::warn!(
                "Unable to commit auth link consumption transaction: {}",
                err
            );
        })?;

        Ok(true)
    }

    #[tracing::instrument(skip_all, err)]
    async fn store_email_change_token(
        &self,
        token: &HashedEmailChangeToken,
    ) -> Result<(), anyhow::Error> {
        let res = sqlx::query(
            r#"
           INSERT INTO t_email_change_tokens (user, new_email, token_hash, expire_at)
           SELECT rowid, ?2, ?3, ?4 FROM t_users WHERE user_id = ?1
           ;"#,
        )
        .bind(token.user().to_string())
        .bind(token.new_email().to_string())
        .bind(token.hash().to_string())
        .bind(token.expire_at())
        .execute(&self.writer)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to insert email change token into database: {}", err);
            anyhow!(err)
        })?;

        if res.rows_affected() != 1 {
            tracing::warn!("Unable to insert email change token into database: unknown user");
            return Err(anyhow!(
                "Unable to insert email change token into database: unknown user"
            ));
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn get_all_email_change_tokens(&self) -> Vec<HashedEmailChangeToken> {
        let res: Vec<(String, String, String, DateTime<Utc>)> = match sqlx::query_as(
            r#"
           SELECT u.user_id, t.new_email, t.token_hash, t.expire_at
           FROM t_email_change_tokens t
           JOIN t_users u ON u.rowid = t.user
           ;"#,
        )
        .fetch_all(&self.readers)
        .await
        {
            Ok(res) => res,
            Err(err) => {
                tracing::warn!("Cannot fetch email change tokens from database: {}", err);
                return Vec::new();
            }
        };
        res.iter()
            .map(|(user, new_email, token, expire_at)| {
                let Ok(new_email) = EmailAddress::try_from(new_email.clone()) else {
                    tracing::warn!(
                        "Stored email change token has an invalid target email: {}",
                        new_email
                    );
                    return None;
                };
                Some(HashedEmailChangeToken::new(
                    UserId::from(user.clone()),
                    new_email,
                    HashedAuthToken::new(token.clone()),
                    *expire_at,
                ))
            })
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default()
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_email_change_token_by_hash(
        &self,
        hash: &HashedAuthToken,
    ) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM t_email_change_tokens WHERE token_hash = ?1;")
            .bind(hash.to_string())
            .execute(&self.writer)
            .await
            .map(|_| ())
            .map_err(|err| anyhow!(err))
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_expired_email_change_tokens(
        &self,
        reference: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM t_email_change_tokens WHERE expire_at <= ?1;")
            .bind(reference)
            .execute(&self.writer)
            .await
            .map(|_| ())
            .map_err(|err| anyhow!(err))
    }

    /// Consumes the token and swaps user's email in a single transaction, so the token can only be
    /// used once even under concurrent validation attempts.
    /// Domain logic voluntary leaks into the infra to ensure atomic compare-and-swap semantic.
    #[tracing::instrument(skip_all, err(Debug))]
    async fn finalize_email_change(
        &self,
        user: &UserId,
        hash: &HashedAuthToken,
    ) -> Result<EmailChangeFinalizeResult, ()> {
        let mut tx = self.writer.begin().await.map_err(|err| {
            tracing::warn!("Unable to start transaction to finalize email change");
            tracing::warn!("{}", err);
        })?;

        let Some(new_email) = Self::delete_email_change_token(&mut tx, user, hash).await? else {
            return Ok(EmailChangeFinalizeResult::UnknownToken);
        };

        let is_unique = Self::is_email_unique(&mut tx, user, new_email.value())
            .await
            .map_err(|err| {
                tracing::warn!("Unable to check email availability in database");
                tracing::warn!("{}", err);
            })?;

        if !is_unique {
            // Token stays consumed: retrying with the same link is pointless.
            tx.commit().await.map(|_| ()).map_err(|err| {
                tracing::warn!(
                    "Unable to commit email change finalization transaction: {}",
                    err
                );
            })?;
            return Ok(EmailChangeFinalizeResult::EmailAlreadyInUse);
        }

        Self::set_user_email_verified(&mut tx, user, new_email.value())
            .await
            .map_err(|err| {
                tracing::warn!("Unable to update user email in database: {}", err);
            })?;

        tx.commit().await.map(|_| ()).map_err(|err| {
            tracing::warn!(
                "Unable to commit email change finalization transaction: {}",
                err
            );
        })?;

        Ok(EmailChangeFinalizeResult::Swapped(new_email))
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn get_user_info(&self, user: &UserId) -> Result<Option<UserInfo>, ()> {
        type UserInfoRow = (Option<String>, bool, bool, bool);
        // LEFT JOIN: users registered without email-based auth also have a t_users row, but the
        // result is only meaningful when the user has an email.
        let res: Result<Option<UserInfoRow>, sqlx::Error> = sqlx::query_as(
            r#"
        SELECT e.email, e.verified, u.password_hash IS NOT NULL, u.is_admin
        FROM t_users u
        LEFT JOIN t_emails e ON e.user = u.rowid
        WHERE u.user_id = ?1
        ;"#,
        )
        .bind(user.to_string())
        .fetch_optional(&self.readers)
        .await;
        match res {
            Ok(Some((Some(email), verified, has_password, is_admin))) => {
                let email_valid = EmailAddress::try_from(email.as_str()).is_ok();
                if !email_valid {
                    tracing::warn!("Stored user email is not a valid email address: {}", &email);
                };
                Ok(Some(UserInfo::new(
                    email,
                    email_valid,
                    verified,
                    has_password,
                    is_admin,
                )))
            }
            // User unknown, or user without an email row.
            Ok(_) => Ok(None),
            Err(err) => {
                tracing::warn!("Cannot fetch user info from database");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }
}

impl SessionRepository for SqliteUsersRepository {
    #[tracing::instrument(skip_all, err(Debug))]
    async fn store_session(&self, session: &HashedSession) -> Result<(), ()> {
        let res = sqlx::query(
            r#"
           INSERT INTO t_sessions (user, token_hash, expire_at)
           SELECT rowid, ?2, ?3 FROM t_users WHERE user_id = ?1
           ;"#,
        )
        .bind(session.user().to_string())
        .bind(session.hash().to_string())
        .bind(session.expire_at())
        .execute(&self.writer)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to insert session into database");
            tracing::warn!("{}", err);
        })?;

        if res.rows_affected() == 0 {
            tracing::warn!("Unable to insert session into database: unknown user");
            return Err(());
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn get_session_by_hash(&self, hash: &HashedSessionToken) -> Option<HashedSession> {
        let res: Option<(String, String, DateTime<Utc>)> = match sqlx::query_as(
            r#"
           SELECT u.user_id, s.token_hash, s.expire_at
           FROM t_sessions s
           JOIN t_users u ON u.rowid = s.user
           WHERE s.token_hash = ?1
           ;"#,
        )
        .bind(hash.to_string())
        .fetch_optional(&self.readers)
        .await
        {
            Ok(res) => res,
            Err(err) => {
                tracing::warn!("Cannot fetch session from database");
                tracing::warn!("{}", err);
                return None;
            }
        };
        res.map(|(user, token, expire_at)| {
            HashedSession::new(
                UserId::from(user),
                HashedSessionToken::new(token),
                expire_at,
            )
        })
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn delete_session_by_hash(&self, hash: &HashedSessionToken) -> Result<(), ()> {
        sqlx::query("DELETE FROM t_sessions WHERE token_hash = ?1;")
            .bind(hash.to_string())
            .execute(&self.writer)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn delete_expired_sessions(&self, reference: DateTime<Utc>) -> Result<(), ()> {
        sqlx::query("DELETE FROM t_sessions WHERE expire_at <= ?1;")
            .bind(reference)
            .execute(&self.writer)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn delete_sessions_for_user(&self, user: &UserId) -> Result<(), ()> {
        sqlx::query(
            r#"
        DELETE FROM t_sessions
        WHERE user = (SELECT rowid FROM t_users WHERE user_id = ?1)
        ;"#,
        )
        .bind(user.to_string())
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|_| ())
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn delete_sessions_for_user_except(
        &self,
        user: &UserId,
        hash: &HashedSessionToken,
    ) -> Result<(), ()> {
        sqlx::query(
            r#"
        DELETE FROM t_sessions
        WHERE
            user = (SELECT rowid FROM t_users WHERE user_id = ?1)
            AND token_hash != ?2
        ;"#,
        )
        .bind(user.to_string())
        .bind(hash.to_string())
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|_| ())
    }
}

#[cfg(test)]
mod test_sqlite_users_repository {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    use chrono::{TimeDelta, Utc};
    use tempfile::NamedTempFile;

    use crate::{
        domain::models::UserId,
        inbound::auth::multi_user::{
            AuthLink, AuthToken, EmailAddress, EmailChangeFinalizeResult, HashedEmailChangeToken,
            HashedSessionToken, Password, Session, SessionToken,
            admin::{
                AdminActor, AdminAuditContext, AdminRepository, BootstrapAdminMode,
                BootstrapAdminOutcome, CreateUserOutcome, PromoteAdminOutcome, RevokeAdminOutcome,
                SetAdminOutcome, SetUserEmailOutcome, SetUserPasswordOutcome,
            },
            service::UserRepository,
            session::SessionRepository,
        },
    };

    use super::*;

    fn test_email() -> EmailAddress {
        EmailAddress::try_from("test@mail.test").unwrap()
    }

    /// Creates a repository and registers a single test user in it.
    async fn repository_with_test_user() -> (NamedTempFile, SqliteUsersRepository) {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        repository
            .store_user(&UserId::test_default(), &test_email(), None)
            .await
            .expect("should have stored user with email");

        (db_file, repository)
    }

    fn password_hash(password: &str) -> HashedPassword {
        Password::try_from(password.to_string())
            .unwrap()
            .as_hash()
            .unwrap()
    }

    /// An audit context usable by the repository tests that call mutating admin methods.
    fn web_audit() -> AdminAuditContext {
        AdminAuditContext::new(AdminActor::Web(UserId::test_default()), Utc::now())
    }

    async fn is_admin(repository: &SqliteUsersRepository, email: &EmailAddress) -> bool {
        sqlx::query_scalar(
            r#"
        SELECT u.is_admin
        FROM t_users u
        JOIN t_emails e ON e.user = u.rowid
        WHERE e.email = ?1
        ;"#,
        )
        .bind(email.to_string())
        .fetch_one(&repository.writer)
        .await
        .unwrap()
    }

    async fn user_count(repository: &SqliteUsersRepository) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM t_users;")
            .fetch_one(&repository.writer)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_bootstrap_admin_creates_verified_admin_with_password() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .unwrap();

        let outcome = repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("s3cret"),
                BootstrapAdminMode::IfNoAdmin,
            )
            .await
            .unwrap();

        assert_eq!(outcome, BootstrapAdminOutcome::Created);
        assert!(is_admin(&repository, &test_email()).await);

        let verified: bool = sqlx::query_scalar("SELECT verified FROM t_emails WHERE email = ?1;")
            .bind(test_email().to_string())
            .fetch_one(&repository.writer)
            .await
            .unwrap();
        assert!(verified);

        let stored: Option<String> = sqlx::query_scalar("SELECT password_hash FROM t_users;")
            .fetch_one(&repository.writer)
            .await
            .unwrap();
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_bootstrap_admin_if_no_admin_skips_when_admin_exists() {
        let (_db, repository) = repository_with_test_user().await;
        repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("s3cret"),
                BootstrapAdminMode::IfNoAdmin,
            )
            .await
            .unwrap();

        let other = EmailAddress::try_from("other@mail.test").unwrap();
        let outcome = repository
            .create_or_promote_admin(
                &web_audit(),
                &other,
                password_hash("other"),
                BootstrapAdminMode::IfNoAdmin,
            )
            .await
            .unwrap();

        assert_eq!(outcome, BootstrapAdminOutcome::SkippedAdminExists);
        assert_eq!(user_count(&repository).await, 1);
    }

    #[tokio::test]
    async fn test_bootstrap_admin_if_no_admin_is_idempotent() {
        let (_db, repository) = repository_with_test_user().await;
        repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("s3cret"),
                BootstrapAdminMode::IfNoAdmin,
            )
            .await
            .unwrap();

        let outcome = repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("s3cret"),
                BootstrapAdminMode::IfNoAdmin,
            )
            .await
            .unwrap();

        assert_eq!(outcome, BootstrapAdminOutcome::SkippedAdminExists);
        assert_eq!(user_count(&repository).await, 1);
    }

    #[tokio::test]
    async fn test_bootstrap_admin_force_promotes_existing_user() {
        let (_db, repository) = repository_with_test_user().await;

        let outcome = repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("s3cret"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();

        assert_eq!(outcome, BootstrapAdminOutcome::Promoted);
        assert!(is_admin(&repository, &test_email()).await);
        assert_eq!(user_count(&repository).await, 1);
    }

    #[tokio::test]
    async fn test_bootstrap_admin_force_updates_existing_admin_password() {
        let (_db, repository) = repository_with_test_user().await;
        repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("first"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();

        let outcome = repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("second"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();

        assert_eq!(outcome, BootstrapAdminOutcome::PasswordUpdated);
        let stored: String =
            sqlx::query_scalar("SELECT password_hash FROM t_users WHERE user_id = ?1;")
                .bind(UserId::test_default().to_string())
                .fetch_one(&repository.writer)
                .await
                .unwrap();
        assert!(
            HashedPassword::new(stored)
                .verify_password(&Password::try_from("second".to_string()).unwrap())
        );
    }

    #[tokio::test]
    async fn test_promote_admin_sets_flag() {
        let (_db, repository) = repository_with_test_user().await;

        let outcome = repository
            .promote_admin(&web_audit(), &test_email())
            .await
            .unwrap();

        assert_eq!(outcome, PromoteAdminOutcome::Promoted);
        assert!(is_admin(&repository, &test_email()).await);
        assert_eq!(user_count(&repository).await, 1);
    }

    #[tokio::test]
    async fn test_promote_admin_already_admin() {
        let (_db, repository) = repository_with_test_user().await;
        repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("pw"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();

        let outcome = repository
            .promote_admin(&web_audit(), &test_email())
            .await
            .unwrap();

        assert_eq!(outcome, PromoteAdminOutcome::AlreadyAdmin);
        assert!(is_admin(&repository, &test_email()).await);
    }

    #[tokio::test]
    async fn test_promote_admin_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .unwrap();

        let outcome = repository
            .promote_admin(&web_audit(), &test_email())
            .await
            .unwrap();

        assert_eq!(outcome, PromoteAdminOutcome::UnknownUser);
    }

    #[tokio::test]
    async fn test_revoke_admin_clears_flag_when_another_admin_exists() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .unwrap();
        let first = EmailAddress::try_from("first@mail.test").unwrap();
        let second = EmailAddress::try_from("second@mail.test").unwrap();
        repository
            .create_or_promote_admin(
                &web_audit(),
                &first,
                password_hash("pw"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();
        repository
            .create_or_promote_admin(
                &web_audit(),
                &second,
                password_hash("pw"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();

        let outcome = repository.revoke_admin(&web_audit(), &first).await.unwrap();

        assert_eq!(outcome, RevokeAdminOutcome::Revoked);
        assert!(!is_admin(&repository, &first).await);
        assert!(is_admin(&repository, &second).await);
    }

    #[tokio::test]
    async fn test_revoke_admin_refuses_last_admin() {
        let (_db, repository) = repository_with_test_user().await;
        repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("pw"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();

        let outcome = repository
            .revoke_admin(&web_audit(), &test_email())
            .await
            .unwrap();

        assert_eq!(outcome, RevokeAdminOutcome::LastAdmin);
        assert!(is_admin(&repository, &test_email()).await);
    }

    #[tokio::test]
    async fn test_revoke_admin_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .unwrap();

        let outcome = repository
            .revoke_admin(&web_audit(), &test_email())
            .await
            .unwrap();

        assert_eq!(outcome, RevokeAdminOutcome::UnknownUser);
    }

    #[tokio::test]
    async fn test_revoke_admin_not_admin() {
        let (_db, repository) = repository_with_test_user().await;

        let outcome = repository
            .revoke_admin(&web_audit(), &test_email())
            .await
            .unwrap();

        assert_eq!(outcome, RevokeAdminOutcome::NotAdmin);
    }

    #[tokio::test]
    async fn test_is_admin_returns_false_for_unknown_and_non_admin_users() {
        let (_db, repository) = repository_with_test_user().await;

        assert!(!repository.is_admin(&UserId::from("unknown")).await.unwrap());
        assert!(!repository.is_admin(&UserId::test_default()).await.unwrap());

        repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("pw"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();

        assert!(repository.is_admin(&UserId::test_default()).await.unwrap());
    }

    #[tokio::test]
    async fn test_list_users_returns_flags() {
        let (_db, repository) = repository_with_test_user().await;
        repository
            .create_user(
                &web_audit(),
                &EmailAddress::try_from("second@mail.test").unwrap(),
                password_hash("pw"),
            )
            .await
            .unwrap();
        repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("pw"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();

        let users = repository.list_users().await.unwrap();

        assert_eq!(users.len(), 2);
        // Ordered by email.
        assert_eq!(users[0].email(), &Some("second@mail.test".to_string()));
        assert_eq!(users[1].email(), &Some("test@mail.test".to_string()));

        let second = &users[0];
        assert!(!second.verified());
        assert!(second.has_password());
        assert!(!second.is_admin());

        assert!(users[1].is_admin());
    }

    #[tokio::test]
    async fn test_list_users_surfaces_unparseable_email() {
        let (_db, repository) = repository_with_test_user().await;

        sqlx::query("INSERT INTO t_users (user_id) VALUES ('broken_user');")
            .execute(&repository.writer)
            .await
            .unwrap();
        sqlx::query(
            r#"
        INSERT INTO t_emails (email, user) VALUES (
            'not-an-email', (SELECT rowid FROM t_users WHERE user_id = 'broken_user')
        );"#,
        )
        .execute(&repository.writer)
        .await
        .unwrap();

        let users = repository.list_users().await.unwrap();

        let broken = users
            .iter()
            .find(|user| user.user_id() == &UserId::from("broken_user"))
            .expect("the broken user should be listed");
        assert_eq!(broken.email(), &Some("not-an-email".to_string()));
        assert!(!broken.email_valid());

        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_list_users_includes_users_without_email() {
        let (_db, repository) = repository_with_test_user().await;

        sqlx::query("INSERT INTO t_users (user_id) VALUES ('no_email_user');")
            .execute(&repository.writer)
            .await
            .unwrap();

        let users = repository.list_users().await.unwrap();

        assert_eq!(users.len(), 2);
        let without_email = users
            .iter()
            .find(|user| user.user_id() == &UserId::from("no_email_user"))
            .expect("a user without an email should be listed");
        assert_eq!(without_email.email(), &None);
        assert!(!without_email.email_valid());
        assert!(!without_email.verified());
        assert!(!without_email.has_password());
        assert!(!without_email.is_admin());
    }

    #[tokio::test]
    async fn test_create_user_stores_unverified_user_with_password() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .unwrap();

        let email = EmailAddress::try_from("new@mail.test").unwrap();
        let outcome = repository
            .create_user(&web_audit(), &email, password_hash("s3cret"))
            .await
            .unwrap();

        let CreateUserOutcome::Created(user_id) = outcome else {
            panic!("expected the user to be created");
        };

        let users = repository.list_users().await.unwrap();
        assert_eq!(users.len(), 1);
        let user = &users[0];
        assert_eq!(user.user_id(), &user_id);
        assert_eq!(user.email(), &Some("new@mail.test".to_string()));
        assert!(!user.verified());
        assert!(user.has_password());
        assert!(!user.is_admin());
    }

    #[tokio::test]
    async fn test_create_user_rejects_duplicate_email() {
        let (_db, repository) = repository_with_test_user().await;

        let outcome = repository
            .create_user(&web_audit(), &test_email(), password_hash("s3cret"))
            .await
            .unwrap();

        assert_eq!(outcome, CreateUserOutcome::EmailAlreadyUsed);
        assert_eq!(repository.list_users().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_set_user_password_updates_existing_user() {
        let (_db, repository) = repository_with_test_user().await;

        let outcome = repository
            .set_user_password(&web_audit(), &UserId::test_default(), password_hash("new"))
            .await
            .unwrap();

        assert_eq!(outcome, SetUserPasswordOutcome::Updated);
        let stored: String =
            sqlx::query_scalar("SELECT password_hash FROM t_users WHERE user_id = ?1;")
                .bind(UserId::test_default().to_string())
                .fetch_one(&repository.writer)
                .await
                .unwrap();
        assert!(
            HashedPassword::new(stored)
                .verify_password(&Password::try_from("new".to_string()).unwrap())
        );
    }

    #[tokio::test]
    async fn test_set_user_password_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .unwrap();

        let outcome = repository
            .set_user_password(&web_audit(), &UserId::from("unknown"), password_hash("new"))
            .await
            .unwrap();

        assert_eq!(outcome, SetUserPasswordOutcome::UnknownUser);
    }

    #[tokio::test]
    async fn test_set_user_email_updates_existing_user() {
        let (_db, repository) = repository_with_test_user().await;

        let outcome = repository
            .set_user_email(
                &web_audit(),
                &UserId::test_default(),
                EmailAddress::try_from("new@mail.test").unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(outcome, SetUserEmailOutcome::Updated);

        let stored: String = sqlx::query_scalar(
            "SELECT email FROM t_emails WHERE user = (SELECT rowid FROM t_users WHERE user_id = ?1);",
        )
        .bind(UserId::test_default().to_string())
        .fetch_one(&repository.writer)
        .await
        .unwrap();
        assert_eq!(stored, "new@mail.test");

        let users = repository.list_users().await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].email(), &Some("new@mail.test".to_string()));
        // A changed address starts out unverified.
        assert!(!users[0].verified());
    }

    #[tokio::test]
    async fn test_set_user_email_rejects_taken_email() {
        let (_db, repository) = repository_with_test_user().await;
        repository
            .create_user(
                &web_audit(),
                &EmailAddress::try_from("second@mail.test").unwrap(),
                password_hash("pw"),
            )
            .await
            .unwrap();

        let outcome = repository
            .set_user_email(
                &web_audit(),
                &UserId::test_default(),
                EmailAddress::try_from("second@mail.test").unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(outcome, SetUserEmailOutcome::EmailAddressTaken);

        // The original address is left untouched.
        let users = repository.list_users().await.unwrap();
        let test_user = users
            .iter()
            .find(|user| user.user_id() == &UserId::test_default())
            .unwrap();
        assert_eq!(test_user.email(), &Some("test@mail.test".to_string()));
    }

    #[tokio::test]
    async fn test_set_user_email_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .unwrap();

        let outcome = repository
            .set_user_email(
                &web_audit(),
                &UserId::from("unknown"),
                EmailAddress::try_from("new@mail.test").unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(outcome, SetUserEmailOutcome::UnknownUser);
    }

    #[tokio::test]
    async fn test_set_user_email_inserts_for_user_without_email() {
        let (_db, repository) = repository_with_test_user().await;
        let user = UserId::from("no_email_user");
        sqlx::query("INSERT INTO t_users (user_id) VALUES (?1);")
            .bind(user.to_string())
            .execute(&repository.writer)
            .await
            .unwrap();

        let outcome = repository
            .set_user_email(
                &web_audit(),
                &user,
                EmailAddress::try_from("added@mail.test").unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(outcome, SetUserEmailOutcome::Updated);

        let users = repository.list_users().await.unwrap();
        let added = users.iter().find(|u| u.user_id() == &user).unwrap();
        assert_eq!(added.email(), &Some("added@mail.test".to_string()));
        assert!(!added.verified());
    }

    #[tokio::test]
    async fn test_set_user_email_to_own_current_email_is_allowed() {
        let (_db, repository) = repository_with_test_user().await;

        let outcome = repository
            .set_user_email(&web_audit(), &UserId::test_default(), test_email())
            .await
            .unwrap();

        assert_eq!(outcome, SetUserEmailOutcome::Updated);
        assert_eq!(repository.list_users().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_set_admin_grants_and_revokes_with_last_admin_guard() {
        let (_db, repository) = repository_with_test_user().await;
        repository
            .create_user(
                &web_audit(),
                &EmailAddress::try_from("second@mail.test").unwrap(),
                password_hash("pw"),
            )
            .await
            .unwrap();

        assert_eq!(
            repository
                .set_admin(&web_audit(), &UserId::test_default(), true)
                .await
                .unwrap(),
            SetAdminOutcome::Updated
        );
        assert!(repository.is_admin(&UserId::test_default()).await.unwrap());

        // A single admin cannot be demoted.
        assert_eq!(
            repository
                .set_admin(&web_audit(), &UserId::test_default(), false)
                .await
                .unwrap(),
            SetAdminOutcome::LastAdmin
        );
        assert!(repository.is_admin(&UserId::test_default()).await.unwrap());

        // With a second admin, the first can be demoted.
        let second = repository
            .list_users()
            .await
            .unwrap()
            .into_iter()
            .find(|user| user.email() == &Some("second@mail.test".to_string()))
            .unwrap()
            .user_id()
            .clone();

        assert_eq!(
            repository
                .set_admin(&web_audit(), &second, true)
                .await
                .unwrap(),
            SetAdminOutcome::Updated
        );
        assert_eq!(
            repository
                .set_admin(&web_audit(), &UserId::test_default(), false)
                .await
                .unwrap(),
            SetAdminOutcome::Updated
        );
        assert!(!repository.is_admin(&UserId::test_default()).await.unwrap());
    }

    #[tokio::test]
    async fn test_set_admin_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .unwrap();

        assert_eq!(
            repository
                .set_admin(&web_audit(), &UserId::from("unknown"), true)
                .await
                .unwrap(),
            SetAdminOutcome::UnknownUser
        );
    }

    #[tokio::test]
    async fn test_revoke_user_sessions_only_affects_target_user() {
        let (_db, repository) = repository_with_test_user().await;
        repository
            .create_user(
                &web_audit(),
                &EmailAddress::try_from("second@mail.test").unwrap(),
                password_hash("pw"),
            )
            .await
            .unwrap();
        let second = repository
            .list_users()
            .await
            .unwrap()
            .into_iter()
            .find(|user| user.email() == &Some("second@mail.test".to_string()))
            .unwrap()
            .user_id()
            .clone();

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        for (user, token) in [
            (UserId::test_default(), "first_token"),
            (second.clone(), "second_token"),
        ] {
            repository
                .store_session(
                    &Session::new(user, SessionToken::from(token.to_string()), expire_at).as_hash(),
                )
                .await
                .unwrap();
        }

        let revoked = repository
            .revoke_user_sessions(&web_audit(), &UserId::test_default())
            .await
            .unwrap();

        assert_eq!(revoked, 1);

        // Only the other user's session remains.
        let remaining: String = sqlx::query_scalar(
            r#"
        SELECT u.user_id
        FROM t_sessions s
        JOIN t_users u ON u.rowid = s.user
        ;"#,
        )
        .fetch_one(&repository.readers)
        .await
        .unwrap();
        assert_eq!(remaining, second.to_string());
    }

    async fn audit_rows(
        repository: &SqliteUsersRepository,
    ) -> Vec<(
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        String,
    )> {
        sqlx::query_as(
            r#"
        SELECT occurred_at, origin, actor_user_id, action, target_user_id, target_email, outcome
        FROM t_admin_audit_log
        ORDER BY id
        ;"#,
        )
        .fetch_all(&repository.readers)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_user_is_audited_with_its_actor() {
        let (_db, repository) = repository_with_test_user().await;
        let actor = UserId::from("acting_admin");
        let audit = AdminAuditContext::new(AdminActor::Web(actor.clone()), Utc::now());

        let email = EmailAddress::try_from("new@mail.test").unwrap();
        let CreateUserOutcome::Created(target) = repository
            .create_user(&audit, &email, password_hash("s3cret"))
            .await
            .unwrap()
        else {
            panic!("expected the user to be created");
        };

        let rows = audit_rows(&repository).await;
        assert_eq!(rows.len(), 1);
        let (occurred_at, origin, actor_user_id, action, target_user_id, target_email, outcome) =
            &rows[0];
        assert!(!occurred_at.is_empty());
        assert_eq!(origin, "web");
        assert_eq!(actor_user_id, &Some(actor.to_string()));
        assert_eq!(action, "create_user");
        assert_eq!(target_user_id, &Some(target.to_string()));
        assert_eq!(target_email.as_deref(), Some("new@mail.test"));
        assert_eq!(outcome, "created");
    }

    #[tokio::test]
    async fn test_cli_actions_are_audited_without_an_actor() {
        let (_db, repository) = repository_with_test_user().await;
        let audit = AdminAuditContext::new(AdminActor::Cli, Utc::now());

        repository
            .promote_admin(&audit, &test_email())
            .await
            .unwrap();

        let rows = audit_rows(&repository).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "cli");
        assert_eq!(rows[0].2, None);
        assert_eq!(rows[0].3, "promote_admin");
        assert_eq!(rows[0].6, "promoted");
    }

    #[tokio::test]
    async fn test_refused_admin_actions_are_audited() {
        let (_db, repository) = repository_with_test_user().await;

        // Unknown user: refused promotion.
        let unknown = EmailAddress::try_from("ghost@mail.test").unwrap();
        assert_eq!(
            repository
                .promote_admin(&web_audit(), &unknown)
                .await
                .unwrap(),
            PromoteAdminOutcome::UnknownUser
        );

        // Last admin: refused demotion.
        repository
            .create_or_promote_admin(
                &web_audit(),
                &test_email(),
                password_hash("pw"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();
        assert_eq!(
            repository
                .set_admin(&web_audit(), &UserId::test_default(), false)
                .await
                .unwrap(),
            SetAdminOutcome::LastAdmin
        );

        let outcomes: Vec<(String, String)> =
            sqlx::query_as("SELECT action, outcome FROM t_admin_audit_log ORDER BY id;")
                .fetch_all(&repository.readers)
                .await
                .unwrap();

        assert_eq!(
            outcomes,
            vec![
                ("promote_admin".to_string(), "unknown_user".to_string()),
                ("bootstrap_admin".to_string(), "promoted".to_string()),
                ("set_admin".to_string(), "last_admin".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn test_password_audit_does_not_leak_secrets() {
        let (_db, repository) = repository_with_test_user().await;

        repository
            .set_user_password(
                &web_audit(),
                &UserId::test_default(),
                password_hash("super-secret"),
            )
            .await
            .unwrap();

        let details: Option<String> = sqlx::query_scalar("SELECT details FROM t_admin_audit_log;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert_eq!(details, None);

        let everything: String = sqlx::query_scalar(
            r#"
        SELECT group_concat(
            COALESCE(details, '') || ' ' || COALESCE(target_email, '') || ' ' || COALESCE(outcome, ''),
            ' '
        ) FROM t_admin_audit_log;"#,
        )
        .fetch_one(&repository.readers)
        .await
        .unwrap();
        assert!(!everything.contains("super-secret"));
    }

    #[tokio::test]
    async fn test_audit_rows_survive_user_deletion() {
        let (_db, repository) = repository_with_test_user().await;

        repository
            .promote_admin(&web_audit(), &test_email())
            .await
            .unwrap();
        sqlx::query("DELETE FROM t_users WHERE user_id = ?1;")
            .bind(UserId::test_default().to_string())
            .execute(&repository.writer)
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM t_admin_audit_log;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert_eq!(count, 1, "the audit trail must outlive the user");
    }

    #[tokio::test]
    async fn test_init_tables() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        sqlx::query("select count(*) from t_users;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        sqlx::query("select count(*) from t_auth_links;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        sqlx::query("select count(*) from t_sessions;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        sqlx::query("select count(*) from t_emails;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        sqlx::query("select count(*) from t_admin_audit_log;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_store_user_with_email() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        repository
            .store_user(
                &UserId::test_default(),
                &EmailAddress::try_from("test@mail.test").unwrap(),
                None,
            )
            .await
            .expect("should have store user with email");
    }

    #[tokio::test]
    async fn test_store_user_with_email_reject_if_emails_exists() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        repository
            .store_user(
                &UserId::test_default(),
                &EmailAddress::try_from("test@mail.test").unwrap(),
                None,
            )
            .await
            .unwrap();

        repository
            .store_user(
                &UserId::from("another_user".to_string()),
                &EmailAddress::try_from("test@mail.test").unwrap(),
                None,
            )
            .await
            .expect_err("Should have rejected duplicated email");
    }

    #[tokio::test]
    async fn test_get_user_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert!(
            repository
                .get_user_by_email(&EmailAddress::try_from("test@mail.test").unwrap())
                .await
                .expect("Should not have err")
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_get_user_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        repository
            .store_user(
                &UserId::test_default(),
                &EmailAddress::try_from("test@mail.test").unwrap(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            repository
                .get_user_by_email(&EmailAddress::try_from("test@mail.test").unwrap())
                .await
                .expect("Should not have err"),
            Some((UserId::test_default(), None))
        );
    }

    #[tokio::test]
    async fn test_get_user_returns_stored_password_hash() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let password = Password::try_from("secret-password".to_string()).unwrap();
        let password_hash = password.as_hash().expect("hashing should succeed");
        repository
            .store_user(
                &UserId::test_default(),
                &test_email(),
                Some(password_hash.clone()),
            )
            .await
            .unwrap();

        let (user, stored_hash) = repository
            .get_user_by_email(&test_email())
            .await
            .expect("should not have err")
            .expect("user should exist");

        assert_eq!(user, UserId::test_default());
        let stored_hash = stored_hash.expect("password hash should be returned");
        assert_eq!(stored_hash, password_hash);
        assert!(stored_hash.verify_password(&password));
    }

    #[tokio::test]
    async fn test_stored_email_is_unverified() {
        let (_db_file, repository) = repository_with_test_user().await;

        let verified: i64 = sqlx::query_scalar("select verified from t_emails;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert_eq!(verified, 0);
    }

    #[tokio::test]
    async fn test_consume_auth_link_verifies_the_bound_email() {
        let (_db_file, repository) = repository_with_test_user().await;
        let link = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("test_token".to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash()
        .unwrap();
        repository.store_auth_link(&link).await.unwrap();

        let consumed = repository
            .consume_auth_link(link.user(), link.hash())
            .await
            .expect("consume should succeed");
        assert!(consumed);

        // The link is consumed...
        assert!(repository.get_all_auth_links().await.is_empty());
        // ...and its bound address is marked verified.
        let verified: bool = sqlx::query_scalar("SELECT verified FROM t_emails WHERE email = ?1")
            .bind(test_email().to_string())
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert!(verified);
    }

    #[tokio::test]
    async fn test_consume_auth_link_does_not_verify_a_changed_email() {
        let (_db_file, repository) = repository_with_test_user().await;
        let link = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("test_token".to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash()
        .unwrap();
        repository.store_auth_link(&link).await.unwrap();

        // The user changes to a different (unverified) address before opening the link.
        let new_email = EmailAddress::try_from("new@mail.test").unwrap();
        repository
            .update_user_email(&UserId::test_default(), &new_email, false)
            .await
            .unwrap();

        let consumed = repository
            .consume_auth_link(link.user(), link.hash())
            .await
            .expect("consume should succeed");
        assert!(consumed);

        // The link is consumed, but the changed address stays unverified.
        assert!(repository.get_all_auth_links().await.is_empty());
        let verified: bool = sqlx::query_scalar("SELECT verified FROM t_emails WHERE email = ?1")
            .bind(new_email.to_string())
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert!(!verified);
    }

    #[tokio::test]
    async fn test_consume_auth_link_is_single_use() {
        let (_db_file, repository) = repository_with_test_user().await;
        let link = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("test_token".to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash()
        .unwrap();
        repository.store_auth_link(&link).await.unwrap();

        assert!(
            repository
                .consume_auth_link(link.user(), link.hash())
                .await
                .unwrap()
        );
        // A second, concurrent consumption finds nothing to delete.
        assert!(
            !repository
                .consume_auth_link(link.user(), link.hash())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_consume_auth_link_without_email_only_consumes() {
        let (_db_file, repository) = repository_with_test_user().await;
        let link = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("legacy_token".to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash()
        .unwrap();
        repository.store_auth_link(&link).await.unwrap();

        // Simulate a link created before the address was tracked.
        sqlx::query("UPDATE t_auth_links SET email = NULL;")
            .execute(&repository.writer)
            .await
            .unwrap();

        let consumed = repository
            .consume_auth_link(link.user(), link.hash())
            .await
            .unwrap();
        assert!(consumed);
        assert!(repository.get_all_auth_links().await.is_empty());
        // A legacy link without a bound address verifies nothing.
        let verified: bool = sqlx::query_scalar("SELECT verified FROM t_emails WHERE email = ?1")
            .bind(test_email().to_string())
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert!(!verified);
    }

    #[tokio::test]
    async fn test_user_has_no_password_hash_by_default() {
        let (_db_file, repository) = repository_with_test_user().await;

        let password_hash: Option<String> =
            sqlx::query_scalar("select password_hash from t_users;")
                .fetch_one(&repository.readers)
                .await
                .unwrap();
        assert_eq!(password_hash, None);
    }

    #[tokio::test]
    async fn test_count_users_without_password() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Empty repository: nobody is missing a password.
        assert_eq!(repository.count_users_without_password().await.unwrap(), 0);

        repository
            .store_user(&UserId::test_default(), &test_email(), None)
            .await
            .unwrap();
        repository
            .store_user(
                &UserId::from("other_user".to_string()),
                &EmailAddress::try_from("other@mail.test").unwrap(),
                Some(test_password_hash()),
            )
            .await
            .unwrap();

        assert_eq!(repository.count_users_without_password().await.unwrap(), 1);

        repository
            .set_user_password_if_absent(&UserId::test_default(), &test_password_hash())
            .await
            .unwrap();

        assert_eq!(repository.count_users_without_password().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_stored_password_hash_can_be_verified_against_password() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let password = Password::try_from("secret-password".to_string()).unwrap();
        let password_hash = password.as_hash().expect("hashing should succeed");
        repository
            .store_user(&UserId::test_default(), &test_email(), Some(password_hash))
            .await
            .expect("should have stored user with password hash");

        let stored_hash: Option<String> = sqlx::query_scalar("select password_hash from t_users;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        let stored_hash = stored_hash.expect("password hash should be stored");
        let parsed =
            PasswordHash::new(&stored_hash).expect("stored hash should be a valid PHC string");
        Argon2::default()
            .verify_password("secret-password".as_bytes(), &parsed)
            .expect("stored hash should verify against the original password");
    }

    #[tokio::test]
    async fn test_user_can_have_multiple_emails() {
        let (_db_file, repository) = repository_with_test_user().await;

        sqlx::query(
            r#"
            INSERT INTO t_emails (email, user) VALUES (
                'other@mail.test', (SELECT rowid FROM t_users WHERE user_id = ?1)
            );"#,
        )
        .bind(UserId::test_default().to_string())
        .execute(&repository.writer)
        .await
        .unwrap();

        for mail in ["test@mail.test", "other@mail.test"] {
            assert_eq!(
                repository
                    .get_user_by_email(&EmailAddress::try_from(mail).unwrap())
                    .await
                    .expect("should not have err"),
                Some((UserId::test_default(), None))
            );
        }
    }

    #[tokio::test]
    async fn test_store_auth_link() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let hashed_auth_link = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("test_token".to_string()),
            expire_at,
        )
        .as_hash()
        .unwrap();

        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect("store_auth_link should have succeeded");

        let links = repository.get_all_auth_links().await;
        assert_eq!(links.len(), 1);

        let link = links.first().unwrap();
        assert_eq!(link.user(), &UserId::test_default());
        assert_eq!(link.hash(), hashed_auth_link.hash());
        assert_eq!(link.expire_at(), &expire_at);
    }

    #[tokio::test]
    async fn test_store_auth_link_token_for_that_user_already_exists() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);

        // Store a first auth link
        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    test_email(),
                    AuthToken::from("test_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        // Store a second auth link, for the same user, different token
        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    test_email(),
                    AuthToken::from("another_test_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        assert_eq!(repository.get_all_auth_links().await.len(), 2);
    }

    #[tokio::test]
    async fn test_store_auth_link_reject_if_token_already_exists() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_auth_link = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash()
        .unwrap();

        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect("store_auth_link should have succeeded");

        // Store operation should fail for same hash
        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect_err("Should have returned an err");

        assert_eq!(repository.get_all_auth_links().await.len(), 1);

        // Existing token should not be changed
        let link = repository
            .get_all_auth_links()
            .await
            .first()
            .unwrap()
            .clone();
        assert_eq!(link.hash(), hashed_auth_link.hash());
    }

    #[tokio::test]
    async fn test_store_auth_link_reject_if_user_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let hashed_auth_link = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash()
        .unwrap();

        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect_err("Should have rejected auth link for unknown user");

        assert!(repository.get_all_auth_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_deleting_user_cascades_to_auth_links() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    test_email(),
                    AuthToken::from("test_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        sqlx::query("DELETE FROM t_users WHERE user_id = ?1;")
            .bind(UserId::test_default().to_string())
            .execute(&repository.writer)
            .await
            .unwrap();

        assert!(repository.get_all_auth_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_tokens_empty() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert!(repository.get_all_auth_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_tokens() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    test_email(),
                    AuthToken::from("a_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    test_email(),
                    AuthToken::from("another_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        let res = repository.get_all_auth_links().await;

        assert_eq!(res.len(), 2);

        let first_token = res.first().unwrap();
        assert_eq!(first_token.user(), &UserId::test_default());
        assert!(
            first_token
                .hash()
                .verify_token(&AuthToken::from("a_token".to_string()))
        );
        assert_eq!(first_token.expire_at(), &expire_at);
    }

    #[tokio::test]
    async fn test_delete_auth_link_ok() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_auth_link = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash()
        .unwrap();

        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect("store_auth_link should have succeeded");

        assert_eq!(repository.get_all_auth_links().await.len(), 1);

        // Delete operation
        repository
            .delete_auth_link_by_hash(hashed_auth_link.hash())
            .await
            .unwrap();

        assert!(repository.get_all_auth_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_delete_expired_auth_links_only_deletes_expired_ones() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expired = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("expired_token".to_string()),
            Utc::now() - TimeDelta::minutes(5),
        )
        .as_hash()
        .unwrap();
        let still_valid = AuthLink::new(
            UserId::test_default(),
            test_email(),
            AuthToken::from("valid_token".to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash()
        .unwrap();
        repository.store_auth_link(&expired).await.unwrap();
        repository.store_auth_link(&still_valid).await.unwrap();

        repository
            .delete_expired_auth_links(Utc::now())
            .await
            .unwrap();

        let res = repository.get_all_auth_links().await;
        assert_eq!(res.len(), 1);
        assert_eq!(res.first().unwrap().hash(), still_valid.hash());
    }

    #[tokio::test]
    async fn test_delete_auth_link_ok_when_token_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Delete operation
        repository
            .delete_auth_link_by_hash(&HashedAuthToken::new("test_token".to_string()))
            .await
            .unwrap();

        assert!(repository.get_all_auth_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_store_session() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            expire_at,
        )
        .as_hash();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        let session = repository
            .get_session_by_hash(hashed_session.hash())
            .await
            .expect("session should be found");

        assert_eq!(session.user(), &UserId::test_default());
        assert_eq!(session.hash(), hashed_session.hash());
        assert_eq!(session.expire_at(), &expire_at);
    }

    #[tokio::test]
    async fn test_store_session_token_for_that_user_already_exists() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);

        // Store a first session
        repository
            .store_session(
                &Session::new(
                    UserId::test_default(),
                    SessionToken::from("test_token".to_string()),
                    expire_at,
                )
                .as_hash(),
            )
            .await
            .expect("store_session should have succeeded");

        // Store a second session, for the same user, different token
        repository
            .store_session(
                &Session::new(
                    UserId::test_default(),
                    SessionToken::from("another_test_token".to_string()),
                    expire_at,
                )
                .as_hash(),
            )
            .await
            .expect("store_session should have succeeded");

        let session = repository
            .get_session_by_hash(&SessionToken::from("another_test_token".to_string()).as_hash())
            .await
            .expect("session should be found");
        assert_eq!(session.user(), &UserId::test_default());
    }

    #[tokio::test]
    async fn test_store_session_reject_if_token_already_exists() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        // Store operation should fail for same hash
        repository
            .store_session(&hashed_session)
            .await
            .expect_err("Should have returned an err");

        // Existing token should not be changed
        let session = repository
            .get_session_by_hash(hashed_session.hash())
            .await
            .expect("session should be found");
        assert_eq!(session.hash(), hashed_session.hash());
    }

    #[tokio::test]
    async fn test_store_session_reject_if_user_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash();

        repository
            .store_session(&hashed_session)
            .await
            .expect_err("Should have rejected session for unknown user");

        assert!(
            repository
                .get_session_by_hash(hashed_session.hash())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_deleting_user_cascades_to_sessions() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash();
        repository.store_session(&hashed_session).await.unwrap();

        sqlx::query("DELETE FROM t_users WHERE user_id = ?1;")
            .bind(UserId::test_default().to_string())
            .execute(&repository.writer)
            .await
            .unwrap();

        assert!(
            repository
                .get_session_by_hash(hashed_session.hash())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_get_session_by_hash_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert!(
            repository
                .get_session_by_hash(&HashedSessionToken::new("test_token".to_string()))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_delete_session_ok() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        // Delete operation
        repository
            .delete_session_by_hash(hashed_session.hash())
            .await
            .unwrap();

        assert!(
            repository
                .get_session_by_hash(hashed_session.hash())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_delete_expired_sessions_only_deletes_expired_ones() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expired = Session::new(
            UserId::test_default(),
            SessionToken::from("expired_token".to_string()),
            Utc::now() - TimeDelta::minutes(5),
        )
        .as_hash();
        let still_valid = Session::new(
            UserId::test_default(),
            SessionToken::from("valid_token".to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash();
        repository.store_session(&expired).await.unwrap();
        repository.store_session(&still_valid).await.unwrap();

        repository
            .delete_expired_sessions(Utc::now())
            .await
            .unwrap();

        assert!(
            repository
                .get_session_by_hash(expired.hash())
                .await
                .is_none()
        );
        assert!(
            repository
                .get_session_by_hash(still_valid.hash())
                .await
                .is_some()
        );
    }

    #[tokio::test]
    async fn test_delete_session_ok_when_token_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Delete operation
        repository
            .delete_session_by_hash(&HashedSessionToken::new("test_token".to_string()))
            .await
            .unwrap();

        assert!(
            repository
                .get_session_by_hash(&HashedSessionToken::new("test_token".to_string()))
                .await
                .is_none()
        );
    }

    fn session_for(token: &str) -> HashedSession {
        Session::new(
            UserId::test_default(),
            SessionToken::from(token.to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash()
    }

    #[tokio::test]
    async fn test_delete_sessions_for_user_removes_all() {
        let (_db_file, repository) = repository_with_test_user().await;
        let first = session_for("first");
        let second = session_for("second");
        repository.store_session(&first).await.unwrap();
        repository.store_session(&second).await.unwrap();

        repository
            .delete_sessions_for_user(&UserId::test_default())
            .await
            .unwrap();

        assert!(repository.get_session_by_hash(first.hash()).await.is_none());
        assert!(
            repository
                .get_session_by_hash(second.hash())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_delete_sessions_for_user_except_keeps_current() {
        let (_db_file, repository) = repository_with_test_user().await;
        let current = session_for("current");
        let other = session_for("other");
        repository.store_session(&current).await.unwrap();
        repository.store_session(&other).await.unwrap();

        repository
            .delete_sessions_for_user_except(&UserId::test_default(), current.hash())
            .await
            .unwrap();

        assert!(
            repository
                .get_session_by_hash(current.hash())
                .await
                .is_some()
        );
        assert!(repository.get_session_by_hash(other.hash()).await.is_none());
    }

    #[tokio::test]
    async fn test_delete_sessions_for_user_does_not_touch_other_users() {
        let (_db_file, repository) = repository_with_test_user().await;
        let other_user = UserId::from("other_user".to_string());
        repository
            .store_user(
                &other_user,
                &EmailAddress::try_from("other@mail.test").unwrap(),
                None,
            )
            .await
            .unwrap();

        let mine = session_for("mine");
        let theirs = Session::new(
            other_user,
            SessionToken::from("theirs".to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash();
        repository.store_session(&mine).await.unwrap();
        repository.store_session(&theirs).await.unwrap();

        repository
            .delete_sessions_for_user(&UserId::test_default())
            .await
            .unwrap();

        assert!(repository.get_session_by_hash(mine.hash()).await.is_none());
        assert!(
            repository
                .get_session_by_hash(theirs.hash())
                .await
                .is_some()
        );
    }

    /// Builds a pre-004 schema (migrations 001 to 003 applied by hand), seeds
    /// it with users, an auth link and a session, marks those migrations as
    /// applied so that initializing the repository only runs 004, and checks
    /// that the data survives the table rebuild with its foreign keys intact.
    #[tokio::test]
    async fn test_email_split_migration_preserves_data() {
        let db_file = NamedTempFile::new().unwrap();
        let migrator = sqlx::migrate!("migrations/users");

        // Apply migrations 001-003 manually to build the pre-004 schema.
        let options = SqliteConnectOptions::from_str(&db_file.path().to_string_lossy())
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePool::connect_with(options).await.unwrap();

        {
            let mut conn = pool.acquire().await.unwrap();
            for migration in migrator.migrations.iter().take(3) {
                sqlx::raw_sql(migration.sql.as_ref())
                    .execute(&mut *conn)
                    .await
                    .unwrap();
            }
        }

        // Seed pre-004 data: two users, an auth link for the first one and a
        // session for the second one (rowids are auto-assigned in order).
        sqlx::query("INSERT INTO t_users (email, user_id) VALUES ('a@mail.test', 'user_a');")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO t_users (email, user_id) VALUES ('b@mail.test', 'user_b');")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO t_auth_links (user, token_hash, expire_at)
             VALUES (1, 'link_hash', '2030-01-01T00:00:00+00:00');",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO t_sessions (user, token_hash, expire_at)
             VALUES (2, 'session_hash', '2030-01-01T00:00:00+00:00');",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Mark migrations 001-003 as applied so that the migrator run below
        // only applies 004.
        sqlx::raw_sql(
            "CREATE TABLE _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        for migration in migrator.migrations.iter().take(3) {
            sqlx::query(
                "INSERT INTO _sqlx_migrations (version, description, success, checksum,
                 execution_time) VALUES (?1, ?2, TRUE, ?3, 0);",
            )
            .bind(migration.version)
            .bind(migration.description.as_ref())
            .bind(migration.checksum.as_ref())
            .execute(&pool)
            .await
            .unwrap();
        }
        pool.close().await;

        // Initializing the repository applies 004 on the seeded data.
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Users are preserved and resolvable through their migrated emails.
        assert_eq!(
            repository
                .get_user_by_email(&EmailAddress::try_from("a@mail.test").unwrap())
                .await
                .expect("should not have err"),
            Some((UserId::from("user_a".to_string()), None))
        );
        assert_eq!(
            repository
                .get_user_by_email(&EmailAddress::try_from("b@mail.test").unwrap())
                .await
                .expect("should not have err"),
            Some((UserId::from("user_b".to_string()), None))
        );

        // Emails of existing users are migrated as verified.
        let emails: Vec<(String, i64)> =
            sqlx::query_as("SELECT email, verified FROM t_emails ORDER BY email;")
                .fetch_all(&repository.readers)
                .await
                .unwrap();
        assert_eq!(
            emails,
            [
                ("a@mail.test".to_string(), 1),
                ("b@mail.test".to_string(), 1)
            ]
        );

        // Auth links and sessions are ephemeral: the migration drops them.
        assert!(repository.get_all_auth_links().await.is_empty());
        assert!(
            repository
                .get_session_by_hash(&HashedSessionToken::new("session_hash".to_string()))
                .await
                .is_none()
        );

        // Existing users are migrated without a password hash.
        let password_hashes: Vec<Option<String>> =
            sqlx::query_scalar("SELECT password_hash FROM t_users ORDER BY rowid;")
                .fetch_all(&repository.readers)
                .await
                .unwrap();
        assert_eq!(password_hashes, [None, None]);
    }

    fn test_password_hash() -> HashedPassword {
        Password::try_from("secret".to_string())
            .unwrap()
            .as_hash()
            .unwrap()
    }

    fn other_test_password_hash() -> HashedPassword {
        Password::try_from("another-secret".to_string())
            .unwrap()
            .as_hash()
            .unwrap()
    }

    #[tokio::test]
    async fn test_get_user_password_returns_none_without_password() {
        let (_db_file, repository) = repository_with_test_user().await;

        let password = repository
            .get_user_password_hash(&UserId::test_default())
            .await
            .unwrap();
        assert_eq!(password, None);
    }

    #[tokio::test]
    async fn test_get_user_password_returns_unknown_user_as_none() {
        let (_db_file, repository) = repository_with_test_user().await;

        let password = repository
            .get_user_password_hash(&UserId::from("unknown_user".to_string()))
            .await
            .unwrap();
        assert_eq!(password, None);
    }

    #[tokio::test]
    async fn test_get_user_password_returns_stored_hash() {
        let (_db_file, repository) = repository_with_test_user().await;
        let hash = test_password_hash();
        repository
            .set_user_password_if_absent(&UserId::test_default(), &hash)
            .await
            .unwrap();

        let stored = repository
            .get_user_password_hash(&UserId::test_default())
            .await
            .unwrap();
        assert_eq!(stored, Some(hash));
    }

    #[tokio::test]
    async fn test_set_user_password_if_absent_sets_when_no_password() {
        let (_db_file, repository) = repository_with_test_user().await;
        let hash = test_password_hash();

        let set = repository
            .set_user_password_if_absent(&UserId::test_default(), &hash)
            .await
            .unwrap();

        assert!(set);
        let stored = repository
            .get_user_password_hash(&UserId::test_default())
            .await
            .unwrap();
        assert_eq!(stored, Some(hash));
    }

    #[tokio::test]
    async fn test_set_user_password_if_absent_refuses_when_password_exists() {
        let (_db_file, repository) = repository_with_test_user().await;
        let first = test_password_hash();
        repository
            .set_user_password_if_absent(&UserId::test_default(), &first)
            .await
            .unwrap();

        let second = other_test_password_hash();
        let set = repository
            .set_user_password_if_absent(&UserId::test_default(), &second)
            .await
            .unwrap();

        assert!(!set, "the second registration must not overwrite the first");
        let stored = repository
            .get_user_password_hash(&UserId::test_default())
            .await
            .unwrap();
        assert_eq!(stored, Some(first));
    }

    #[tokio::test]
    async fn test_set_user_password_if_absent_fails_for_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let set = repository
            .set_user_password_if_absent(&UserId::test_default(), &test_password_hash())
            .await
            .unwrap();
        assert!(!set, "no row exists, so nothing was set");
    }

    #[tokio::test]
    async fn test_replace_user_password_if_matches_replaces_on_match() {
        let (_db_file, repository) = repository_with_test_user().await;
        let first = test_password_hash();
        repository
            .set_user_password_if_absent(&UserId::test_default(), &first)
            .await
            .unwrap();
        let second = other_test_password_hash();

        let replaced = repository
            .replace_user_password_if_matches(&UserId::test_default(), &first, &second)
            .await
            .unwrap();

        assert!(replaced);
        let stored = repository
            .get_user_password_hash(&UserId::test_default())
            .await
            .unwrap();
        assert_eq!(stored, Some(second));
    }

    #[tokio::test]
    async fn test_replace_user_password_if_matches_refuses_on_stale_hash() {
        let (_db_file, repository) = repository_with_test_user().await;
        let first = test_password_hash();
        repository
            .set_user_password_if_absent(&UserId::test_default(), &first)
            .await
            .unwrap();
        let second = other_test_password_hash();
        repository
            .replace_user_password_if_matches(&UserId::test_default(), &first, &second)
            .await
            .unwrap();

        // The now-stale `first` hash must not be able to overwrite `second`.
        let third = Password::try_from("third-secret".to_string())
            .unwrap()
            .as_hash()
            .unwrap();
        let replaced = repository
            .replace_user_password_if_matches(&UserId::test_default(), &first, &third)
            .await
            .unwrap();

        assert!(
            !replaced,
            "a stale current hash must not overwrite the new one"
        );
        let stored = repository
            .get_user_password_hash(&UserId::test_default())
            .await
            .unwrap();
        assert_eq!(stored, Some(second));
    }

    #[tokio::test]
    async fn test_replace_user_password_if_matches_refuses_when_no_password() {
        let (_db_file, repository) = repository_with_test_user().await;

        let replaced = repository
            .replace_user_password_if_matches(
                &UserId::test_default(),
                &test_password_hash(),
                &other_test_password_hash(),
            )
            .await
            .unwrap();

        assert!(!replaced, "no password is stored, so nothing was replaced");
    }

    #[tokio::test]
    async fn test_replace_user_password_if_matches_refuses_for_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let replaced = repository
            .replace_user_password_if_matches(
                &UserId::test_default(),
                &test_password_hash(),
                &other_test_password_hash(),
            )
            .await
            .unwrap();
        assert!(!replaced, "no row exists, so nothing was replaced");
    }

    fn new_email() -> EmailAddress {
        EmailAddress::try_from("new@mail.test").unwrap()
    }

    fn stored_email_change_token() -> HashedEmailChangeToken {
        HashedEmailChangeToken::new(
            UserId::test_default(),
            new_email(),
            AuthToken::new().as_hash().unwrap(),
            Utc::now() + TimeDelta::minutes(15),
        )
    }

    #[tokio::test]
    async fn test_get_user_email() {
        let (_db_file, repository) = repository_with_test_user().await;

        let email = repository
            .get_user_email(&UserId::test_default())
            .await
            .unwrap();
        assert_eq!(email, Some(test_email()));

        let missing = repository
            .get_user_email(&UserId::from("unknown".to_string()))
            .await
            .unwrap();
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_store_email_change_token() {
        let (_db_file, repository) = repository_with_test_user().await;
        let token = stored_email_change_token();

        repository
            .store_email_change_token(&token)
            .await
            .expect("store_email_change_token should have succeeded");

        let tokens = repository.get_all_email_change_tokens().await;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].user(), token.user());
        assert_eq!(tokens[0].new_email(), token.new_email());
        assert_eq!(tokens[0].hash(), token.hash());
        assert_eq!(tokens[0].expire_at(), token.expire_at());
    }

    #[tokio::test]
    async fn test_store_email_change_token_rejects_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let token = HashedEmailChangeToken::new(
            UserId::test_default(),
            new_email(),
            AuthToken::new().as_hash().unwrap(),
            Utc::now() + TimeDelta::minutes(15),
        );
        repository
            .store_email_change_token(&token)
            .await
            .expect_err("unknown user should be rejected");
    }

    // #[tokio::test]
    // async fn test_delete_email_change_tokens_for_user() {
    //     let (_db_file, repository) = repository_with_test_user().await;
    //     let token = stored_email_change_token();
    //     repository.store_email_change_token(&token).await.unwrap();

    //     // Deleting for another user is a no-op.
    //     repository
    //         .delete_email_change_tokens_for_user(&UserId::from("other".to_string()))
    //         .await
    //         .unwrap();
    //     assert_eq!(repository.get_all_email_change_tokens().await.len(), 1);

    //     repository
    //         .delete_email_change_tokens_for_user(&UserId::test_default())
    //         .await
    //         .unwrap();
    //     assert!(repository.get_all_email_change_tokens().await.is_empty());
    // }

    #[tokio::test]
    async fn test_delete_email_change_token_by_hash_and_expiry() {
        let (_db_file, repository) = repository_with_test_user().await;
        let token = stored_email_change_token();
        repository.store_email_change_token(&token).await.unwrap();

        // Not expired yet: cleanup is a no-op.
        repository
            .delete_expired_email_change_tokens(Utc::now())
            .await
            .unwrap();
        assert_eq!(repository.get_all_email_change_tokens().await.len(), 1);

        repository
            .delete_expired_email_change_tokens(*token.expire_at() + TimeDelta::seconds(1))
            .await
            .unwrap();
        assert!(repository.get_all_email_change_tokens().await.is_empty());

        // Store another token and delete it by hash.
        let token = stored_email_change_token();
        repository.store_email_change_token(&token).await.unwrap();
        repository
            .delete_email_change_token_by_hash(token.hash())
            .await
            .unwrap();
        assert!(repository.get_all_email_change_tokens().await.is_empty());
    }

    #[tokio::test]
    async fn test_update_user_email_marks_unverified_by_default() {
        let (_db_file, repository) = repository_with_test_user().await;

        repository
            .update_user_email(&UserId::test_default(), &new_email(), false)
            .await
            .expect("update_user_email should have succeeded");

        // The new email is stored, marked unverified, and resolves to the same user.
        let (user, _) = repository
            .get_user_by_email(&new_email())
            .await
            .unwrap()
            .expect("new email should resolve to the user");
        assert_eq!(user, UserId::test_default());
        let verified: bool = sqlx::query_scalar("SELECT verified FROM t_emails WHERE email = ?1")
            .bind(new_email().to_string())
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert!(!verified);
    }

    #[tokio::test]
    async fn test_update_user_email_rejects_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        repository
            .update_user_email(&UserId::test_default(), &new_email(), false)
            .await
            .expect_err("unknown user should be rejected");
    }

    #[tokio::test]
    async fn test_finalize_email_change_swaps_email_and_verifies() {
        let (_db_file, repository) = repository_with_test_user().await;
        let token = stored_email_change_token();
        repository.store_email_change_token(&token).await.unwrap();

        let res = repository
            .finalize_email_change(token.user(), token.hash())
            .await
            .unwrap();
        // The swapped email is the one carried by the consumed token row.
        assert_eq!(res, EmailChangeFinalizeResult::Swapped(new_email()));

        // Email swapped, marked verified, and the token was consumed.
        let verified: bool = sqlx::query_scalar("SELECT verified FROM t_emails WHERE email = ?1")
            .bind(new_email().to_string())
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert!(verified);
        assert!(repository.get_all_email_change_tokens().await.is_empty());
    }

    #[tokio::test]
    async fn test_finalize_email_change_consumes_token_even_if_email_taken() {
        let (_db_file, repository) = repository_with_test_user().await;
        let token = stored_email_change_token();
        repository.store_email_change_token(&token).await.unwrap();

        // Another user owns the target email.
        let other = UserId::from("other_user".to_string());
        repository
            .store_user(&other, &new_email(), None)
            .await
            .unwrap();

        let res = repository
            .finalize_email_change(token.user(), token.hash())
            .await
            .unwrap();
        assert_eq!(res, EmailChangeFinalizeResult::EmailAlreadyInUse);

        // The token was consumed, and the requesting user kept their original email.
        assert!(repository.get_all_email_change_tokens().await.is_empty());
        assert_eq!(
            repository
                .get_user_email(&UserId::test_default())
                .await
                .unwrap(),
            Some(test_email())
        );
    }

    #[tokio::test]
    async fn test_finalize_email_change_unknown_token() {
        let (_db_file, repository) = repository_with_test_user().await;

        let res = repository
            .finalize_email_change(
                &UserId::test_default(),
                &AuthToken::new().as_hash().unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res, EmailChangeFinalizeResult::UnknownToken);
    }

    #[tokio::test]
    async fn test_finalize_email_change_allows_keeping_unchanged_email() {
        // Changing to the current email is a service-level no-op, but if it ever reaches the
        // repository, the user's own row must not count as "in use".
        let (_db_file, repository) = repository_with_test_user().await;
        let token = HashedEmailChangeToken::new(
            UserId::test_default(),
            test_email(),
            AuthToken::new().as_hash().unwrap(),
            Utc::now() + TimeDelta::minutes(15),
        );
        repository.store_email_change_token(&token).await.unwrap();

        let res = repository
            .finalize_email_change(token.user(), token.hash())
            .await
            .unwrap();
        assert_eq!(res, EmailChangeFinalizeResult::Swapped(test_email()));
    }

    #[tokio::test]
    async fn test_get_user_info() {
        let (_db_file, repository) = repository_with_test_user().await;

        // Freshly registered user: unverified email, no password.
        let info = repository
            .get_user_info(&UserId::test_default())
            .await
            .unwrap()
            .expect("user should exist");
        assert_eq!(info.email(), test_email().value());
        assert!(!info.email_verified());
        assert!(!info.has_password());

        // With a password and a verified email.
        repository
            .set_user_password_if_absent(&UserId::test_default(), &test_password_hash())
            .await
            .unwrap();
        repository
            .update_user_email(&UserId::test_default(), &test_email(), true)
            .await
            .unwrap();
        let info = repository
            .get_user_info(&UserId::test_default())
            .await
            .unwrap()
            .unwrap();
        assert!(info.email_verified());
        assert!(info.has_password());
    }

    #[tokio::test]
    async fn test_get_user_info_unknown_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert_eq!(
            repository
                .get_user_info(&UserId::test_default())
                .await
                .unwrap(),
            None
        );
    }
}
