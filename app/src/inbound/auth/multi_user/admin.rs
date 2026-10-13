//! Admin role bootstrap.
//!
//! An admin is a regular user carrying the `is_admin` flag. This module owns the out-of-band
//! operations that create or promote the first admin (used at startup from environment variables
//! and by the break-glass CLI) and that revoke the flag again.
//!
//! The admin features exposed to logged-in users (creating users, setting passwords, ...) build on
//! the same port and service; they are added in a later step.

use super::{EmailAddress, HashedPassword, Password};

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
    HashingFailed,
    Storage,
}

impl std::fmt::Display for AdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
