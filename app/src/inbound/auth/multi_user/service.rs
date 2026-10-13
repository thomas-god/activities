use std::sync::Arc;

use chrono::TimeDelta;
use derive_more::Constructor;

use crate::{
    domain::models::UserId,
    inbound::auth::multi_user::{
        AuthLink, AuthLinkValidationResult, AuthMethods, AuthToken, CheckSessionResult,
        EmailAddress, EmailChangeFinalizeResult, EmailChangeToken, EmailChangeValidationResult,
        EmailVerificationResult, GenerateAuthLinkRequest, GenerateAuthLinkResult, HashedAuthLink,
        HashedAuthToken, HashedEmailChangeToken, HashedPassword, HashedSessionToken,
        IMultiUserService, ISessionService, MailProvider, Password, RegisterPasswordRequest,
        RegisterPasswordResult, RevokeSessionsResult, SessionToken, UpdateEmailRequest,
        UpdateEmailResult, UpdatePasswordRequest, UpdatePasswordResult, UserInfo, UserInfoResult,
        UserLoginResult, UserRegistrationRequest, UserRegistrationResult,
    },
};

#[derive(Debug, Clone, Constructor)]
pub struct MultiUserService<UR, SS, MP> {
    user_repository: Arc<UR>,
    session_service: SS,
    auth_methods: AuthMethods,
    mail_provider: Arc<MP>,
}

impl<UR, SS, MP> IMultiUserService for MultiUserService<UR, SS, MP>
where
    UR: UserRepository,
    SS: ISessionService,
    MP: MailProvider,
{
    #[tracing::instrument(skip_all)]
    async fn register_user(&self, req: UserRegistrationRequest) -> UserRegistrationResult {
        let Some(password) = req.password() else {
            return UserRegistrationResult::PasswordRequired;
        };
        let Some(password_hash) = password.as_hash() else {
            return UserRegistrationResult::Retry;
        };

        let email = req.email().clone();
        let user = match self.user_repository.get_user_by_email(&email).await {
            Ok(None) => {
                let user = UserId::new();
                let store_res = self
                    .user_repository
                    .store_user(&user, &email, Some(password_hash))
                    .await;
                if store_res.is_err() {
                    return UserRegistrationResult::Retry;
                }
                user
            }
            // Existing users never get their password hash overwritten via the registration flow.
            Ok(Some((user, _))) => user,
            Err(()) => return UserRegistrationResult::Retry,
        };

        if !self.auth_methods.email() {
            // Email-based auth is disabled: the account is created without any email exchange.
            return UserRegistrationResult::Success;
        }

        match self
            .generate_auth_link(GenerateAuthLinkRequest::new(user, email))
            .await
        {
            GenerateAuthLinkResult::Success => UserRegistrationResult::Success,
            GenerateAuthLinkResult::Retry => UserRegistrationResult::Retry,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn login_user(&self, email: EmailAddress, password: Option<Password>) -> UserLoginResult {
        match password {
            Some(password) => self.login_with_password(email, password).await,
            None if self.auth_methods.email() => self.login_with_email_link(email).await,
            None => UserLoginResult::Invalid,
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn validate_auth_link(&self, token: AuthToken) -> Result<AuthLinkValidationResult, ()> {
        let user = match self.validate_auth_token(&token).await {
            Ok(Some(user)) => user,
            Ok(None) => return Ok(AuthLinkValidationResult::Invalid),
            _ => return Err(()),
        };

        self.session_service
            .generate_session_token(&user)
            .await
            .map(AuthLinkValidationResult::Success)
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn check_session_token(&self, token: &SessionToken) -> Result<CheckSessionResult, ()> {
        self.session_service.check_session_token(token).await
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn logout(&self, token: &SessionToken) -> Result<(), ()> {
        self.session_service.logout(token).await
    }

    #[tracing::instrument(skip_all)]
    async fn revoke_other_sessions(
        &self,
        user: &UserId,
        current: &HashedSessionToken,
    ) -> RevokeSessionsResult {
        match self
            .session_service
            .revoke_other_sessions(user, current)
            .await
        {
            Ok(()) => RevokeSessionsResult::Success,
            Err(()) => RevokeSessionsResult::Retry,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn revoke_all_sessions(&self, user: &UserId) -> RevokeSessionsResult {
        match self.session_service.revoke_all_sessions(user).await {
            Ok(()) => RevokeSessionsResult::Success,
            Err(()) => RevokeSessionsResult::Retry,
        }
    }

    /// Allow an user to register a password if and only if there is no password already set (e.g.)
    /// to avoid locking out users of their account when switching from email-based auth to
    /// password-based auth. Fails if a password already exist, use `update_password` instead.
    #[tracing::instrument(skip_all)]
    async fn register_password(&self, req: RegisterPasswordRequest) -> RegisterPasswordResult {
        let Some(new_hash) = req.password().as_hash() else {
            return RegisterPasswordResult::Retry;
        };

        match self
            .user_repository
            .set_user_password_if_absent(req.user(), &new_hash)
            .await
        {
            Ok(true) => {}
            Ok(false) => return RegisterPasswordResult::PasswordAlreadySet,
            Err(()) => return RegisterPasswordResult::Retry,
        }

        match self
            .session_service
            .generate_session_token(req.user())
            .await
        {
            Ok(session) => RegisterPasswordResult::Success(session),
            Err(()) => RegisterPasswordResult::Retry,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn update_password(&self, req: UpdatePasswordRequest) -> UpdatePasswordResult {
        let current_hash = match self
            .user_repository
            .get_user_password_hash(req.user())
            .await
        {
            Ok(Some(hash)) => hash,
            Ok(None) => {
                HashedPassword::decoy_verification(req.current());
                return UpdatePasswordResult::InvalidCurrentPassword;
            }
            Err(()) => return UpdatePasswordResult::Retry,
        };

        if !current_hash.verify_password(req.current()) {
            return UpdatePasswordResult::InvalidCurrentPassword;
        }

        let Some(new_hash) = req.password().as_hash() else {
            return UpdatePasswordResult::Retry;
        };

        match self
            .user_repository
            .replace_user_password_if_matches(req.user(), &current_hash, &new_hash)
            .await
        {
            Ok(true) => {}
            Ok(false) => return UpdatePasswordResult::InvalidCurrentPassword,
            Err(()) => return UpdatePasswordResult::Retry,
        }

        match self
            .session_service
            .generate_session_token(req.user())
            .await
        {
            Ok(session) => UpdatePasswordResult::Success(session),
            Err(()) => UpdatePasswordResult::Retry,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn update_email(&self, req: UpdateEmailRequest) -> UpdateEmailResult {
        if !self.auth_methods.email() {
            return self.change_email_unverified(req).await;
        }
        self.request_email_change(req).await
    }

    #[tracing::instrument(skip_all)]
    async fn request_email_verification(&self, user: &UserId) -> EmailVerificationResult {
        if !self.auth_methods.email() {
            return EmailVerificationResult::EmailAuthDisabled;
        }

        let email = match self.get_user_info(user).await {
            UserInfoResult::Success(info) if info.verified() => {
                return EmailVerificationResult::AlreadyVerified;
            }
            UserInfoResult::Success(info) => info.email().clone(),
            UserInfoResult::Retry => return EmailVerificationResult::Retry,
        };

        match self
            .generate_auth_link(GenerateAuthLinkRequest::new(user.clone(), email))
            .await
        {
            GenerateAuthLinkResult::Success => EmailVerificationResult::EmailSent,
            GenerateAuthLinkResult::Retry => EmailVerificationResult::Retry,
        }
    }

    /// Consumes an email change verification token and swaps the user's email if it is still
    /// free. Single-use: the token is consumed even when the target email is taken.
    // TODO: reconsider whether an email change should invalidate existing sessions once session
    // revocation is implemented.
    #[tracing::instrument(skip_all, err(Debug))]
    async fn validate_email_change(
        &self,
        token: AuthToken,
    ) -> Result<EmailChangeValidationResult, ()> {
        let tokens = self.user_repository.get_all_email_change_tokens().await;

        let mut found = None;

        let now = chrono::Utc::now();
        for stored in tokens {
            if stored.is_expired(&now) {
                let _ = self
                    .user_repository
                    .delete_email_change_token_by_hash(stored.hash())
                    .await;
                continue;
            }
            if stored.hash().verify_token(&token) {
                found = Some(stored);
            }
        }

        let Some(found) = found else {
            return Ok(EmailChangeValidationResult::Invalid);
        };

        self.user_repository
            .finalize_email_change(found.user(), found.hash())
            .await
            .map(EmailChangeValidationResult::from)
    }

    /// Exposes the authenticated user's own account metadata (`GET /api/account/me`).
    #[tracing::instrument(skip_all)]
    async fn get_user_info(&self, user: &UserId) -> UserInfoResult {
        match self.user_repository.get_user_info(user).await {
            Ok(Some(info)) => UserInfoResult::Success(info),
            // The user vanished under an existing session: treat as a retryable failure rather
            // than inventing data.
            Ok(None) => {
                tracing::warn!(%user, "User info requested for an unknown or email-less user");
                UserInfoResult::Retry
            }
            Err(()) => UserInfoResult::Retry,
        }
    }
}

impl<UR, SS, MP> MultiUserService<UR, SS, MP>
where
    UR: UserRepository,
    SS: ISessionService,
    MP: MailProvider,
{
    #[tracing::instrument(skip_all)]
    async fn login_with_email_link(&self, email: EmailAddress) -> UserLoginResult {
        let user = match self.user_repository.get_user_by_email(&email).await {
            Ok(Some((user, _))) => user,
            Ok(None) => {
                AuthToken::decoy_hash();
                // Same return as the happy path to not leak email/user existence.
                return UserLoginResult::EmailSent;
            }
            Err(()) => return UserLoginResult::Retry,
        };

        let req = GenerateAuthLinkRequest::new(user, email);
        match self.generate_auth_link(req).await {
            GenerateAuthLinkResult::Success => UserLoginResult::EmailSent,
            GenerateAuthLinkResult::Retry => UserLoginResult::Retry,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn login_with_password(
        &self,
        email: EmailAddress,
        password: Password,
    ) -> UserLoginResult {
        let fetched = self.user_repository.get_user_by_email(&email).await;

        let (user, password_hash) = match fetched {
            Ok(Some(found)) => found,
            Ok(None) => {
                HashedPassword::decoy_verification(&password);
                return UserLoginResult::Invalid;
            }
            Err(()) => return UserLoginResult::Retry,
        };

        let Some(password_hash) = password_hash else {
            HashedPassword::decoy_verification(&password);
            return UserLoginResult::Invalid;
        };

        if !password_hash.verify_password(&password) {
            return UserLoginResult::Invalid;
        }

        match self.session_service.generate_session_token(&user).await {
            Ok(session) => UserLoginResult::Session(session),
            Err(()) => UserLoginResult::Retry,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn generate_auth_link(&self, req: GenerateAuthLinkRequest) -> GenerateAuthLinkResult {
        let auth_token = AuthToken::new();
        let auth_link = AuthLink::new(
            req.user().clone(),
            req.email().clone(),
            auth_token.clone(),
            chrono::Utc::now() + TimeDelta::minutes(15),
        );
        let Some(hashed_auth_link) = auth_link.as_hash() else {
            return GenerateAuthLinkResult::Retry;
        };

        let Ok(()) = self
            .user_repository
            .store_auth_link(&hashed_auth_link)
            .await
        else {
            return GenerateAuthLinkResult::Retry;
        };

        // The email is sent in the background rather than awaited here, so that this endpoint's
        // response time doesn't depend on SMTP round-trip time to avoid information leak by timing
        // attack.
        let repository = self.user_repository.clone();
        let mail_provider = self.mail_provider.clone();
        let email = req.email().clone();
        tokio::spawn(async move {
            if mail_provider
                .send_auth_link_email(&email, &auth_link)
                .await
                .is_err()
            {
                tracing::error!(%email, "Failed to send authentication link email");
                let _ = repository
                    .delete_auth_link_by_hash(hashed_auth_link.hash())
                    .await;
            }
        });

        GenerateAuthLinkResult::Success
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn validate_auth_token(
        &self,
        token: &AuthToken,
    ) -> Result<Option<crate::domain::models::UserId>, ()> {
        let links = self.user_repository.get_all_auth_links().await;

        let mut found = None;

        let now = chrono::Utc::now();
        for link in links {
            if link.is_expired(&now) {
                let _ = self
                    .user_repository
                    .delete_auth_link_by_hash(link.hash())
                    .await;
                continue;
            }
            if link.hash().verify_token(token) {
                found = Some(link);
            }
        }

        let Some(link) = found else {
            return Ok(None);
        };

        match self
            .user_repository
            .consume_auth_link(link.user(), link.hash())
            .await
        {
            Ok(true) => Ok(Some(link.user().clone())),
            Ok(false) => Ok(None),
            Err(()) => Err(()),
        }
    }

    /// Applies an email change synchronously, without verification. The new email is marked
    /// unverified; a later flow could re-verify it.
    #[tracing::instrument(skip_all)]
    async fn change_email_unverified(&self, req: UpdateEmailRequest) -> UpdateEmailResult {
        match self
            .check_email_changeable(req.user(), req.new_email())
            .await
        {
            Ok(true) => {}
            Ok(false) => return UpdateEmailResult::Changed,
            Err(()) => return UpdateEmailResult::Retry,
        }

        match self
            .user_repository
            .update_user_email(req.user(), req.new_email(), false)
            .await
        {
            Ok(()) => UpdateEmailResult::Changed,
            Err(()) => UpdateEmailResult::Retry,
        }
    }

    /// Stores an email change token and emails the verification link to the new address in the
    /// background. The swap itself only happens on `validate_email_change`.
    #[tracing::instrument(skip_all)]
    async fn request_email_change(&self, req: UpdateEmailRequest) -> UpdateEmailResult {
        match self
            .check_email_changeable(req.user(), req.new_email())
            .await
        {
            Ok(true) => {}
            Ok(false) => return UpdateEmailResult::VerificationEmailSent,
            Err(()) => return UpdateEmailResult::Retry,
        }

        let token = EmailChangeToken::new(
            req.user().clone(),
            req.new_email().clone(),
            AuthToken::new(),
            chrono::Utc::now() + TimeDelta::minutes(15),
        );
        let Some(hashed_token) = token.as_hash() else {
            return UpdateEmailResult::Retry;
        };

        if self
            .user_repository
            .store_email_change_token(&hashed_token)
            .await
            .is_err()
        {
            return UpdateEmailResult::Retry;
        }

        let repository = self.user_repository.clone();
        let mail_provider = self.mail_provider.clone();
        tokio::spawn(async move {
            if mail_provider
                .send_email_change_email(token.new_email(), &token)
                .await
                .is_err()
            {
                tracing::error!(
                    email = %token.new_email(),
                    "Failed to send email change verification email"
                );
                let _ = repository
                    .delete_email_change_token_by_hash(hashed_token.hash())
                    .await;
            }
        });

        UpdateEmailResult::VerificationEmailSent
    }

    /// Shared preconditions for both email change flows. Returns
    /// - `Ok(true)` when the change can proceed,
    /// - `Ok(false)` when the caller should answer success without doing anything to not leak,
    /// - `Err(())` on repository failure, which callers must surface as a retryable error.
    #[tracing::instrument(skip_all)]
    async fn check_email_changeable(
        &self,
        user: &UserId,
        new_email: &EmailAddress,
    ) -> Result<bool, ()> {
        match self.user_repository.get_user_email(user).await {
            Ok(Some(current)) if current == *new_email => {
                // Changing to the current email: nothing to do.
                return Ok(false);
            }
            Ok(Some(_)) => {}
            // The user has no email row: nothing we can act on.
            Ok(None) => return Ok(false),
            Err(()) => return Err(()),
        }

        match self.user_repository.get_user_by_email(new_email).await {
            Ok(Some((owner, _))) if owner != *user => {
                // The target email is already registered to another user: answer the same as a
                // success to not leak that account's existence.
                tracing::warn!(%user, new_email = %new_email, "Email change requested for an email already in use");
                return Ok(false);
            }
            Ok(Some(_)) | Ok(None) => {}
            Err(()) => return Err(()),
        }

        Ok(true)
    }
}

pub trait UserRepository: Clone + Send + Sync + 'static {
    fn get_user_by_email(
        &self,
        email: &EmailAddress,
    ) -> impl Future<Output = Result<Option<(UserId, Option<HashedPassword>)>, ()>> + Send;

    fn store_user(
        &self,
        user: &UserId,
        email: &EmailAddress,
        password_hash: Option<HashedPassword>,
    ) -> impl Future<Output = Result<(), ()>> + Send;

    fn get_user_password_hash(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Option<HashedPassword>, ()>> + Send;

    fn count_users_without_password(&self) -> impl Future<Output = Result<u64, ()>> + Send;

    fn set_user_password_if_absent(
        &self,
        user: &UserId,
        password_hash: &HashedPassword,
    ) -> impl Future<Output = Result<bool, ()>> + Send;

    fn replace_user_password_if_matches(
        &self,
        user: &UserId,
        current_hash: &HashedPassword,
        new_hash: &HashedPassword,
    ) -> impl Future<Output = Result<bool, ()>> + Send;

    fn store_auth_link(
        &self,
        link: &HashedAuthLink,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn get_all_auth_links(&self) -> impl Future<Output = Vec<HashedAuthLink>> + Send;

    fn delete_auth_link_by_hash(
        &self,
        hash: &HashedAuthToken,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn consume_auth_link(
        &self,
        user: &UserId,
        hash: &HashedAuthToken,
    ) -> impl Future<Output = Result<bool, ()>> + Send;

    fn delete_expired_auth_links(
        &self,
        reference: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn get_user_email(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Option<EmailAddress>, ()>> + Send;

    fn update_user_email(
        &self,
        user: &UserId,
        new_email: &EmailAddress,
        verified: bool,
    ) -> impl Future<Output = Result<(), ()>> + Send;

    fn store_email_change_token(
        &self,
        token: &HashedEmailChangeToken,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn get_all_email_change_tokens(
        &self,
    ) -> impl Future<Output = Vec<HashedEmailChangeToken>> + Send;

    fn delete_email_change_token_by_hash(
        &self,
        hash: &HashedAuthToken,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn delete_expired_email_change_tokens(
        &self,
        reference: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn finalize_email_change(
        &self,
        user: &UserId,
        hash: &HashedAuthToken,
    ) -> impl Future<Output = Result<EmailChangeFinalizeResult, ()>> + Send;

    /// Returns the user's email, whether it is verified, and whether a password is registered.
    /// `None` when the user is unknown or has no email row.
    fn get_user_info(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Option<UserInfo>, ()>> + Send;
}

#[tracing::instrument(skip_all)]
pub fn spawn_expired_auth_links_cleanup<UR: UserRepository>(
    repository: Arc<UR>,
    interval: std::time::Duration,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(err) = repository
                .delete_expired_auth_links(chrono::Utc::now())
                .await
            {
                tracing::warn!("Failed to clean up expired auth links: {err}");
            }
        }
    });
}

#[tracing::instrument(skip_all)]
pub fn spawn_expired_email_change_tokens_cleanup<UR: UserRepository>(
    repository: Arc<UR>,
    interval: std::time::Duration,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(err) = repository
                .delete_expired_email_change_tokens(chrono::Utc::now())
                .await
            {
                tracing::warn!("Failed to clean up expired email change tokens: {err}");
            }
        }
    });
}

#[cfg(test)]
mod test_utils {
    use mockall::mock;

    use crate::inbound::auth::multi_user::test_utils::MockMailProvider;

    use super::super::test_utils::MockSessionService;
    use super::*;

    mock! {
        pub UserRepository {}

        impl Clone for UserRepository {
            fn clone(&self) -> Self;
        }

        impl UserRepository for UserRepository {
            async fn get_user_by_email(
                &self,
                email: &EmailAddress
            ) -> Result<Option<(UserId, Option<HashedPassword>)>, ()>;

            async fn store_user(
                &self,
                user: &UserId,
                email: &EmailAddress,
                password_hash: Option<HashedPassword>
            ) -> Result<(), ()>;

            async fn get_user_password_hash(
                &self,
                user: &UserId,
            ) -> Result<Option<HashedPassword>, ()>;

            async fn count_users_without_password(&self) -> Result<u64, ()>;

            async fn set_user_password_if_absent(
                &self,
                user: &UserId,
                password_hash: &HashedPassword,
            ) -> Result<bool, ()>;

            async fn replace_user_password_if_matches(
                &self,
                user: &UserId,
                current_hash: &HashedPassword,
                new_hash: &HashedPassword,
            ) -> Result<bool, ()>;

            async fn store_auth_link(
                &self,
                link: &HashedAuthLink,
            ) -> Result<(), anyhow::Error>;

            async fn get_all_auth_links(&self) -> Vec<HashedAuthLink>;

            async fn delete_auth_link_by_hash(
                &self,
                hash: &HashedAuthToken,
            ) -> Result<(), anyhow::Error>;

            async fn consume_auth_link(
                &self,
                user: &UserId,
                hash: &HashedAuthToken,
            ) -> Result<bool, ()>;

            async fn delete_expired_auth_links(
                &self,
                reference: chrono::DateTime<chrono::Utc>,
            ) -> Result<(), anyhow::Error>;

            async fn get_user_email(&self, user: &UserId) -> Result<Option<EmailAddress>, ()>;

            async fn update_user_email(
                &self,
                user: &UserId,
                new_email: &EmailAddress,
                verified: bool,
            ) -> Result<(), ()>;

            async fn store_email_change_token(
                &self,
                token: &HashedEmailChangeToken,
            ) -> Result<(), anyhow::Error>;

            async fn get_all_email_change_tokens(&self) -> Vec<HashedEmailChangeToken>;

            async fn delete_email_change_token_by_hash(
                &self,
                hash: &HashedAuthToken,
            ) -> Result<(), anyhow::Error>;

            async fn delete_expired_email_change_tokens(
                &self,
                reference: chrono::DateTime<chrono::Utc>,
            ) -> Result<(), anyhow::Error>;

            async fn finalize_email_change(
                &self,
                user: &UserId,
                hash: &HashedAuthToken,
            ) -> Result<EmailChangeFinalizeResult, ()>;

            async fn get_user_info(&self, user: &UserId) -> Result<Option<UserInfo>, ()>;
        }
    }

    pub const TEST_EMAIL: &str = "test@mail.test";

    pub fn mail_and_password_service(
        user: MockUserRepository,
        mail: MockMailProvider,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        mail_and_password_service_with_session(user, MockSessionService::new(), mail)
    }

    pub fn mail_and_password_service_with_session(
        user: MockUserRepository,
        session: MockSessionService,
        mail: MockMailProvider,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        MultiUserService::new(
            Arc::new(user),
            session,
            AuthMethods::new(true),
            Arc::new(mail),
        )
    }

    pub fn password_only_service(
        user: MockUserRepository,
        mail: MockMailProvider,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        password_only_service_with_session(user, MockSessionService::new(), mail)
    }

    pub fn password_only_service_with_session(
        user: MockUserRepository,
        session: MockSessionService,
        mail: MockMailProvider,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        MultiUserService::new(
            Arc::new(user),
            session,
            AuthMethods::new(false),
            Arc::new(mail),
        )
    }

    pub fn registration_request(password: Option<&str>) -> UserRegistrationRequest {
        UserRegistrationRequest::new(
            EmailAddress::try_from(TEST_EMAIL).unwrap(),
            password.map(|p| Password::try_from(p.to_string()).unwrap()),
        )
    }
}

#[cfg(test)]
mod test_user_service_register_new_user {
    use anyhow::anyhow;

    use crate::{
        domain::models::UserId,
        inbound::auth::multi_user::{
            UserRegistrationResult,
            service::test_utils::{
                MockUserRepository, TEST_EMAIL, mail_and_password_service, password_only_service,
                registration_request,
            },
            test_utils::MockMailProvider,
        },
    };

    use super::*;

    fn email() -> EmailAddress {
        EmailAddress::try_from(TEST_EMAIL).unwrap()
    }

    #[tokio::test]
    async fn test_register_user_ok_path() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .withf(|mail, _| mail == &email())
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, addr, password_hash| addr == &email() && password_hash.is_some())
            .returning(|_, _, _| Ok(()));
        user.expect_store_auth_link().times(1).returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);

        // Give the spawned background email task a chance to run before the mocks (and
        // their `.times(1)` expectations) are checked on drop at the end of the test.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_register_user_store_auth_link_err() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .returning(|_, _, _| Ok(()));
        user.expect_store_auth_link()
            .returning(|_| Err(anyhow!("error")));
        let service = mail_and_password_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }

    #[tokio::test]
    async fn test_register_user_store_user_fails() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .returning(|_, _, _| Err(()));
        user.expect_store_auth_link().times(0);

        let service = mail_and_password_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }

    #[tokio::test]
    async fn test_register_user_user_already_exists_send_auth_link() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        user.expect_store_user().times(0);
        user.expect_store_auth_link().times(1).returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_cannot_check_if_user_already_exists() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Err(()));
        user.expect_store_user().times(0);
        user.expect_store_auth_link().times(0);

        let service = mail_and_password_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_and_password_stores_password_hash() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, _email, password_hash| password_hash.is_some())
            .returning(|_, _, _| Ok(()));
        user.expect_store_auth_link().times(1).returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_mail_and_password_without_password_requires_password() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().times(0);
        user.expect_store_user().times(0);
        user.expect_store_auth_link().times(0);

        let service = mail_and_password_service(user, mailer);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::PasswordRequired);
    }

    #[tokio::test]
    async fn test_password_only_creates_user_without_validating_email() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, addr, password_hash| addr == &email() && password_hash.is_some())
            .returning(|_, _, _| Ok(()));
        user.expect_store_auth_link().times(0);

        let service = password_only_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_password_only_requires_password() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().times(0);
        user.expect_store_user().times(0);
        user.expect_store_auth_link().times(0);

        let service = password_only_service(user, mailer);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::PasswordRequired);
    }

    #[tokio::test]
    async fn test_password_only_with_existing_user_never_overwrites_password() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        user.expect_store_user().times(0);
        user.expect_store_auth_link().times(0);

        let service = password_only_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_mail_only_with_existing_user_never_stores_password() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        user.expect_store_user().times(0);
        user.expect_store_auth_link().times(1).returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_register_user_deletes_auth_link_if_background_email_fails() {
        // The result is still Success: the email failure is only handled in the background.
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .returning(|_, _| Err(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .returning(|_, _, _| Ok(()));
        user.expect_store_auth_link().times(1).returning(|_| Ok(()));
        user.expect_delete_auth_link_by_hash()
            .times(1)
            .returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

#[cfg(test)]
mod test_user_service_login_user {
    use anyhow::anyhow;
    use chrono::{TimeDelta, Utc};

    use crate::{
        domain::models::UserId,
        inbound::auth::multi_user::{
            GenerateSessionTokenResult, HashedPassword, Password, SessionToken,
            service::test_utils::{
                MockUserRepository, TEST_EMAIL, mail_and_password_service,
                mail_and_password_service_with_session, password_only_service_with_session,
            },
            test_utils::{MockMailProvider, MockSessionService},
        },
    };

    use super::*;

    fn email() -> EmailAddress {
        EmailAddress::try_from(TEST_EMAIL).unwrap()
    }

    fn password(value: &str) -> Password {
        Password::try_from(value.to_string()).unwrap()
    }

    fn password_hash(value: &str) -> HashedPassword {
        password(value).as_hash().expect("hashing should succeed")
    }

    #[tokio::test]
    async fn test_mail_only_unknown_email_answers_email_sent() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_auth_link().times(0);

        let service = mail_and_password_service(user, mailer);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::EmailSent);
    }

    #[tokio::test]
    async fn test_mail_only_repo_error_returns_retry() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Err(()));
        user.expect_store_auth_link().times(0);

        let service = mail_and_password_service(user, mailer);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_only_store_auth_link_err_returns_retry() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        user.expect_store_auth_link()
            .returning(|_| Err(anyhow!("error")));

        let service = mail_and_password_service(user, mailer);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_only_found_user_generates_auth_link() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .withf(|mail, auth_link| {
                mail == &email() && auth_link.user() == &UserId::test_default()
            })
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        user.expect_store_auth_link()
            .times(1)
            .withf(|link| link.user() == &UserId::test_default())
            .returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::EmailSent);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_mail_and_password_repo_error_returns_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Err(()));
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = mail_and_password_service_with_session(user, session, mailer);

        let res = service.login_user(email(), Some(password("secret"))).await;

        assert_eq!(res, UserLoginResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_and_password_correct_password_creates_session() {
        let hash = password_hash("secret");
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(move |_| Ok(Some((UserId::test_default(), Some(hash.clone())))));
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);

        let expected_token = SessionToken::from("session-token");
        let token = expected_token.clone();
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(move |_| {
                Ok(GenerateSessionTokenResult::new(
                    token.clone(),
                    Utc::now() + TimeDelta::days(30),
                ))
            });

        let service = mail_and_password_service_with_session(user, session, mailer);

        let res = service.login_user(email(), Some(password("secret"))).await;

        let UserLoginResult::Session(session) = res else {
            panic!("expected a session, got {res:?}");
        };
        assert_eq!(session.token(), &expected_token);
    }

    #[tokio::test]
    async fn test_mail_and_password_wrong_password_is_invalid() {
        let hash = password_hash("secret");
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(move |_| Ok(Some((UserId::test_default(), Some(hash.clone())))));
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = mail_and_password_service_with_session(user, session, mailer);

        let res = service.login_user(email(), Some(password("wrong"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_mail_and_password_user_without_registered_password_is_invalid() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = mail_and_password_service_with_session(user, session, mailer);

        let res = service.login_user(email(), Some(password("secret"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_mail_and_password_unknown_email_is_invalid() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = mail_and_password_service_with_session(user, session, mailer);

        let res = service.login_user(email(), Some(password("secret"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_mail_and_password_session_creation_failure_returns_retry() {
        let hash = password_hash("secret");
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(move |_| Ok(Some((UserId::test_default(), Some(hash.clone())))));
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .returning(|_| Err(()));

        let service = mail_and_password_service_with_session(user, session, mailer);

        let res = service.login_user(email(), Some(password("secret"))).await;

        assert_eq!(res, UserLoginResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_and_password_without_password_falls_back_to_mail() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        user.expect_store_auth_link()
            .times(1)
            .withf(|link| link.user() == &UserId::test_default())
            .returning(|_| Ok(()));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = mail_and_password_service_with_session(user, session, mailer);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::EmailSent);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_password_only_correct_password_creates_session() {
        let hash = password_hash("secret");
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(move |_| Ok(Some((UserId::test_default(), Some(hash.clone())))));
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);

        let expected_token = SessionToken::from("session-token");
        let token = expected_token.clone();
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(move |_| {
                Ok(GenerateSessionTokenResult::new(
                    token.clone(),
                    Utc::now() + TimeDelta::days(30),
                ))
            });

        let service = password_only_service_with_session(user, session, mailer);

        let res = service.login_user(email(), Some(password("secret"))).await;

        let UserLoginResult::Session(session) = res else {
            panic!("expected a session, got {res:?}");
        };
        assert_eq!(session.token(), &expected_token);
    }

    #[tokio::test]
    async fn test_password_only_wrong_password_is_invalid() {
        let hash = password_hash("secret");
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(move |_| Ok(Some((UserId::test_default(), Some(hash.clone())))));
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, mailer);

        let res = service.login_user(email(), Some(password("wrong"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_password_only_missing_password_is_invalid() {
        let mut user = MockUserRepository::new();
        // Password-only mode must not even look the user up without a password.
        user.expect_get_user_by_email().times(0);
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, mailer);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_password_only_unknown_email_is_invalid() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, mailer);

        let res = service.login_user(email(), Some(password("secret"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }
}

#[cfg(test)]
mod test_user_service_password_management {
    use chrono::{TimeDelta, Utc};

    use crate::inbound::auth::multi_user::{
        GenerateSessionTokenResult, HashedPassword, Password, RegisterPasswordRequest,
        RegisterPasswordResult, SessionToken, UpdatePasswordRequest, UpdatePasswordResult,
        service::test_utils::{
            MockUserRepository, mail_and_password_service_with_session,
            password_only_service_with_session,
        },
        test_utils::{MockMailProvider, MockSessionService},
    };

    use super::*;

    fn password(value: &str) -> Password {
        Password::try_from(value.to_string()).unwrap()
    }

    fn password_hash(value: &str) -> HashedPassword {
        password(value).as_hash().expect("hashing should succeed")
    }

    fn register_request(new_password: &str) -> RegisterPasswordRequest {
        RegisterPasswordRequest::new(UserId::test_default(), password(new_password))
    }

    fn update_request(current: &str, new: &str) -> UpdatePasswordRequest {
        UpdatePasswordRequest::new(UserId::test_default(), password(current), password(new))
    }

    fn successful_session() -> GenerateSessionTokenResult {
        GenerateSessionTokenResult::new(
            SessionToken::from("new-session-token"),
            Utc::now() + TimeDelta::days(30),
        )
    }

    #[tokio::test]
    async fn test_register_password_success_stores_hash_and_creates_session() {
        let mut user = MockUserRepository::new();
        user.expect_set_user_password_if_absent()
            .times(1)
            .withf(move |user, hash| {
                user == &UserId::test_default() && hash.verify_password(&password("new-secret"))
            })
            .returning(|_, _| Ok(true));
        let expected_session = successful_session();
        let expected_token = expected_session.token().clone();
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(move |_| Ok(expected_session.clone()));

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .register_password(register_request("new-secret"))
            .await;

        let RegisterPasswordResult::Success(session) = res else {
            panic!("expected success, got {res:?}");
        };
        assert_eq!(session.token(), &expected_token);
    }

    #[tokio::test]
    async fn test_register_password_fails_when_password_already_set() {
        let mut user = MockUserRepository::new();
        user.expect_set_user_password_if_absent()
            .times(1)
            .returning(|_, _| Ok(false));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .register_password(register_request("new-secret"))
            .await;

        assert_eq!(res, RegisterPasswordResult::PasswordAlreadySet);
    }

    #[tokio::test]
    async fn test_register_password_repo_error_returns_retry() {
        let mut user = MockUserRepository::new();
        user.expect_set_user_password_if_absent()
            .times(1)
            .returning(|_, _| Err(()));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .register_password(register_request("new-secret"))
            .await;

        assert_eq!(res, RegisterPasswordResult::Retry);
    }

    #[tokio::test]
    async fn test_register_password_session_error_returns_retry() {
        let mut user = MockUserRepository::new();
        user.expect_set_user_password_if_absent()
            .times(1)
            .returning(|_, _| Ok(true));
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .returning(|_| Err(()));

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .register_password(register_request("new-secret"))
            .await;

        assert_eq!(res, RegisterPasswordResult::Retry);
    }

    #[tokio::test]
    async fn test_register_password_works_in_mail_and_password_mode() {
        let mut user = MockUserRepository::new();
        user.expect_set_user_password_if_absent()
            .times(1)
            .withf(move |_, hash| hash.verify_password(&password("new-secret")))
            .returning(|_, _| Ok(true));
        let expected_session = successful_session();
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .returning(move |_| Ok(expected_session.clone()));

        let service =
            mail_and_password_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .register_password(register_request("new-secret"))
            .await;

        assert!(matches!(res, RegisterPasswordResult::Success(_)));
    }

    #[tokio::test]
    async fn test_update_password_success_replaces_hash_and_creates_session() {
        let current_hash = password_hash("current");
        let mut user = MockUserRepository::new();
        user.expect_get_user_password_hash()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(move |_| Ok(Some(current_hash.clone())));
        user.expect_replace_user_password_if_matches()
            .times(1)
            .withf(move |user, current, new| {
                user == &UserId::test_default()
                    && current.verify_password(&password("current"))
                    && new.verify_password(&password("new-secret"))
            })
            .returning(|_, _, _| Ok(true));
        let expected_session = successful_session();
        let expected_token = expected_session.token().clone();
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(move |_| Ok(expected_session.clone()));

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .update_password(update_request("current", "new-secret"))
            .await;

        let UpdatePasswordResult::Success(session) = res else {
            panic!("expected success, got {res:?}");
        };
        assert_eq!(session.token(), &expected_token);
    }

    #[tokio::test]
    async fn test_update_password_wrong_current_password_is_invalid() {
        let current_hash = password_hash("current");
        let mut user = MockUserRepository::new();
        user.expect_get_user_password_hash()
            .returning(move |_| Ok(Some(current_hash.clone())));
        user.expect_replace_user_password_if_matches().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .update_password(update_request("wrong", "new-secret"))
            .await;

        assert_eq!(res, UpdatePasswordResult::InvalidCurrentPassword);
    }

    #[tokio::test]
    async fn test_update_password_without_registered_password_is_invalid() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_password_hash()
            .times(1)
            .returning(|_| Ok(None));
        user.expect_replace_user_password_if_matches().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .update_password(update_request("current", "new-secret"))
            .await;

        assert_eq!(res, UpdatePasswordResult::InvalidCurrentPassword);
    }

    #[tokio::test]
    async fn test_update_password_concurrent_modification_returns_invalid() {
        let current_hash = password_hash("current");
        let mut user = MockUserRepository::new();
        user.expect_get_user_password_hash()
            .returning(move |_| Ok(Some(current_hash.clone())));
        // Another request replaced the password between the lookup and the CAS update.
        user.expect_replace_user_password_if_matches()
            .times(1)
            .returning(|_, _, _| Ok(false));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .update_password(update_request("current", "new-secret"))
            .await;

        assert_eq!(res, UpdatePasswordResult::InvalidCurrentPassword);
    }

    #[tokio::test]
    async fn test_update_password_repo_error_returns_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_password_hash().returning(|_| Err(()));
        user.expect_replace_user_password_if_matches().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .update_password(update_request("current", "new-secret"))
            .await;

        assert_eq!(res, UpdatePasswordResult::Retry);
    }

    #[tokio::test]
    async fn test_update_password_store_error_returns_retry() {
        let current_hash = password_hash("current");
        let mut user = MockUserRepository::new();
        user.expect_get_user_password_hash()
            .returning(move |_| Ok(Some(current_hash.clone())));
        user.expect_replace_user_password_if_matches()
            .times(1)
            .returning(|_, _, _| Err(()));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .update_password(update_request("current", "new-secret"))
            .await;

        assert_eq!(res, UpdatePasswordResult::Retry);
    }

    #[tokio::test]
    async fn test_update_password_session_error_returns_retry() {
        let current_hash = password_hash("current");
        let mut user = MockUserRepository::new();
        user.expect_get_user_password_hash()
            .returning(move |_| Ok(Some(current_hash.clone())));
        user.expect_replace_user_password_if_matches()
            .times(1)
            .returning(|_, _, _| Ok(true));
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .returning(|_| Err(()));

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .update_password(update_request("current", "new-secret"))
            .await;

        assert_eq!(res, UpdatePasswordResult::Retry);
    }

    #[tokio::test]
    async fn test_update_password_works_in_mail_and_password_mode() {
        let current_hash = password_hash("current");
        let mut user = MockUserRepository::new();
        user.expect_get_user_password_hash()
            .returning(move |_| Ok(Some(current_hash.clone())));
        user.expect_replace_user_password_if_matches()
            .times(1)
            .returning(|_, _, _| Ok(true));
        let expected_session = successful_session();
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .returning(move |_| Ok(expected_session.clone()));

        let service =
            mail_and_password_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .update_password(update_request("current", "new-secret"))
            .await;

        assert!(matches!(res, UpdatePasswordResult::Success(_)));
    }

    #[tokio::test]
    async fn test_update_password_unknown_user_is_invalid_without_telling_which_field_failed() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_password_hash().returning(|_| Err(()));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(user, session, MockMailProvider::new());

        let res = service
            .update_password(update_request("current", "new-secret"))
            .await;

        assert_eq!(res, UpdatePasswordResult::Retry);
    }
}

#[derive(Debug, Clone)]
pub struct DisabledUserService {}

impl IMultiUserService for DisabledUserService {
    async fn check_session_token(&self, _token: &SessionToken) -> Result<CheckSessionResult, ()> {
        panic!("User service is disabled")
    }

    async fn login_user(
        &self,
        _email: EmailAddress,
        _password: Option<Password>,
    ) -> crate::inbound::auth::multi_user::UserLoginResult {
        panic!("User service is disabled")
    }

    async fn register_user(&self, _req: UserRegistrationRequest) -> UserRegistrationResult {
        panic!("User service is disabled")
    }

    async fn validate_auth_link(
        &self,
        _token: AuthToken,
    ) -> Result<crate::inbound::auth::multi_user::AuthLinkValidationResult, ()> {
        panic!("User service is disabled")
    }

    async fn logout(&self, _token: &SessionToken) -> Result<(), ()> {
        panic!("User service is disabled")
    }

    async fn revoke_other_sessions(
        &self,
        _user: &UserId,
        _current: &HashedSessionToken,
    ) -> crate::inbound::auth::multi_user::RevokeSessionsResult {
        panic!("User service is disabled")
    }

    async fn revoke_all_sessions(
        &self,
        _user: &UserId,
    ) -> crate::inbound::auth::multi_user::RevokeSessionsResult {
        panic!("User service is disabled")
    }

    async fn register_password(&self, _req: RegisterPasswordRequest) -> RegisterPasswordResult {
        panic!("User service is disabled")
    }

    async fn update_password(&self, _req: UpdatePasswordRequest) -> UpdatePasswordResult {
        panic!("User service is disabled")
    }

    async fn update_email(&self, _req: UpdateEmailRequest) -> UpdateEmailResult {
        panic!("User service is disabled")
    }

    async fn request_email_verification(
        &self,
        _user: &UserId,
    ) -> crate::inbound::auth::multi_user::EmailVerificationResult {
        panic!("User service is disabled")
    }

    async fn validate_email_change(
        &self,
        _token: AuthToken,
    ) -> Result<EmailChangeValidationResult, ()> {
        panic!("User service is disabled")
    }

    async fn get_user_info(&self, _user: &UserId) -> UserInfoResult {
        panic!("User service is disabled")
    }
}

#[cfg(test)]
mod test_user_service_validate_auth_link {
    use chrono::{TimeDelta, Utc};

    use crate::inbound::auth::multi_user::{
        AuthLinkValidationResult, AuthToken, GenerateSessionTokenResult, HashedAuthLink,
        SessionToken,
        service::test_utils::{MockUserRepository, mail_and_password_service_with_session},
        test_utils::{MockMailProvider, MockSessionService},
    };

    use super::*;

    const RAW_AUTH_TOKEN: &str = "a very long and secure auth token";

    fn auth_token() -> AuthToken {
        AuthToken::from(RAW_AUTH_TOKEN.to_string())
    }

    fn bound_email() -> EmailAddress {
        EmailAddress::try_from("test@mail.test").unwrap()
    }

    /// A stored auth link whose hash matches [`auth_token`].
    fn stored_auth_link(expire_at: chrono::DateTime<Utc>) -> HashedAuthLink {
        HashedAuthLink::new(
            UserId::test_default(),
            Some(bound_email()),
            auth_token().as_hash().expect("hashing should succeed"),
            expire_at,
        )
    }

    fn new_service(
        user: MockUserRepository,
        session: MockSessionService,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        mail_and_password_service_with_session(user, session, MockMailProvider::new())
    }

    #[tokio::test]
    async fn test_ok_path() {
        let mut user = MockUserRepository::new();
        let link = stored_auth_link(Utc::now() + TimeDelta::minutes(5));
        let link_clone = link.clone();
        user.expect_get_all_auth_links()
            .returning(move || vec![link_clone.clone()]);
        let token = auth_token();
        user.expect_consume_auth_link()
            .times(1)
            .withf(move |user, hash| user == &UserId::test_default() && hash.verify_token(&token))
            .returning(|_, _| Ok(true));
        let mut session = MockSessionService::new();
        let session_token = SessionToken::new();
        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let cloned_session_token = session_token.clone();
        let cloned_expire_at = expire_at;
        session
            .expect_generate_session_token()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(move |_| {
                Ok(GenerateSessionTokenResult::new(
                    cloned_session_token.clone(),
                    cloned_expire_at,
                ))
            });

        let service = new_service(user, session);

        let res = service.validate_auth_link(auth_token()).await;

        let Ok(AuthLinkValidationResult::Success(session)) = res else {
            unreachable!("Should have return a Ok(AuthLinkValidationResult::Success(_))")
        };
        assert_eq!(session_token.as_hash(), session.token().as_hash());
        assert_eq!(*session.expire_at(), expire_at);
    }

    #[tokio::test]
    async fn test_auth_token_rejected() {
        let mut user = MockUserRepository::new();
        user.expect_get_all_auth_links().returning(Vec::new);
        user.expect_delete_auth_link_by_hash().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = new_service(user, session);

        let res = service.validate_auth_link(auth_token()).await;

        let Ok(AuthLinkValidationResult::Invalid) = res else {
            unreachable!("Should have return a Ok(AuthLinkValidationResult::Invalid)")
        };
    }

    #[tokio::test]
    async fn test_matching_link_found_but_expired() {
        let mut user = MockUserRepository::new();
        let link = stored_auth_link(Utc::now() - TimeDelta::minutes(5));
        let link_clone = link.clone();
        user.expect_get_all_auth_links()
            .returning(move || vec![link_clone.clone()]);
        let token = auth_token();
        user.expect_delete_auth_link_by_hash()
            .times(1)
            .withf(move |hash| hash.verify_token(&token))
            .returning(|_| Ok(()));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = new_service(user, session);

        let res = service.validate_auth_link(auth_token()).await;

        let Ok(AuthLinkValidationResult::Invalid) = res else {
            unreachable!("Should have return a Ok(AuthLinkValidationResult::Invalid)")
        };
    }

    #[tokio::test]
    async fn test_only_the_matching_link_is_deleted() {
        let mut user = MockUserRepository::new();
        let matching = stored_auth_link(Utc::now() + TimeDelta::minutes(5));
        let other_token = AuthToken::new();
        let other = HashedAuthLink::new(
            UserId::test_default(),
            Some(bound_email()),
            other_token.as_hash().expect("hashing should succeed"),
            Utc::now() + TimeDelta::minutes(5),
        );
        let matching_clone = matching.clone();
        user.expect_get_all_auth_links()
            .returning(move || vec![matching_clone.clone(), other.clone()]);
        let token = auth_token();
        // Only the link matching the provided token must be consumed; the other one stays.
        user.expect_consume_auth_link()
            .times(1)
            .withf(move |user, hash| user == &UserId::test_default() && hash.verify_token(&token))
            .returning(|_, _| Ok(true));
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .returning(|_| {
                Ok(GenerateSessionTokenResult::new(
                    SessionToken::new(),
                    Utc::now() + TimeDelta::minutes(5),
                ))
            });

        let service = new_service(user, session);

        let res = service.validate_auth_link(auth_token()).await;

        assert!(matches!(res, Ok(AuthLinkValidationResult::Success(_))));
    }

    #[tokio::test]
    async fn test_cannot_generate_session_token() {
        let mut user = MockUserRepository::new();
        let link = stored_auth_link(Utc::now() + TimeDelta::minutes(5));
        let link_clone = link.clone();
        user.expect_get_all_auth_links()
            .returning(move || vec![link_clone.clone()]);
        user.expect_consume_auth_link().returning(|_, _| Ok(true));
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .returning(|_| Err(()));

        let service = new_service(user, session);

        let res = service.validate_auth_link(auth_token()).await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_link_consumed_concurrently_is_invalid() {
        let mut user = MockUserRepository::new();
        let link = stored_auth_link(Utc::now() + TimeDelta::minutes(5));
        let link_clone = link.clone();
        user.expect_get_all_auth_links()
            .returning(move || vec![link_clone.clone()]);
        // Another request consumed the same link first.
        user.expect_consume_auth_link().returning(|_, _| Ok(false));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = new_service(user, session);

        let res = service.validate_auth_link(auth_token()).await;

        let Ok(AuthLinkValidationResult::Invalid) = res else {
            unreachable!("Should have return a Ok(AuthLinkValidationResult::Invalid)")
        };
    }

    #[tokio::test]
    async fn test_consume_auth_link_error_is_error() {
        let mut user = MockUserRepository::new();
        let link = stored_auth_link(Utc::now() + TimeDelta::minutes(5));
        let link_clone = link.clone();
        user.expect_get_all_auth_links()
            .returning(move || vec![link_clone.clone()]);
        user.expect_consume_auth_link().returning(|_, _| Err(()));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = new_service(user, session);

        let res = service.validate_auth_link(auth_token()).await;

        assert!(res.is_err())
    }
}

#[cfg(test)]
mod test_user_service_check_session_token {
    use crate::inbound::auth::multi_user::{
        SessionToken,
        service::test_utils::{MockUserRepository, mail_and_password_service_with_session},
        test_utils::{MockMailProvider, MockSessionService},
    };

    use super::*;

    fn new_service(
        session: MockSessionService,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        mail_and_password_service_with_session(
            MockUserRepository::new(),
            session,
            MockMailProvider::new(),
        )
    }

    #[tokio::test]
    async fn test_token_valid() {
        let mut session = MockSessionService::new();
        session
            .expect_check_session_token()
            .returning(|_| Ok(CheckSessionResult::new(UserId::test_default(), None)));

        let service = new_service(session);

        let res = service
            .check_session_token(&SessionToken(
                "a very long and secure session token".to_string(),
            ))
            .await;

        assert_eq!(*res.unwrap().user(), UserId::test_default());
    }

    #[tokio::test]
    async fn test_token_invalid_or_does_not_exist_or_else() {
        let mut session = MockSessionService::new();
        session.expect_check_session_token().returning(|_| Err(()));

        let service = new_service(session);

        let res = service
            .check_session_token(&SessionToken(
                "a very long and secure session token".to_string(),
            ))
            .await;

        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_logout_delegates_to_session_service() {
        let mut session = MockSessionService::new();
        session
            .expect_logout()
            .times(1)
            .withf(|token| token == &SessionToken::from("a very long and secure session token"))
            .returning(|_| Ok(()));

        let service = new_service(session);

        let res = service
            .logout(&SessionToken::from("a very long and secure session token"))
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_logout_error_is_propagated() {
        let mut session = MockSessionService::new();
        session.expect_logout().returning(|_| Err(()));

        let service = new_service(session);

        let res = service
            .logout(&SessionToken::from("a very long and secure session token"))
            .await;

        assert!(res.is_err());
    }
}

#[cfg(test)]
mod test_spawn_expired_auth_links_cleanup {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use crate::inbound::auth::multi_user::service::test_utils::MockUserRepository;

    use super::*;

    #[tokio::test]
    async fn test_cleanup_deletes_expired_auth_links_periodically() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let mut user = MockUserRepository::new();
        user.expect_delete_expired_auth_links()
            .times(1..)
            .withf(|reference| reference <= &chrono::Utc::now())
            .returning(move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            });

        let repository = Arc::new(user);
        spawn_expired_auth_links_cleanup(repository, std::time::Duration::from_millis(1));

        // The ticker's first tick fires immediately; give the task a moment to run.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        assert!(calls.load(Ordering::SeqCst) >= 1);
    }
}
#[cfg(test)]
mod test_user_service_update_email {
    use anyhow::anyhow;
    use chrono::{TimeDelta, Utc};

    use crate::inbound::auth::multi_user::{
        EmailAddress, EmailChangeFinalizeResult, EmailChangeToken, EmailChangeValidationResult,
        HashedEmailChangeToken,
        service::test_utils::{
            MockUserRepository, TEST_EMAIL, mail_and_password_service,
            mail_and_password_service_with_session, password_only_service, registration_request,
        },
        test_utils::{MockMailProvider, MockSessionService},
    };

    use super::*;

    fn email(value: &str) -> EmailAddress {
        EmailAddress::try_from(value).unwrap()
    }

    fn current_email() -> EmailAddress {
        email(TEST_EMAIL)
    }

    fn new_email() -> EmailAddress {
        email("new@mail.test")
    }

    fn update_request(new_email: &str) -> UpdateEmailRequest {
        UpdateEmailRequest::new(UserId::test_default(), email(new_email))
    }

    /// A stored token whose raw token value is [`RAW_TOKEN`] and whose target is [`new_email`].
    fn stored_change_token(expire_at: chrono::DateTime<Utc>) -> HashedEmailChangeToken {
        HashedEmailChangeToken::new(
            UserId::test_default(),
            new_email(),
            raw_token().as_hash().expect("hashing should succeed"),
            expire_at,
        )
    }

    fn raw_token() -> AuthToken {
        AuthToken::from("a very long and secure email change token".to_string())
    }

    /// Common expectations for a changeable email: the user has the current email and the target
    /// is free.
    fn expect_changeable(user: &mut MockUserRepository) {
        user.expect_get_user_email()
            .returning(|_| Ok(Some(current_email())));
        user.expect_get_user_by_email().returning(|email| {
            if email == &current_email() {
                Ok(Some((UserId::test_default(), None)))
            } else {
                Ok(None)
            }
        });
    }

    #[tokio::test]
    async fn test_mail_mode_stores_token_and_sends_verification_email() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_email_change_email()
            .times(1)
            .withf(|mail, token| mail == &new_email() && token.user() == &UserId::test_default())
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        expect_changeable(&mut user);
        user.expect_store_email_change_token()
            .times(1)
            .withf(|token| {
                token.user() == &UserId::test_default() && token.new_email() == &new_email()
            })
            .returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service.update_email(update_request("new@mail.test")).await;

        assert_eq!(res, UpdateEmailResult::VerificationEmailSent);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_mail_mode_silent_success_when_email_already_in_use() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_email_change_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_email()
            .returning(|_| Ok(Some(current_email())));
        // The target email belongs to another user.
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::from("other".to_string()), None))));
        user.expect_store_email_change_token().times(0);

        let service = mail_and_password_service(user, mailer);

        let res = service.update_email(update_request("new@mail.test")).await;

        // Same answer as the happy path, to not leak the other account's existence.
        assert_eq!(res, UpdateEmailResult::VerificationEmailSent);
    }

    #[tokio::test]
    async fn test_mail_mode_unchanged_email_is_a_no_op() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_email_change_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_email()
            .returning(|_| Ok(Some(current_email())));
        user.expect_get_user_by_email().times(0);
        user.expect_store_email_change_token().times(0);

        let service = mail_and_password_service(user, mailer);

        let res = service.update_email(update_request(TEST_EMAIL)).await;

        assert_eq!(res, UpdateEmailResult::VerificationEmailSent);
    }

    #[tokio::test]
    async fn test_mail_mode_replaces_previous_pending_token() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_email_change_email()
            .times(1)
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        expect_changeable(&mut user);
        user.expect_store_email_change_token()
            .times(1)
            .returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service.update_email(update_request("new@mail.test")).await;

        assert_eq!(res, UpdateEmailResult::VerificationEmailSent);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_mail_mode_store_token_failure_returns_retry() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_email_change_email().times(0);
        let mut user = MockUserRepository::new();
        expect_changeable(&mut user);
        user.expect_store_email_change_token()
            .returning(|_| Err(anyhow!("error")));

        let service = mail_and_password_service(user, mailer);

        let res = service.update_email(update_request("new@mail.test")).await;

        assert_eq!(res, UpdateEmailResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_mode_deletes_token_if_background_email_fails() {
        // The result is still VerificationEmailSent: the email failure is only handled in the
        // background.
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_email_change_email()
            .times(1)
            .returning(|_, _| Err(()));
        let mut user = MockUserRepository::new();
        expect_changeable(&mut user);
        user.expect_store_email_change_token().returning(|_| Ok(()));
        user.expect_delete_email_change_token_by_hash()
            .times(1)
            .returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service.update_email(update_request("new@mail.test")).await;

        assert_eq!(res, UpdateEmailResult::VerificationEmailSent);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_mail_mode_repo_error_returns_retry() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_email_change_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_email().returning(|_| Err(()));

        let service = mail_and_password_service(user, mailer);

        let res = service.update_email(update_request("new@mail.test")).await;

        assert_eq!(res, UpdateEmailResult::Retry);
    }

    #[tokio::test]
    async fn test_password_mode_updates_email_synchronously_unverified() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_email_change_email().times(0);
        let mut user = MockUserRepository::new();
        expect_changeable(&mut user);
        user.expect_update_user_email()
            .times(1)
            .withf(|user, target, verified| {
                user == &UserId::test_default() && target == &new_email() && !verified
            })
            .returning(|_, _, _| Ok(()));
        user.expect_store_email_change_token().times(0);

        let service = password_only_service(user, mailer);

        let res = service.update_email(update_request("new@mail.test")).await;

        assert_eq!(res, UpdateEmailResult::Changed);
    }

    #[tokio::test]
    async fn test_password_mode_silent_success_when_email_already_in_use() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_email_change_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_email()
            .returning(|_| Ok(Some(current_email())));
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::from("other".to_string()), None))));
        user.expect_update_user_email().times(0);

        let service = password_only_service(user, mailer);

        let res = service.update_email(update_request("new@mail.test")).await;

        assert_eq!(res, UpdateEmailResult::Changed);
    }

    #[tokio::test]
    async fn test_password_mode_unchanged_email_is_a_no_op() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_email_change_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_email()
            .returning(|_| Ok(Some(current_email())));
        user.expect_get_user_by_email().times(0);
        user.expect_update_user_email().times(0);

        let service = password_only_service(user, mailer);

        let res = service.update_email(update_request(TEST_EMAIL)).await;

        assert_eq!(res, UpdateEmailResult::Changed);
    }

    #[tokio::test]
    async fn test_password_mode_repo_error_returns_retry() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_email_change_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_email()
            .returning(|_| Ok(Some(current_email())));
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_update_user_email().returning(|_, _, _| Err(()));

        let service = password_only_service(user, mailer);

        let res = service.update_email(update_request("new@mail.test")).await;

        assert_eq!(res, UpdateEmailResult::Retry);
    }

    #[tokio::test]
    async fn test_validate_email_change_ok_path() {
        let mut user = MockUserRepository::new();
        let link = stored_change_token(Utc::now() + TimeDelta::minutes(5));
        let link_clone = link.clone();
        user.expect_get_all_email_change_tokens()
            .returning(move || vec![link_clone.clone()]);
        let token = raw_token();
        let session = MockSessionService::new();

        user.expect_finalize_email_change()
            .times(1)
            .withf(move |user, hash| user == &UserId::test_default() && hash.verify_token(&token))
            .returning(|_, _| Ok(EmailChangeFinalizeResult::Swapped(new_email())));

        let service =
            mail_and_password_service_with_session(user, session, MockMailProvider::new());

        let res = service.validate_email_change(raw_token()).await;

        let Ok(EmailChangeValidationResult::Success(swapped)) = res else {
            panic!("expected Success, got {res:?}");
        };
        assert_eq!(swapped, new_email());
    }

    #[tokio::test]
    async fn test_validate_email_change_reports_email_already_in_use() {
        let mut user = MockUserRepository::new();
        let link = stored_change_token(Utc::now() + TimeDelta::minutes(5));
        user.expect_get_all_email_change_tokens()
            .returning(move || vec![link.clone()]);
        // Unlike the request step, the conflict is reported explicitly.
        user.expect_finalize_email_change()
            .times(1)
            .returning(|_, _| Ok(EmailChangeFinalizeResult::EmailAlreadyInUse));

        let service = mail_and_password_service_with_session(
            user,
            MockSessionService::new(),
            MockMailProvider::new(),
        );

        let res = service.validate_email_change(raw_token()).await;

        assert_eq!(res.unwrap(), EmailChangeValidationResult::EmailAlreadyInUse);
    }

    #[tokio::test]
    async fn test_validate_email_change_rejected_for_unknown_token() {
        let mut user = MockUserRepository::new();
        user.expect_get_all_email_change_tokens()
            .returning(Vec::new);
        user.expect_finalize_email_change().times(0);

        let service = mail_and_password_service_with_session(
            user,
            MockSessionService::new(),
            MockMailProvider::new(),
        );

        let res = service.validate_email_change(raw_token()).await;

        assert_eq!(res.unwrap(), EmailChangeValidationResult::Invalid);
    }

    #[tokio::test]
    async fn test_validate_email_change_expired_token_is_deleted_and_rejected() {
        let mut user = MockUserRepository::new();
        let link = stored_change_token(Utc::now() - TimeDelta::minutes(5));
        user.expect_get_all_email_change_tokens()
            .returning(move || vec![link.clone()]);
        user.expect_delete_email_change_token_by_hash()
            .times(1)
            .returning(|_| Ok(()));
        user.expect_finalize_email_change().times(0);

        let service = mail_and_password_service_with_session(
            user,
            MockSessionService::new(),
            MockMailProvider::new(),
        );

        let res = service.validate_email_change(raw_token()).await;

        assert_eq!(res.unwrap(), EmailChangeValidationResult::Invalid);
    }

    #[tokio::test]
    async fn test_validate_email_change_unknown_token_in_finalize_is_invalid() {
        // Covers the concurrent-validation race: another request consumed the token between the
        // scan and the finalization.
        let mut user = MockUserRepository::new();
        let link = stored_change_token(Utc::now() + TimeDelta::minutes(5));
        user.expect_get_all_email_change_tokens()
            .returning(move || vec![link.clone()]);
        user.expect_finalize_email_change()
            .times(1)
            .returning(|_, _| Ok(EmailChangeFinalizeResult::UnknownToken));

        let service = mail_and_password_service_with_session(
            user,
            MockSessionService::new(),
            MockMailProvider::new(),
        );

        let res = service.validate_email_change(raw_token()).await;

        assert_eq!(res.unwrap(), EmailChangeValidationResult::Invalid);
    }

    #[tokio::test]
    async fn test_validate_email_change_repo_error_is_error() {
        let mut user = MockUserRepository::new();
        let link = stored_change_token(Utc::now() + TimeDelta::minutes(5));
        user.expect_get_all_email_change_tokens()
            .returning(move || vec![link.clone()]);
        user.expect_finalize_email_change()
            .times(1)
            .returning(|_, _| Err(()));

        let service = mail_and_password_service_with_session(
            user,
            MockSessionService::new(),
            MockMailProvider::new(),
        );

        let res = service.validate_email_change(raw_token()).await;

        assert!(res.is_err());
    }

    /// Sanity check on the shared preconditions helper: a changeable request passes, and the
    /// no-op cases short-circuit.
    #[tokio::test]
    async fn test_check_email_changeable_returns_false_for_no_ops() {
        let mut user = MockUserRepository::new();
        expect_changeable(&mut user);

        let service = mail_and_password_service(user, MockMailProvider::new());

        // Same email: no-op.
        assert_eq!(
            service
                .check_email_changeable(&UserId::test_default(), &current_email())
                .await,
            Ok(false)
        );

        // Email owned by another user: silent no-op.
        let mut user = MockUserRepository::new();
        user.expect_get_user_email()
            .returning(|_| Ok(Some(current_email())));
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::from("other".to_string()), None))));
        let service = mail_and_password_service(user, MockMailProvider::new());
        assert_eq!(
            service
                .check_email_changeable(&UserId::test_default(), &new_email())
                .await,
            Ok(false)
        );
    }

    /// Sanity check on the shared preconditions helper: repository failures surface as `Err` so
    /// callers return a retryable error instead of a silent success.
    #[tokio::test]
    async fn test_check_email_changeable_returns_err_on_repo_error() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_email().returning(|_| Err(()));
        let service = mail_and_password_service(user, MockMailProvider::new());
        assert_eq!(
            service
                .check_email_changeable(&UserId::test_default(), &new_email())
                .await,
            Err(())
        );

        let mut user = MockUserRepository::new();
        user.expect_get_user_email()
            .returning(|_| Ok(Some(current_email())));
        user.expect_get_user_by_email().returning(|_| Err(()));
        let service = mail_and_password_service(user, MockMailProvider::new());
        assert_eq!(
            service
                .check_email_changeable(&UserId::test_default(), &new_email())
                .await,
            Err(())
        );
    }

    /// Registration still works after the trait extension (compile-time coverage that all mock
    /// impls remain complete).
    #[tokio::test]
    async fn test_registration_still_works() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .returning(|_, _, _| Ok(()));
        user.expect_store_auth_link().times(1).returning(|_| Ok(()));
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .returning(|_, _| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    /// Keep the token type import used: verifies that a freshly built token carries the target
    /// email through its hash.
    #[test]
    fn test_email_change_token_carries_target_email() {
        let token = EmailChangeToken::new(
            UserId::test_default(),
            new_email(),
            AuthToken::new(),
            Utc::now() + TimeDelta::minutes(15),
        );
        let hashed = token.as_hash().unwrap();
        assert_eq!(hashed.new_email(), &new_email());
        assert_eq!(hashed.user(), &UserId::test_default());
    }
}

#[cfg(test)]
mod test_user_service_get_user_info {
    use crate::inbound::auth::multi_user::{
        EmailAddress, UserInfo,
        service::test_utils::{MockUserRepository, TEST_EMAIL, mail_and_password_service},
        test_utils::MockMailProvider,
    };

    use super::*;

    fn email() -> EmailAddress {
        EmailAddress::try_from(TEST_EMAIL).unwrap()
    }

    #[tokio::test]
    async fn test_get_user_info_success() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_info()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(|_| Ok(Some(UserInfo::new(email(), true, false))));

        let service = mail_and_password_service(user, MockMailProvider::new());

        let res = service.get_user_info(&UserId::test_default()).await;

        let UserInfoResult::Success(info) = res else {
            panic!("expected Success, got {res:?}");
        };
        assert_eq!(info.email(), &email());
        assert!(info.verified());
        assert!(!info.has_password());
    }

    #[tokio::test]
    async fn test_get_user_info_unknown_user_is_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_info().returning(|_| Ok(None));

        let service = mail_and_password_service(user, MockMailProvider::new());

        let res = service.get_user_info(&UserId::test_default()).await;

        assert_eq!(res, UserInfoResult::Retry);
    }

    #[tokio::test]
    async fn test_get_user_info_repo_error_is_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_info().returning(|_| Err(()));

        let service = mail_and_password_service(user, MockMailProvider::new());

        let res = service.get_user_info(&UserId::test_default()).await;

        assert_eq!(res, UserInfoResult::Retry);
    }
}

#[cfg(test)]
mod test_user_service_request_email_verification {
    use crate::inbound::auth::multi_user::{
        EmailAddress, EmailVerificationResult, UserInfo,
        service::test_utils::{
            MockUserRepository, TEST_EMAIL, mail_and_password_service, password_only_service,
        },
        test_utils::MockMailProvider,
    };

    use super::*;

    fn email() -> EmailAddress {
        EmailAddress::try_from(TEST_EMAIL).unwrap()
    }

    #[tokio::test]
    async fn test_request_email_verification_disabled_when_email_auth_disabled() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_info().times(0);
        user.expect_store_auth_link().times(0);

        let service = password_only_service(user, MockMailProvider::new());

        let res = service
            .request_email_verification(&UserId::test_default())
            .await;

        assert_eq!(res, EmailVerificationResult::EmailAuthDisabled);
    }

    #[tokio::test]
    async fn test_request_email_verification_already_verified_is_noop() {
        let mut mailer = MockMailProvider::new();
        mailer.expect_send_auth_link_email().times(0);
        let mut user = MockUserRepository::new();
        user.expect_get_user_info()
            .times(1)
            .returning(|_| Ok(Some(UserInfo::new(email(), true, false))));
        user.expect_store_auth_link().times(0);

        let service = mail_and_password_service(user, mailer);

        let res = service
            .request_email_verification(&UserId::test_default())
            .await;

        assert_eq!(res, EmailVerificationResult::AlreadyVerified);
    }

    #[tokio::test]
    async fn test_request_email_verification_sends_auth_link() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .withf(|mail, _| mail == &email())
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_info()
            .times(1)
            .returning(|_| Ok(Some(UserInfo::new(email(), false, false))));
        user.expect_store_auth_link()
            .times(1)
            .withf(|link| link.user() == &UserId::test_default())
            .returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service
            .request_email_verification(&UserId::test_default())
            .await;

        assert_eq!(res, EmailVerificationResult::EmailSent);
        // Let the background email send run before the test ends.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn test_request_email_verification_user_info_error_is_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_info().returning(|_| Err(()));
        user.expect_store_auth_link().times(0);

        let service = mail_and_password_service(user, MockMailProvider::new());

        let res = service
            .request_email_verification(&UserId::test_default())
            .await;

        assert_eq!(res, EmailVerificationResult::Retry);
    }

    #[tokio::test]
    async fn test_request_email_verification_store_auth_link_err_returns_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_info()
            .times(1)
            .returning(|_| Ok(Some(UserInfo::new(email(), false, false))));
        user.expect_store_auth_link()
            .times(1)
            .returning(|_| Err(anyhow::anyhow!("boom")));

        let service = mail_and_password_service(user, MockMailProvider::new());

        let res = service
            .request_email_verification(&UserId::test_default())
            .await;

        assert_eq!(res, EmailVerificationResult::Retry);
    }
}

#[cfg(test)]
mod test_user_service_revoke_sessions {
    use crate::inbound::auth::multi_user::{
        RevokeSessionsResult, SessionToken,
        service::test_utils::{MockUserRepository, mail_and_password_service_with_session},
        test_utils::{MockMailProvider, MockSessionService},
    };

    use super::*;

    #[tokio::test]
    async fn test_revoke_other_sessions_delegates_and_maps_success() {
        let mut session = MockSessionService::new();
        let current = SessionToken::from("current".to_string()).as_hash();
        let expected = current.clone();
        session
            .expect_revoke_other_sessions()
            .times(1)
            .withf(move |user, hash| user == &UserId::test_default() && hash == &expected)
            .returning(|_, _| Ok(()));

        let service = mail_and_password_service_with_session(
            MockUserRepository::new(),
            session,
            MockMailProvider::new(),
        );

        let res = service
            .revoke_other_sessions(&UserId::test_default(), &current)
            .await;

        assert_eq!(res, RevokeSessionsResult::Success);
    }

    #[tokio::test]
    async fn test_revoke_other_sessions_maps_error_to_retry() {
        let mut session = MockSessionService::new();
        session
            .expect_revoke_other_sessions()
            .returning(|_, _| Err(()));

        let service = mail_and_password_service_with_session(
            MockUserRepository::new(),
            session,
            MockMailProvider::new(),
        );

        let res = service
            .revoke_other_sessions(
                &UserId::test_default(),
                &SessionToken::from("current".to_string()).as_hash(),
            )
            .await;

        assert_eq!(res, RevokeSessionsResult::Retry);
    }

    #[tokio::test]
    async fn test_revoke_all_sessions_delegates_and_maps_success() {
        let mut session = MockSessionService::new();
        session
            .expect_revoke_all_sessions()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(|_| Ok(()));

        let service = mail_and_password_service_with_session(
            MockUserRepository::new(),
            session,
            MockMailProvider::new(),
        );

        let res = service.revoke_all_sessions(&UserId::test_default()).await;

        assert_eq!(res, RevokeSessionsResult::Success);
    }

    #[tokio::test]
    async fn test_revoke_all_sessions_maps_error_to_retry() {
        let mut session = MockSessionService::new();
        session.expect_revoke_all_sessions().returning(|_| Err(()));

        let service = mail_and_password_service_with_session(
            MockUserRepository::new(),
            session,
            MockMailProvider::new(),
        );

        let res = service.revoke_all_sessions(&UserId::test_default()).await;

        assert_eq!(res, RevokeSessionsResult::Retry);
    }
}
