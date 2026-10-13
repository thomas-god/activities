//! Admin role bootstrap.
//!
//! An admin is a regular user carrying the `is_admin` flag. This module owns the out-of-band
//! operations that create or promote the first admin (used at startup from environment variables
//! and by the break-glass CLI) and that revoke the flag again.
//!
//! The admin features exposed to logged-in users (creating users, setting passwords, ...) build on
//! the same port and service; they are added in a later step.

use super::{EmailAddress, HashedPassword, Password};
use crate::config::AdminBootstrapConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapAdminMode {
    IfNoAdmin,
    Force,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapAdminOutcome {
    Created,
    Promoted,
    PasswordUpdated,
    SkippedAdminExists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokeAdminOutcome {
    Revoked,
    NotAdmin,
    LastAdmin,
    UnknownUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminError {
    InvalidEmail,
    EmptyPassword,
    HashingFailed,
    Storage,
}

impl std::fmt::Display for AdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEmail => write!(f, "invalid admin email address"),
            Self::EmptyPassword => write!(f, "the admin password must not be empty"),
            Self::HashingFailed => write!(f, "failed to hash the admin password"),
            Self::Storage => write!(f, "the user database could not be reached"),
        }
    }
}

impl std::error::Error for AdminError {}

pub trait AdminRepository: Clone + Send + Sync + 'static {
    /// Creates the user if the email is unknown, or promotes the matching user, storing the given
    /// password hash. A created email is marked verified. With [`BootstrapAdminMode::IfNoAdmin`],
    /// the "no admin exists" check and the write must share a transaction so two concurrent
    /// bootstraps cannot both create an admin.
    fn create_or_promote_admin(
        &self,
        email: &EmailAddress,
        password_hash: HashedPassword,
        mode: BootstrapAdminMode,
    ) -> impl Future<Output = Result<BootstrapAdminOutcome, ()>> + Send;

    /// Clears the admin flag of the matching user. Must refuse to revoke the last remaining admin.
    fn revoke_admin(
        &self,
        email: &EmailAddress,
    ) -> impl Future<Output = Result<RevokeAdminOutcome, ()>> + Send;
}

#[derive(Debug, Clone)]
pub struct AdminService<AR> {
    repository: AR,
}

impl<AR: AdminRepository> AdminService<AR> {
    pub fn new(repository: AR) -> Self {
        Self { repository }
    }

    pub async fn bootstrap_admin(
        &self,
        email: EmailAddress,
        password: Password,
        mode: BootstrapAdminMode,
    ) -> Result<BootstrapAdminOutcome, AdminError> {
        let Some(password_hash) = password.as_hash() else {
            return Err(AdminError::HashingFailed);
        };

        self.repository
            .create_or_promote_admin(&email, password_hash, mode)
            .await
            .map_err(|()| AdminError::Storage)
    }

    /// Applies the environment bootstrap configuration (`ACTIVITIES_BOOTSTRAP_ADMIN_*`).
    ///
    /// Only acts when no admin exists yet, so the variables become inert once an admin has been
    /// established.
    pub async fn apply_bootstrap_config(
        &self,
        config: &AdminBootstrapConfig,
    ) -> Result<BootstrapAdminOutcome, AdminError> {
        let email =
            EmailAddress::try_from(config.email.as_str()).map_err(|()| AdminError::InvalidEmail)?;
        let password =
            Password::try_from(config.password.clone()).map_err(|()| AdminError::EmptyPassword)?;

        let outcome = self
            .bootstrap_admin(email, password, BootstrapAdminMode::IfNoAdmin)
            .await?;

        match outcome {
            BootstrapAdminOutcome::SkippedAdminExists => tracing::info!(
                "An admin already exists: ignoring the admin bootstrap configuration, remove it from the deployment."
            ),
            _ => tracing::warn!(
                "Bootstrapped admin `{}` from the admin bootstrap configuration, remove it from the deployment once you have logged in.",
                config.email
            ),
        }

        Ok(outcome)
    }

    pub async fn revoke_admin(
        &self,
        email: EmailAddress,
    ) -> Result<RevokeAdminOutcome, AdminError> {
        self.repository
            .revoke_admin(&email)
            .await
            .map_err(|()| AdminError::Storage)
    }
}

#[cfg(test)]
mod tests {
    use mockall::mock;

    use super::*;

    mock! {
        AdminRepository {}

        impl Clone for AdminRepository {
            fn clone(&self) -> Self;
        }

        impl AdminRepository for AdminRepository {
            async fn create_or_promote_admin(
                &self,
                email: &EmailAddress,
                password_hash: HashedPassword,
                mode: BootstrapAdminMode,
            ) -> Result<BootstrapAdminOutcome, ()>;

            async fn revoke_admin(
                &self,
                email: &EmailAddress,
            ) -> Result<RevokeAdminOutcome, ()>;
        }
    }

    fn bootstrap_config(email: &str, password: &str) -> AdminBootstrapConfig {
        AdminBootstrapConfig {
            email: email.to_string(),
            password: password.to_string(),
        }
    }

    fn parsed_password(password: &str) -> Password {
        Password::try_from(password.to_string()).unwrap()
    }

    #[tokio::test]
    async fn test_apply_bootstrap_config_parses_and_uses_if_no_admin() {
        let mut repository = MockAdminRepository::new();
        repository
            .expect_create_or_promote_admin()
            .times(1)
            .withf(|email, _hash, mode| {
                email.value() == "admin@example.com" && *mode == BootstrapAdminMode::IfNoAdmin
            })
            .returning(|_, _, _| Ok(BootstrapAdminOutcome::Created));

        let outcome = AdminService::new(repository)
            .apply_bootstrap_config(&bootstrap_config("admin@example.com", "s3cret"))
            .await
            .unwrap();

        assert_eq!(outcome, BootstrapAdminOutcome::Created);
    }

    #[tokio::test]
    async fn test_apply_bootstrap_config_rejects_invalid_email() {
        let err = AdminService::new(MockAdminRepository::new())
            .apply_bootstrap_config(&bootstrap_config("not-an-email", "s3cret"))
            .await
            .unwrap_err();

        assert_eq!(err, AdminError::InvalidEmail);
    }

    #[tokio::test]
    async fn test_apply_bootstrap_config_rejects_empty_password() {
        let err = AdminService::new(MockAdminRepository::new())
            .apply_bootstrap_config(&bootstrap_config("admin@example.com", ""))
            .await
            .unwrap_err();

        assert_eq!(err, AdminError::EmptyPassword);
    }

    #[tokio::test]
    async fn test_bootstrap_admin_hashes_password_and_forwards_force_mode() {
        let mut repository = MockAdminRepository::new();
        repository
            .expect_create_or_promote_admin()
            .times(1)
            .withf(|email, hash, mode| {
                email.value() == "admin@example.com"
                    && *mode == BootstrapAdminMode::Force
                    && hash.verify_password(&parsed_password("s3cret"))
            })
            .returning(|_, _, _| Ok(BootstrapAdminOutcome::Promoted));

        let outcome = AdminService::new(repository)
            .bootstrap_admin(
                EmailAddress::try_from("admin@example.com").unwrap(),
                parsed_password("s3cret"),
                BootstrapAdminMode::Force,
            )
            .await
            .unwrap();

        assert_eq!(outcome, BootstrapAdminOutcome::Promoted);
    }
}
