use std::sync::Arc;

use chrono::TimeDelta;
use derive_more::Constructor;

use crate::{
    domain::models::UserId,
    inbound::auth::multi_user::{
        AuthLink, AuthLinkValidationResult, AuthMethods, AuthToken, CheckSessionResult,
        EmailAddress, GenerateAuthLinkRequest, GenerateAuthLinkResult, HashedAuthLink,
        HashedAuthToken, HashedPassword, IMultiUserService, ISessionService, MailProvider,
        Password, SessionToken, UserLoginResult, UserRegistrationRequest, UserRegistrationResult,
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
        let password_hash = match (self.auth_methods.password(), req.password()) {
            (true, Some(password)) => match password.as_hash() {
                Some(hash) => Some(hash),
                None => return UserRegistrationResult::Retry,
            },
            // Registering without a password is valid if email-based is active: the
            // user can still authenticate via the authentication link sent by email.
            (true, None) if self.auth_methods.email() => None,
            // Password-only mode: a password is required to create a new account.
            (true, None) => return UserRegistrationResult::PasswordRequired,
            // Password-based auth is disabled: silently ignore any provided password.
            (false, _) => None,
        };

        let email = req.email().clone();
        let user = match self.user_repository.get_user_by_email(&email).await {
            Ok(None) => {
                let user = UserId::new();
                let store_res = self
                    .user_repository
                    .store_user(&user, &email, password_hash)
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
        match (self.auth_methods.password(), password) {
            // Password active and supplied: verify it and issue a session on match.
            (true, Some(password)) => self.login_with_password(email, password).await,
            // Password-only mode without a password: no mail fallback is available.
            (true, None) if !self.auth_methods.email() => UserLoginResult::Invalid,
            // Mail active (with or without password): fall back to the authentication link flow.
            _ => self.login_with_email_link(email).await,
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

        if let Some(link) = &found {
            let _ = self
                .user_repository
                .delete_auth_link_by_hash(link.hash())
                .await;
        }

        Ok(found.map(|link| link.user().clone()))
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

    fn store_auth_link(
        &self,
        link: &HashedAuthLink,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn get_all_auth_links(&self) -> impl Future<Output = Vec<HashedAuthLink>> + Send;

    fn delete_auth_link_by_hash(
        &self,
        hash: &HashedAuthToken,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn delete_expired_auth_links(
        &self,
        reference: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;
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

            async fn store_auth_link(
                &self,
                link: &HashedAuthLink,
            ) -> Result<(), anyhow::Error>;

            async fn get_all_auth_links(&self) -> Vec<HashedAuthLink>;

            async fn delete_auth_link_by_hash(
                &self,
                hash: &HashedAuthToken,
            ) -> Result<(), anyhow::Error>;

            async fn delete_expired_auth_links(
                &self,
                reference: chrono::DateTime<chrono::Utc>,
            ) -> Result<(), anyhow::Error>;
        }
    }

    pub const TEST_EMAIL: &str = "test@mail.test";

    pub fn mail_only_service(
        user: MockUserRepository,

        mail: MockMailProvider,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        mail_only_service_with_session(user, MockSessionService::new(), mail)
    }

    pub fn mail_only_service_with_session(
        user: MockUserRepository,
        session: MockSessionService,
        mail: MockMailProvider,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        MultiUserService::new(
            Arc::new(user),
            session,
            AuthMethods::new(true, false),
            Arc::new(mail),
        )
    }

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
            AuthMethods::new(true, true),
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
            AuthMethods::new(false, true),
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
                MockUserRepository, TEST_EMAIL, mail_and_password_service, mail_only_service,
                password_only_service, registration_request,
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
            .withf(|_user, addr, password_hash| addr == &email() && password_hash.is_none())
            .returning(|_, _, _| Ok(()));
        user.expect_store_auth_link().times(1).returning(|_| Ok(()));

        let service = mail_only_service(user, mailer);

        let res = service.register_user(registration_request(None)).await;

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
        let service = mail_only_service(user, mailer);

        let res = service.register_user(registration_request(None)).await;

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

        let service = mail_only_service(user, mailer);

        let res = service.register_user(registration_request(None)).await;

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

        let service = mail_only_service(user, mailer);

        let res = service.register_user(registration_request(None)).await;

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

        let service = mail_only_service(user, mailer);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_only_ignores_provided_password() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, _email, password_hash| password_hash.is_none())
            .returning(|_, _, _| Ok(()));
        user.expect_store_auth_link().times(1).returning(|_| Ok(()));

        let service = mail_only_service(user, mailer);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
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
    async fn test_mail_and_password_without_password_uses_mail_flow() {
        let mut mailer = MockMailProvider::new();
        mailer
            .expect_send_auth_link_email()
            .times(1)
            .returning(|_, _| Ok(()));
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, _email, password_hash| password_hash.is_none())
            .returning(|_, _, _| Ok(()));
        user.expect_store_auth_link().times(1).returning(|_| Ok(()));

        let service = mail_and_password_service(user, mailer);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::Success);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
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

        let service = mail_only_service(user, mailer);

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

        let service = mail_only_service(user, mailer);

        let res = service.register_user(registration_request(None)).await;

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
                MockUserRepository, TEST_EMAIL, mail_and_password_service_with_session,
                mail_only_service, password_only_service_with_session,
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

        let service = mail_only_service(user, mailer);

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

        let service = mail_only_service(user, mailer);

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

        let service = mail_only_service(user, mailer);

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

        let service = mail_only_service(user, mailer);

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
}

#[cfg(test)]
mod test_user_service_validate_auth_link {
    use chrono::{TimeDelta, Utc};

    use crate::inbound::auth::multi_user::{
        AuthLinkValidationResult, AuthToken, GenerateSessionTokenResult, HashedAuthLink,
        SessionToken,
        service::test_utils::{MockUserRepository, mail_only_service_with_session},
        test_utils::{MockMailProvider, MockSessionService},
    };

    use super::*;

    const RAW_AUTH_TOKEN: &str = "a very long and secure auth token";

    fn auth_token() -> AuthToken {
        AuthToken::from(RAW_AUTH_TOKEN.to_string())
    }

    /// A stored auth link whose hash matches [`auth_token`].
    fn stored_auth_link(expire_at: chrono::DateTime<Utc>) -> HashedAuthLink {
        HashedAuthLink::new(
            UserId::test_default(),
            auth_token().as_hash().expect("hashing should succeed"),
            expire_at,
        )
    }

    fn new_service(
        user: MockUserRepository,
        session: MockSessionService,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        mail_only_service_with_session(user, session, MockMailProvider::new())
    }

    #[tokio::test]
    async fn test_ok_path() {
        let mut user = MockUserRepository::new();
        let link = stored_auth_link(Utc::now() + TimeDelta::minutes(5));
        let link_clone = link.clone();
        user.expect_get_all_auth_links()
            .returning(move || vec![link_clone.clone()]);
        let token = auth_token();
        user.expect_delete_auth_link_by_hash()
            .times(1)
            .withf(move |hash| hash.verify_token(&token))
            .returning(|_| Ok(()));
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
            other_token.as_hash().expect("hashing should succeed"),
            Utc::now() + TimeDelta::minutes(5),
        );
        let matching_clone = matching.clone();
        user.expect_get_all_auth_links()
            .returning(move || vec![matching_clone.clone(), other.clone()]);
        let token = auth_token();
        // Only the link matching the provided token must be deleted; the other one stays.
        user.expect_delete_auth_link_by_hash()
            .times(1)
            .withf(move |hash| hash.verify_token(&token))
            .returning(|_| Ok(()));
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
        user.expect_delete_auth_link_by_hash().returning(|_| Ok(()));
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .returning(|_| Err(()));

        let service = new_service(user, session);

        let res = service.validate_auth_link(auth_token()).await;

        assert!(res.is_err())
    }
}

#[cfg(test)]
mod test_user_service_check_session_token {
    use crate::inbound::auth::multi_user::{
        SessionToken,
        service::test_utils::{MockUserRepository, mail_only_service_with_session},
        test_utils::{MockMailProvider, MockSessionService},
    };

    use super::*;

    fn new_service(
        session: MockSessionService,
    ) -> MultiUserService<MockUserRepository, MockSessionService, MockMailProvider> {
        mail_only_service_with_session(MockUserRepository::new(), session, MockMailProvider::new())
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
