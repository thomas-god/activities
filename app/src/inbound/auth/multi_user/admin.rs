//! Admin role and user management.
//!
//! An admin is a regular user carrying the `is_admin` flag. This module owns both the out-of-band
//! bootstrap of the first admin (used at startup from environment variables and by the break-glass
//! CLI) and the admin user-management operations exposed to logged-in admins (listing and creating
//! users, setting passwords, granting or revoking admin, revoking sessions).

use derive_more::Constructor;

use super::{EmailAddress, HashedPassword, Password};
use crate::{config::AdminBootstrapConfig, domain::models::UserId};

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
pub enum PromoteAdminOutcome {
    Promoted,
    AlreadyAdmin,
    UnknownUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokeAdminOutcome {
    Revoked,
    NotAdmin,
    LastAdmin,
    UnknownUser,
}

/// A user as exposed by the admin user-management operations.
///
/// The email is the raw stored value and `email_valid` reports whether it still parses as an
/// [`EmailAddress`]: an account whose address no longer validates (e.g. after a manual database
/// edit) must be surfaced rather than make the whole listing fail.
#[derive(Debug, Clone, PartialEq, Eq, Constructor)]
pub struct AdminUser {
    user_id: UserId,
    email: Option<String>,
    email_valid: bool,
    verified: bool,
    has_password: bool,
    is_admin: bool,
}

impl AdminUser {
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn email(&self) -> &Option<String> {
        &self.email
    }

    pub fn email_valid(&self) -> bool {
        self.email_valid
    }

    pub fn verified(&self) -> bool {
        self.verified
    }

    pub fn has_password(&self) -> bool {
        self.has_password
    }

    pub fn is_admin(&self) -> bool {
        self.is_admin
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateUserOutcome {
    Created(UserId),
    EmailAlreadyUsed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetUserPasswordOutcome {
    Updated,
    UnknownUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetUserEmailOutcome {
    Updated,
    EmailAddressTaken,
    UnknownUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetAdminOutcome {
    Updated,
    UnknownUser,
    LastAdmin,
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
    fn create_or_promote_admin(
        &self,
        email: &EmailAddress,
        password_hash: HashedPassword,
        mode: BootstrapAdminMode,
    ) -> impl Future<Output = Result<BootstrapAdminOutcome, ()>> + Send;

    fn promote_admin(
        &self,
        email: &EmailAddress,
    ) -> impl Future<Output = Result<PromoteAdminOutcome, ()>> + Send;

    fn revoke_admin(
        &self,
        email: &EmailAddress,
    ) -> impl Future<Output = Result<RevokeAdminOutcome, ()>> + Send;

    fn is_admin(&self, user: &UserId) -> impl Future<Output = Result<bool, ()>> + Send;

    fn list_users(&self) -> impl Future<Output = Result<Vec<AdminUser>, ()>> + Send;

    fn create_user(
        &self,
        email: &EmailAddress,
        password_hash: HashedPassword,
    ) -> impl Future<Output = Result<CreateUserOutcome, ()>> + Send;

    fn set_user_password(
        &self,
        user: &UserId,
        password_hash: HashedPassword,
    ) -> impl Future<Output = Result<SetUserPasswordOutcome, ()>> + Send;

    fn set_user_email(
        &self,
        user: &UserId,
        email: EmailAddress,
    ) -> impl Future<Output = Result<SetUserEmailOutcome, ()>> + Send;

    fn set_admin(
        &self,
        user: &UserId,
        is_admin: bool,
    ) -> impl Future<Output = Result<SetAdminOutcome, ()>> + Send;

    fn revoke_user_sessions(&self, user: &UserId) -> impl Future<Output = Result<u64, ()>> + Send;
}

pub trait IAdminService: Clone + Send + Sync + 'static {
    fn is_admin(&self, user: UserId) -> impl Future<Output = Result<bool, AdminError>> + Send;

    fn list_users(&self) -> impl Future<Output = Result<Vec<AdminUser>, AdminError>> + Send;

    fn create_user(
        &self,
        email: EmailAddress,
        password: Password,
    ) -> impl Future<Output = Result<CreateUserOutcome, AdminError>> + Send;

    fn set_user_password(
        &self,
        user: UserId,
        password: Password,
    ) -> impl Future<Output = Result<SetUserPasswordOutcome, AdminError>> + Send;

    fn set_user_email(
        &self,
        user: UserId,
        email: EmailAddress,
    ) -> impl Future<Output = Result<SetUserEmailOutcome, AdminError>> + Send;

    fn set_admin(
        &self,
        user: UserId,
        is_admin: bool,
    ) -> impl Future<Output = Result<SetAdminOutcome, AdminError>> + Send;

    fn revoke_user_sessions(
        &self,
        user: UserId,
    ) -> impl Future<Output = Result<u64, AdminError>> + Send;
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

    pub async fn promote_admin(
        &self,
        email: EmailAddress,
    ) -> Result<PromoteAdminOutcome, AdminError> {
        self.repository
            .promote_admin(&email)
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

impl<AR: AdminRepository> IAdminService for AdminService<AR> {
    async fn is_admin(&self, user: UserId) -> Result<bool, AdminError> {
        self.repository
            .is_admin(&user)
            .await
            .map_err(|()| AdminError::Storage)
    }

    async fn list_users(&self) -> Result<Vec<AdminUser>, AdminError> {
        self.repository
            .list_users()
            .await
            .map_err(|()| AdminError::Storage)
    }

    async fn create_user(
        &self,
        email: EmailAddress,
        password: Password,
    ) -> Result<CreateUserOutcome, AdminError> {
        let Some(password_hash) = password.as_hash() else {
            return Err(AdminError::HashingFailed);
        };

        self.repository
            .create_user(&email, password_hash)
            .await
            .map_err(|()| AdminError::Storage)
    }

    async fn set_user_password(
        &self,
        user: UserId,
        password: Password,
    ) -> Result<SetUserPasswordOutcome, AdminError> {
        let Some(password_hash) = password.as_hash() else {
            return Err(AdminError::HashingFailed);
        };

        self.repository
            .set_user_password(&user, password_hash)
            .await
            .map_err(|()| AdminError::Storage)
    }

    async fn set_user_email(
        &self,
        user: UserId,
        email: EmailAddress,
    ) -> Result<SetUserEmailOutcome, AdminError> {
        self.repository
            .set_user_email(&user, email)
            .await
            .map_err(|()| AdminError::Storage)
    }

    async fn set_admin(&self, user: UserId, is_admin: bool) -> Result<SetAdminOutcome, AdminError> {
        self.repository
            .set_admin(&user, is_admin)
            .await
            .map_err(|()| AdminError::Storage)
    }

    async fn revoke_user_sessions(&self, user: UserId) -> Result<u64, AdminError> {
        self.repository
            .revoke_user_sessions(&user)
            .await
            .map_err(|()| AdminError::Storage)
    }
}

/// No-op admin service for the single-user modes, where the admin role does not exist and the
/// admin routes are never mounted.
#[derive(Debug, Clone, Copy, Default)]
pub struct DisabledAdminService {}

impl IAdminService for DisabledAdminService {
    async fn is_admin(&self, _user: UserId) -> Result<bool, AdminError> {
        Err(AdminError::Storage)
    }

    async fn list_users(&self) -> Result<Vec<AdminUser>, AdminError> {
        Err(AdminError::Storage)
    }

    async fn create_user(
        &self,
        _email: EmailAddress,
        _password: Password,
    ) -> Result<CreateUserOutcome, AdminError> {
        Err(AdminError::Storage)
    }

    async fn set_user_password(
        &self,
        _user: UserId,
        _password: Password,
    ) -> Result<SetUserPasswordOutcome, AdminError> {
        Err(AdminError::Storage)
    }

    async fn set_user_email(
        &self,
        _user: UserId,
        _email: EmailAddress,
    ) -> Result<SetUserEmailOutcome, AdminError> {
        Err(AdminError::Storage)
    }

    async fn set_admin(
        &self,
        _user: UserId,
        _is_admin: bool,
    ) -> Result<SetAdminOutcome, AdminError> {
        Err(AdminError::Storage)
    }

    async fn revoke_user_sessions(&self, _user: UserId) -> Result<u64, AdminError> {
        Err(AdminError::Storage)
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

            async fn promote_admin(
                &self,
                email: &EmailAddress,
            ) -> Result<PromoteAdminOutcome, ()>;

            async fn revoke_admin(
                &self,
                email: &EmailAddress,
            ) -> Result<RevokeAdminOutcome, ()>;

            async fn is_admin(&self, user: &UserId) -> Result<bool, ()>;

            async fn list_users(&self) -> Result<Vec<AdminUser>, ()>;

            async fn create_user(
                &self,
                email: &EmailAddress,
                password_hash: HashedPassword,
            ) -> Result<CreateUserOutcome, ()>;

            async fn set_user_password(
                &self,
                user: &UserId,
                password_hash: HashedPassword,
            ) -> Result<SetUserPasswordOutcome, ()>;

            async fn set_user_email(
                &self,
                user: &UserId,
                email: EmailAddress,
            ) -> Result<SetUserEmailOutcome, ()>;

            async fn set_admin(
                &self,
                user: &UserId,
                is_admin: bool,
            ) -> Result<SetAdminOutcome, ()>;

            async fn revoke_user_sessions(&self, user: &UserId) -> Result<u64, ()>;
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

    fn email(value: &str) -> EmailAddress {
        EmailAddress::try_from(value).unwrap()
    }

    fn user(value: &str) -> UserId {
        UserId::from(value)
    }

    #[tokio::test]
    async fn test_is_admin_forwards_user_identity() {
        let mut repository = MockAdminRepository::new();
        repository
            .expect_is_admin()
            .times(1)
            .withf(|u| u == &user("target"))
            .returning(|_| Ok(true));

        assert!(
            AdminService::new(repository)
                .is_admin(user("target"))
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_is_admin_maps_repository_error() {
        let mut repository = MockAdminRepository::new();
        repository.expect_is_admin().returning(|_| Err(()));

        assert_eq!(
            AdminService::new(repository)
                .is_admin(user("target"))
                .await
                .unwrap_err(),
            AdminError::Storage
        );
    }

    #[tokio::test]
    async fn test_list_users_forwards_to_repository() {
        let mut repository = MockAdminRepository::new();
        repository.expect_list_users().times(1).returning(|| {
            Ok(vec![AdminUser::new(
                user("target"),
                Some("user@mail.test".to_string()),
                true,
                true,
                false,
                false,
            )])
        });

        let users = AdminService::new(repository).list_users().await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].email(), &Some("user@mail.test".to_string()));
    }

    #[tokio::test]
    async fn test_create_user_hashes_password_and_forwards() {
        let mut repository = MockAdminRepository::new();
        repository
            .expect_create_user()
            .times(1)
            .withf(|email, hash| {
                email.value() == "new@mail.test" && hash.verify_password(&parsed_password("s3cret"))
            })
            .returning(|_, _| Ok(CreateUserOutcome::Created(user("new"))));

        let outcome = AdminService::new(repository)
            .create_user(email("new@mail.test"), parsed_password("s3cret"))
            .await
            .unwrap();

        assert_eq!(outcome, CreateUserOutcome::Created(user("new")));
    }

    #[tokio::test]
    async fn test_set_user_password_hashes_password_and_forwards() {
        let mut repository = MockAdminRepository::new();
        repository
            .expect_set_user_password()
            .times(1)
            .withf(|u, hash| {
                u == &user("target") && hash.verify_password(&parsed_password("s3cret"))
            })
            .returning(|_, _| Ok(SetUserPasswordOutcome::Updated));

        let outcome = AdminService::new(repository)
            .set_user_password(user("target"), parsed_password("s3cret"))
            .await
            .unwrap();

        assert_eq!(outcome, SetUserPasswordOutcome::Updated);
    }

    #[tokio::test]
    async fn test_set_admin_forwards_flag_and_outcome() {
        let mut repository = MockAdminRepository::new();
        repository
            .expect_set_admin()
            .times(1)
            .withf(|u, is_admin| u == &user("target") && *is_admin)
            .returning(|_, _| Ok(SetAdminOutcome::LastAdmin));

        let outcome = AdminService::new(repository)
            .set_admin(user("target"), true)
            .await
            .unwrap();

        assert_eq!(outcome, SetAdminOutcome::LastAdmin);
    }

    #[tokio::test]
    async fn test_revoke_user_sessions_returns_count() {
        let mut repository = MockAdminRepository::new();
        repository
            .expect_revoke_user_sessions()
            .times(1)
            .withf(|u| u == &user("target"))
            .returning(|_| Ok(2));

        assert_eq!(
            AdminService::new(repository)
                .revoke_user_sessions(user("target"))
                .await
                .unwrap(),
            2
        );
    }
}
