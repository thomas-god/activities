use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

use base64::{Engine, engine::general_purpose};
use chrono::{DateTime, Utc};
use derive_more::{Constructor, Display};
use email_address::EmailAddress as EmailAddressValidator;
use rand::Rng;

use crate::domain::models::UserId;

pub mod infra;
pub mod services;

#[derive(Debug, Clone, Constructor)]
pub struct AuthenticatedUser(UserId);

impl AuthenticatedUser {
    pub fn user(&self) -> &UserId {
        &self.0
    }
}

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
    type Error = ();
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if EmailAddressValidator::is_valid(&value) {
            return Ok(Self(value));
        }
        Err(())
    }
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

    pub fn as_hash(&self) -> Result<HashedAuthToken, ()> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        match argon2.hash_password(self.0.as_bytes(), &salt) {
            Ok(hash) => Ok(HashedAuthToken::new(hash.to_string())),
            Err(_err) => Err(()),
        }
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

    pub fn as_hash(&self) -> Result<HashedAuthLink, ()> {
        let Ok(hash) = self.token().as_hash() else {
            return Err(());
        };
        Ok(HashedAuthLink::new(
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

    pub fn as_hash(&self) -> Result<HashedSession, ()> {
        let Ok(hash) = self.token().as_hash() else {
            return Err(());
        };
        Ok(HashedSession::new(
            self.user().clone(),
            hash,
            *self.expire_at(),
        ))
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

#[derive(Clone, Debug)]
pub struct SessionToken(String);

#[allow(clippy::new_without_default)]
impl SessionToken {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        let mut random_bytes = [0u8; 24];
        rng.fill(&mut random_bytes);

        Self(general_purpose::URL_SAFE_NO_PAD.encode(random_bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn as_hash(&self) -> Result<HashedSessionToken, ()> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        match argon2.hash_password(self.0.as_bytes(), &salt) {
            Ok(hash) => Ok(HashedSessionToken::new(hash.to_string())),
            Err(_err) => Err(()),
        }
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

impl HashedSessionToken {
    pub fn verify_token(&self, token: &SessionToken) -> bool {
        let Ok(hashed_password) = PasswordHash::new(&self.0) else {
            return false;
        };
        let argon2 = Argon2::default();

        argon2
            .verify_password(token.as_bytes(), &hashed_password)
            .is_ok()
    }
}

impl std::fmt::Display for HashedSessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserRegistrationResult {
    Success,
    Retry,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserLoginResult {
    Success,
    Retry,
}

#[derive(Debug, Clone)]
pub enum AuthLinkValidationResult {
    Success(GenerateSessionTokenResult),
    Invalid,
}

pub trait IUserService: Clone + Send + Sync + 'static {
    fn register_user(
        &self,
        email: EmailAddress,
    ) -> impl Future<Output = UserRegistrationResult> + Send;

    fn login_user(&self, email: EmailAddress) -> impl Future<Output = UserLoginResult> + Send;

    fn validate_auth_link(
        &self,
        token: AuthToken,
    ) -> impl Future<Output = Result<AuthLinkValidationResult, ()>> + Send;

    fn check_session_token(
        &self,
        token: &SessionToken,
    ) -> impl Future<Output = Result<CheckSessionResult, ()>> + Send;
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
    /// user can actually retry (e.g. we fail to send the email containing the auth link).
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

#[derive(Clone, Debug, Constructor)]
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

        impl IUserService for UserService {
            async fn register_user(
                &self,
                email: EmailAddress,
            ) -> UserRegistrationResult;

            async fn login_user(
                &self,
                email: EmailAddress
            ) -> UserLoginResult;

            async fn validate_auth_link(
                &self,
                token: AuthToken,
            ) -> Result<AuthLinkValidationResult, ()>;

            async fn check_session_token(
                &self,
                _token: &SessionToken
            ) -> Result<CheckSessionResult, ()>;
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
}
