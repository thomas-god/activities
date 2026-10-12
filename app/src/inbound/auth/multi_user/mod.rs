use std::sync::LazyLock;

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

use base64::{Engine, engine::general_purpose};
use chrono::{DateTime, Utc};
use derive_more::{Constructor, Display};
use email_address::EmailAddress as EmailAddressValidator;
use rand::RngExt;
use sha2::{Digest, Sha256};

pub mod infra;
pub mod service;
pub mod session;

pub use service::{DisabledUserService, MultiUserService};
pub use session::{SessionService, spawn_expired_sessions_cleanup};

use crate::{
    config::MultiUserConfig, domain::models::UserId, inbound::http::DoNothingMailProvider,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Display)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for EmailAddress {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if EmailAddressValidator::is_valid(value) {
            return Ok(Self(value.to_string()));
        }
        Err(())
    }
}

impl TryFrom<String> for EmailAddress {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if EmailAddressValidator::is_valid(&value) {
            return Ok(Self(value));
        }
        Err(value)
    }
}

fn argon2_hash(data: &[u8]) -> Option<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(data, &salt)
        .ok()
        .map(|hash| hash.to_string())
}

#[derive(Clone, Debug)]
pub struct AuthToken(String);

#[allow(clippy::new_without_default)]
impl AuthToken {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        let mut random_bytes = [0u8; 24];
        rng.fill(&mut random_bytes);

        Self(general_purpose::URL_SAFE_NO_PAD.encode(random_bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn as_hash(&self) -> Option<HashedAuthToken> {
        argon2_hash(self.0.as_bytes()).map(HashedAuthToken::new)
    }

    /// Performs the same Argon2 hashing work as [`AuthToken::as_hash`] and discards the result.
    ///
    /// Used to equalize the work done for unknown users (which never generate an auth link) with
    /// the found-user path, so their timing matches and cannot be used to enumerate accounts.
    pub fn decoy_hash() {
        let _ = AuthToken::new().as_hash();
    }
}

impl std::fmt::Display for AuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AuthToken {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Constructor, PartialEq)]
pub struct HashedAuthToken(String);

impl HashedAuthToken {
    pub fn verify_token(&self, token: &AuthToken) -> bool {
        let Ok(hashed_password) = PasswordHash::new(&self.0) else {
            return false;
        };
        let argon2 = Argon2::default();

        argon2
            .verify_password(token.as_bytes(), &hashed_password)
            .is_ok()
    }
}

impl std::fmt::Display for HashedAuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Constructor)]
pub struct AuthLink {
    user: UserId,
    token: AuthToken,
    expire_at: chrono::DateTime<Utc>,
}

impl AuthLink {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn token(&self) -> &AuthToken {
        &self.token
    }

    pub fn expire_at(&self) -> &chrono::DateTime<Utc> {
        &self.expire_at
    }

    pub fn as_hash(&self) -> Option<HashedAuthLink> {
        let hash = self.token().as_hash()?;
        Some(HashedAuthLink::new(
            self.user().clone(),
            hash,
            *self.expire_at(),
        ))
    }
}

#[derive(Clone, Debug, Constructor)]
pub struct HashedAuthLink {
    user: UserId,
    hash: HashedAuthToken,
    expire_at: chrono::DateTime<Utc>,
}

impl HashedAuthLink {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn hash(&self) -> &HashedAuthToken {
        &self.hash
    }

    pub fn expire_at(&self) -> &chrono::DateTime<Utc> {
        &self.expire_at
    }

    pub fn is_expired(&self, reference: &chrono::DateTime<Utc>) -> bool {
        reference >= &self.expire_at
    }
}

#[derive(Clone, Debug, Constructor)]
pub struct EmailChangeToken {
    user: UserId,
    new_email: EmailAddress,
    token: AuthToken,
    expire_at: chrono::DateTime<Utc>,
}

impl EmailChangeToken {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn new_email(&self) -> &EmailAddress {
        &self.new_email
    }

    pub fn token(&self) -> &AuthToken {
        &self.token
    }

    pub fn expire_at(&self) -> &chrono::DateTime<Utc> {
        &self.expire_at
    }

    pub fn as_hash(&self) -> Option<HashedEmailChangeToken> {
        let hash = self.token().as_hash()?;
        Some(HashedEmailChangeToken::new(
            self.user().clone(),
            self.new_email().clone(),
            hash,
            *self.expire_at(),
        ))
    }
}

#[derive(Clone, Debug, Constructor)]
pub struct HashedEmailChangeToken {
    user: UserId,
    new_email: EmailAddress,
    hash: HashedAuthToken,
    expire_at: chrono::DateTime<Utc>,
}

impl HashedEmailChangeToken {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn new_email(&self) -> &EmailAddress {
        &self.new_email
    }

    pub fn hash(&self) -> &HashedAuthToken {
        &self.hash
    }

    pub fn expire_at(&self) -> &chrono::DateTime<Utc> {
        &self.expire_at
    }

    pub fn is_expired(&self, reference: &chrono::DateTime<Utc>) -> bool {
        reference >= &self.expire_at
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EmailChangeFinalizeResult {
    Swapped(EmailAddress),
    EmailAlreadyInUse,
    UnknownToken,
}

#[derive(Clone, Debug, Constructor)]
pub struct Session {
    user: UserId,
    token: SessionToken,
    expire_at: chrono::DateTime<Utc>,
}

impl Session {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn token(&self) -> &SessionToken {
        &self.token
    }

    pub fn expire_at(&self) -> &chrono::DateTime<Utc> {
        &self.expire_at
    }

    pub fn as_hash(&self) -> HashedSession {
        HashedSession::new(
            self.user().clone(),
            self.token().as_hash(),
            *self.expire_at(),
        )
    }
}

#[derive(Clone, Debug, Constructor)]
pub struct HashedSession {
    user: UserId,
    hash: HashedSessionToken,
    expire_at: chrono::DateTime<Utc>,
}

impl HashedSession {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn hash(&self) -> &HashedSessionToken {
        &self.hash
    }

    pub fn expire_at(&self) -> &chrono::DateTime<Utc> {
        &self.expire_at
    }

    pub fn is_expired(&self, reference: &chrono::DateTime<Utc>) -> bool {
        reference >= &self.expire_at
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionToken(String);

#[allow(clippy::new_without_default)]
impl SessionToken {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        let mut random_bytes = [0u8; 24];
        rng.fill(&mut random_bytes);

        Self(general_purpose::URL_SAFE_NO_PAD.encode(random_bytes))
    }

    pub fn as_hash(&self) -> HashedSessionToken {
        let mut hasher = Sha256::new();
        hasher.update(self.0.as_bytes());
        HashedSessionToken::new(const_hex::encode(hasher.finalize()))
    }
}

impl From<&str> for SessionToken {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for SessionToken {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Constructor, PartialEq)]
pub struct HashedSessionToken(String);

impl std::fmt::Display for HashedSessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserRegistrationResult {
    Success,
    Retry,
    /// Password-based authentication is active but no password was provided.
    PasswordRequired,
}

#[derive(Clone, PartialEq)]
pub struct Password(String);

impl Password {
    pub fn as_hash(&self) -> Option<HashedPassword> {
        argon2_hash(self.0.as_bytes()).map(HashedPassword::new)
    }
}

impl TryFrom<String> for Password {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }
        Ok(Self(value))
    }
}

// Never leak the password content through debug output.
impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Password([redacted])")
    }
}

#[derive(Clone, Debug, Constructor, PartialEq)]
pub struct HashedPassword(String);

impl HashedPassword {
    pub fn verify_password(&self, password: &Password) -> bool {
        let Ok(hashed_password) = PasswordHash::new(&self.0) else {
            return false;
        };
        let argon2 = Argon2::default();

        argon2
            .verify_password(password.0.as_bytes(), &hashed_password)
            .is_ok()
    }

    /// A decoy hash, generated once at startup from random bytes.
    ///
    /// Its preimage is random and never stored, so no password is expected to verify against it.
    /// Verifying still performs a full Argon2 verification though, matching the cost of checking a
    /// real user hash. Used to equalize the work done for unknown users and users that have no
    /// password stored, so their timing matches a genuine failed password check and cannot be used
    /// to enumerate accounts or detect whether one has a password.
    pub fn dummy() -> &'static Self {
        static DUMMY: LazyLock<HashedPassword> = LazyLock::new(|| {
            let mut rng = rand::rng();
            let mut random_bytes = [0u8; 32];
            rng.fill(&mut random_bytes);
            HashedPassword::new(
                argon2_hash(&random_bytes).expect("hashing random bytes should never fail"),
            )
        });
        &DUMMY
    }

    /// Performs a full Argon2 verification against [`HashedPassword::dummy`].
    ///
    /// Always fails, but costs the same as verifying a real user hash, so callers can use it to
    /// equalize timing between unknown users / users without a stored password and a genuine
    /// wrong-password check. The always-failing outcome is discarded on purpose, so callers cannot
    /// accidentally act on it.
    pub fn decoy_verification(password: &Password) {
        let _ = HashedPassword::dummy().verify_password(password);
    }
}

impl std::fmt::Display for HashedPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct UserRegistrationRequest {
    email: EmailAddress,
    password: Option<Password>,
}

impl UserRegistrationRequest {
    pub fn email(&self) -> &EmailAddress {
        &self.email
    }

    pub fn password(&self) -> &Option<Password> {
        &self.password
    }
}

#[derive(Debug, Clone, Copy, Constructor)]
pub struct AuthMethods {
    email: bool,
    password: bool,
}

impl From<&MultiUserConfig> for AuthMethods {
    fn from(value: &MultiUserConfig) -> Self {
        Self {
            email: value.email.is_some(),
            password: value.password.is_some(),
        }
    }
}

impl AuthMethods {
    pub fn email(&self) -> bool {
        self.email
    }

    pub fn password(&self) -> bool {
        self.password
    }
}

/// Request to register the first password for a user (who has no password yet).
#[derive(Debug, Clone, Constructor)]
pub struct RegisterPasswordRequest {
    user: UserId,
    new: Password,
}

impl RegisterPasswordRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }
    pub fn password(&self) -> &Password {
        &self.new
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegisterPasswordResult {
    Success(GenerateSessionTokenResult),
    PasswordAlreadySet,
    PasswordAuthDisabled,
    Retry,
}

/// Request to replace an existing password, requiring the current one.
#[derive(Debug, Clone, Constructor)]
pub struct UpdatePasswordRequest {
    user: UserId,
    current: Password,
    new: Password,
}

impl UpdatePasswordRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }
    pub fn current(&self) -> &Password {
        &self.current
    }
    pub fn password(&self) -> &Password {
        &self.new
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdatePasswordResult {
    Success(GenerateSessionTokenResult),
    InvalidCurrentPassword,
    PasswordAuthDisabled,
    Retry,
}

#[derive(Debug, Clone, Constructor)]
pub struct UpdateEmailRequest {
    user: UserId,
    new_email: EmailAddress,
}

impl UpdateEmailRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }
    pub fn new_email(&self) -> &EmailAddress {
        &self.new_email
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateEmailResult {
    Changed,
    VerificationEmailSent,
    Retry,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EmailChangeValidationResult {
    Success(EmailAddress),
    EmailAlreadyInUse,
    Invalid,
}

impl From<EmailChangeFinalizeResult> for EmailChangeValidationResult {
    fn from(value: EmailChangeFinalizeResult) -> Self {
        match value {
            EmailChangeFinalizeResult::Swapped(new_email) => Self::Success(new_email),
            EmailChangeFinalizeResult::EmailAlreadyInUse => Self::EmailAlreadyInUse,
            EmailChangeFinalizeResult::UnknownToken => Self::Invalid,
        }
    }
}

/// Information about the authenticated user, exposed at `GET /api/account/me`.
#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct UserInfo {
    email: EmailAddress,
    verified: bool,
    /// Whether the user has registered a password, so clients can offer `register_password` vs
    /// `update_password`.
    has_password: bool,
}

impl UserInfo {
    pub fn email(&self) -> &EmailAddress {
        &self.email
    }

    pub fn verified(&self) -> bool {
        self.verified
    }

    pub fn has_password(&self) -> bool {
        self.has_password
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserInfoResult {
    Success(UserInfo),
    Retry,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserLoginResult {
    /// Mail-based path: an auth link was (or would have been) sent; never reveals whether the
    /// user exists.
    EmailSent,
    /// Password-based path: the password matched and a session was created.
    Session(GenerateSessionTokenResult),
    /// A single, uniform outcome for unknown user / wrong password / user without password.
    Invalid,
    /// Infrastructure failure (repo error, session creation error, hashing failure).
    Retry,
}

#[derive(Debug, Clone)]
pub enum AuthLinkValidationResult {
    Success(GenerateSessionTokenResult),
    Invalid,
}

pub trait IMultiUserService: Clone + Send + Sync + 'static {
    fn register_user(
        &self,
        req: UserRegistrationRequest,
    ) -> impl Future<Output = UserRegistrationResult> + Send;

    fn login_user(
        &self,
        email: EmailAddress,
        password: Option<Password>,
    ) -> impl Future<Output = UserLoginResult> + Send;

    fn validate_auth_link(
        &self,
        token: AuthToken,
    ) -> impl Future<Output = Result<AuthLinkValidationResult, ()>> + Send;

    fn check_session_token(
        &self,
        token: &SessionToken,
    ) -> impl Future<Output = Result<CheckSessionResult, ()>> + Send;

    fn logout(&self, token: &SessionToken) -> impl Future<Output = Result<(), ()>> + Send;

    fn register_password(
        &self,
        req: RegisterPasswordRequest,
    ) -> impl Future<Output = RegisterPasswordResult> + Send;

    fn update_password(
        &self,
        req: UpdatePasswordRequest,
    ) -> impl Future<Output = UpdatePasswordResult> + Send;

    fn update_email(
        &self,
        req: UpdateEmailRequest,
    ) -> impl Future<Output = UpdateEmailResult> + Send;

    fn validate_email_change(
        &self,
        token: AuthToken,
    ) -> impl Future<Output = Result<EmailChangeValidationResult, ()>> + Send;

    fn get_user_info(&self, user: &UserId) -> impl Future<Output = UserInfoResult> + Send;
}

#[derive(Debug, Clone, Constructor)]
pub struct GenerateAuthLinkRequest {
    user: UserId,
    email: EmailAddress,
}

impl GenerateAuthLinkRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn email(&self) -> &EmailAddress {
        &self.email
    }
}

#[derive(Debug, Clone)]
pub enum GenerateAuthLinkResult {
    /// [GenerateAuthLinkResult::Success] covers all functional cases, regardless of user actually
    /// existing to not leak that information.
    Success,
    /// [GenerateAuthLinkResult::Retry] only covers infrastructure related issues for which the
    /// user can actually retry (e.g. we fail to hash the auth token, or to persist it). Failure
    /// to send the email itself is not covered here: that happens in the background after this
    /// result is returned, so it can't be reported to the caller. It's logged instead.
    Retry,
}

pub trait IAuthLinkService: Clone + Send + Sync + 'static {
    fn generate_auth_link(
        &self,
        req: GenerateAuthLinkRequest,
    ) -> impl Future<Output = GenerateAuthLinkResult> + Send;

    fn validate_auth_token(
        &self,
        token: &AuthToken,
    ) -> impl Future<Output = Result<Option<UserId>, ()>> + Send;
}

#[derive(Clone, Debug, Constructor, PartialEq)]
pub struct GenerateSessionTokenResult {
    token: SessionToken,
    expire_at: DateTime<Utc>,
}

impl GenerateSessionTokenResult {
    pub fn token(&self) -> &SessionToken {
        &self.token
    }

    pub fn expire_at(&self) -> &DateTime<Utc> {
        &self.expire_at
    }
}

#[derive(Clone, Debug, Constructor)]
pub struct CheckSessionResult {
    user: UserId,
    refreshed: Option<GenerateSessionTokenResult>,
}

impl CheckSessionResult {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn refreshed(&self) -> &Option<GenerateSessionTokenResult> {
        &self.refreshed
    }
}

pub trait ISessionService: Clone + Send + Sync + 'static {
    fn generate_session_token(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<GenerateSessionTokenResult, ()>> + Send;

    fn check_session_token(
        &self,
        token: &SessionToken,
    ) -> impl Future<Output = Result<CheckSessionResult, ()>> + Send;

    fn logout(&self, token: &SessionToken) -> impl Future<Output = Result<(), ()>> + Send;
}

pub trait MailProvider: Clone + Send + Sync + 'static {
    fn send_auth_link_email(
        &self,
        email: &EmailAddress,
        auth_link: &AuthLink,
    ) -> impl Future<Output = Result<(), ()>> + Send;

    fn send_email_change_email(
        &self,
        email: &EmailAddress,
        email_change_token: &EmailChangeToken,
    ) -> impl Future<Output = Result<(), ()>> + Send;
}

/// Runtime choice between an enabled and a disabled [`MailProvider`].
#[derive(Debug, Clone)]
pub enum MaybeMailProvider<Enabled> {
    Enabled(Enabled),
    Disabled(DoNothingMailProvider),
}

impl<Enabled> MailProvider for MaybeMailProvider<Enabled>
where
    Enabled: MailProvider,
{
    async fn send_auth_link_email(
        &self,
        email: &EmailAddress,
        auth_link: &AuthLink,
    ) -> Result<(), ()> {
        match self {
            Self::Enabled(service) => service.send_auth_link_email(email, auth_link).await,
            Self::Disabled(service) => service.send_auth_link_email(email, auth_link).await,
        }
    }

    async fn send_email_change_email(
        &self,
        email: &EmailAddress,
        email_change_token: &EmailChangeToken,
    ) -> Result<(), ()> {
        match self {
            Self::Enabled(service) => {
                service
                    .send_email_change_email(email, email_change_token)
                    .await
            }
            Self::Disabled(service) => {
                service
                    .send_email_change_email(email, email_change_token)
                    .await
            }
        }
    }
}

#[cfg(test)]
pub mod test_utils {
    use mockall::mock;

    use super::*;

    mock! {
        pub UserService {}

        impl Clone for UserService {
            fn clone(&self) -> Self;
        }

        impl IMultiUserService for UserService {
            async fn register_user(
                &self,
                req: UserRegistrationRequest,
            ) -> UserRegistrationResult;

            async fn login_user(
                &self,
                email: EmailAddress,
                password: Option<Password>
            ) -> UserLoginResult;

            async fn validate_auth_link(
                &self,
                token: AuthToken,
            ) -> Result<AuthLinkValidationResult, ()>;

            async fn check_session_token(
                &self,
                _token: &SessionToken
            ) -> Result<CheckSessionResult, ()>;

            async fn logout(&self, token: &SessionToken) -> Result<(), ()>;

            async fn register_password(
                &self,
                req: RegisterPasswordRequest,
            ) -> RegisterPasswordResult;

            async fn update_password(
                &self,
                req: UpdatePasswordRequest,
            ) -> UpdatePasswordResult;

            async fn update_email(
                &self,
                req: UpdateEmailRequest,
            ) -> UpdateEmailResult;

            async fn validate_email_change(
                &self,
                token: AuthToken,
            ) -> Result<EmailChangeValidationResult, ()>;

            async fn get_user_info(&self, user: &UserId) -> UserInfoResult;
        }
    }

    mock! {
        pub AuthLinkService {}

        impl Clone for AuthLinkService {
            fn clone(&self) -> Self;
        }

        impl IAuthLinkService for AuthLinkService {
            async fn generate_auth_link(
                &self,
                req: GenerateAuthLinkRequest
            ) -> GenerateAuthLinkResult;

            async fn validate_auth_token(
                &self,
                token: &AuthToken
            ) -> Result<Option<UserId>, ()>;
        }
    }

    mock! {
        pub SessionService {}

        impl Clone for SessionService {
            fn clone(&self) -> Self;
        }

        impl ISessionService for SessionService {
            async fn generate_session_token(
                &self,
                user: &UserId,
            ) ->Result<GenerateSessionTokenResult, ()>;

            async fn check_session_token(
                &self,
                _token: &SessionToken
            ) -> Result<CheckSessionResult, ()>;

            async fn logout(&self, token: &SessionToken) -> Result<(), ()>;
        }
    }

    mock! {
        pub MailProvider {}

        impl Clone for MailProvider {
            fn clone(&self) -> Self;
        }

        impl MailProvider for MailProvider {
            async fn send_auth_link_email(
                &self,
                email: &EmailAddress,
                auth_link: &AuthLink,
            ) -> Result<(), ()>;

            async fn send_email_change_email(
                &self,
                email: &EmailAddress,
                email_change_token: &EmailChangeToken,
            ) -> Result<(), ()>;
        }
    }
}

#[cfg(test)]
mod test {
    use chrono::TimeDelta;

    use super::*;

    #[test]
    fn test_auth_link_expiry() {
        let expire_at = chrono::Utc::now();
        let link = HashedAuthLink::new(
            UserId::test_default(),
            AuthToken::new().as_hash().unwrap(),
            expire_at,
        );
        assert!(link.is_expired(&(expire_at + TimeDelta::seconds(1))));
        assert!(!link.is_expired(&(expire_at - TimeDelta::seconds(1))));
    }

    #[test]
    fn test_email_change_token_expiry() {
        let expire_at = chrono::Utc::now();
        let token = HashedEmailChangeToken::new(
            UserId::test_default(),
            EmailAddress::try_from("test@mail.test").unwrap(),
            AuthToken::new().as_hash().unwrap(),
            expire_at,
        );
        assert!(token.is_expired(&(expire_at + TimeDelta::seconds(1))));
        assert!(!token.is_expired(&(expire_at - TimeDelta::seconds(1))));
    }

    #[test]
    fn test_hashed_password_verify() {
        let password = Password::try_from("secret".to_string()).unwrap();
        let hash = password.as_hash().unwrap();

        assert!(hash.verify_password(&password));
        assert!(!hash.verify_password(&Password::try_from("wrong".to_string()).unwrap()));
    }

    #[test]
    fn test_dummy_password_hash_never_verifies() {
        let password = Password::try_from("secret".to_string()).unwrap();

        assert!(!HashedPassword::dummy().verify_password(&password));
    }

    #[test]
    fn test_decoy_verification_runs_for_any_password() {
        // The wrapper discards the (always failing) outcome; it must be safe to call with any
        // password, including one that used to match the old hardcoded decoy preimage.
        HashedPassword::decoy_verification(&Password::try_from("secret".to_string()).unwrap());
        HashedPassword::decoy_verification(
            &Password::try_from("dummy-password-that-never-matches".to_string()).unwrap(),
        );
    }

    #[test]
    fn test_auth_token_decoy_hash_runs() {
        // The wrapper discards the hash; it must be safe to call standalone.
        AuthToken::decoy_hash();
    }
}
