use std::sync::Arc;

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::{
    domain::models::UserId,
    inbound::auth::multi_user::{
        AuthLinkValidationResult, AuthMethods, AuthToken, CheckSessionResult, EmailAddress,
        GenerateAuthLinkRequest, GenerateAuthLinkResult, HashedPassword, IAuthLinkService,
        ISessionService, IUserService, Password, SessionToken, UserLoginResult,
        UserRegistrationRequest, UserRegistrationResult,
    },
};

#[derive(Debug, Clone, Constructor)]
pub struct UserService<MLS, UR, SS> {
    auth_link_service: Arc<Mutex<MLS>>,
    user_repository: Arc<Mutex<UR>>,
    session_service: SS,
    auth_methods: AuthMethods,
}

impl<MLS, UR, SS> IUserService for UserService<MLS, UR, SS>
where
    MLS: IAuthLinkService,
    UR: UserRepository,
    SS: ISessionService,
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
        let repo = self.user_repository.lock().await;
        let user = match repo.get_user_by_email(&email).await {
            Ok(None) => {
                let user = UserId::new();
                let store_res = repo.store_user(&user, &email, password_hash).await;
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

        let req = GenerateAuthLinkRequest::new(user, email);
        let res = self
            .auth_link_service
            .lock()
            .await
            .generate_auth_link(req)
            .await;

        match res {
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
        let user = match self
            .auth_link_service
            .lock()
            .await
            .validate_auth_token(&token)
            .await
        {
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

impl<MLS, UR, SS> UserService<MLS, UR, SS>
where
    MLS: IAuthLinkService,
    UR: UserRepository,
    SS: ISessionService,
{
    #[tracing::instrument(skip_all)]
    async fn login_with_email_link(&self, email: EmailAddress) -> UserLoginResult {
        let user = match self
            .user_repository
            .lock()
            .await
            .get_user_by_email(&email)
            .await
        {
            Ok(Some((user, _))) => user,
            Ok(None) => {
                AuthToken::decoy_hash();
                // Same return as the happy path to not leak email/user existence.
                return UserLoginResult::EmailSent;
            }
            Err(()) => return UserLoginResult::Retry,
        };

        let req = GenerateAuthLinkRequest::new(user, email);
        match self
            .auth_link_service
            .lock()
            .await
            .generate_auth_link(req)
            .await
        {
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
        let fetched = self
            .user_repository
            .lock()
            .await
            .get_user_by_email(&email)
            .await;

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
}

#[cfg(test)]
mod test_utils {
    use mockall::mock;

    use super::super::test_utils::{MockAuthLinkService, MockSessionService};
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
        }
    }

    pub const TEST_EMAIL: &str = "test@mail.test";

    pub fn mail_only_service(
        auth_link: MockAuthLinkService,
        user: MockUserRepository,
    ) -> UserService<MockAuthLinkService, MockUserRepository, MockSessionService> {
        mail_only_service_with_session(auth_link, user, MockSessionService::new())
    }

    pub fn mail_only_service_with_session(
        auth_link: MockAuthLinkService,
        user: MockUserRepository,
        session: MockSessionService,
    ) -> UserService<MockAuthLinkService, MockUserRepository, MockSessionService> {
        UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            session,
            AuthMethods::new(true, false),
        )
    }

    pub fn mail_and_password_service(
        auth_link: MockAuthLinkService,
        user: MockUserRepository,
    ) -> UserService<MockAuthLinkService, MockUserRepository, MockSessionService> {
        mail_and_password_service_with_session(auth_link, user, MockSessionService::new())
    }

    pub fn mail_and_password_service_with_session(
        auth_link: MockAuthLinkService,
        user: MockUserRepository,
        session: MockSessionService,
    ) -> UserService<MockAuthLinkService, MockUserRepository, MockSessionService> {
        UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            session,
            AuthMethods::new(true, true),
        )
    }

    pub fn password_only_service(
        auth_link: MockAuthLinkService,
        user: MockUserRepository,
    ) -> UserService<MockAuthLinkService, MockUserRepository, MockSessionService> {
        password_only_service_with_session(auth_link, user, MockSessionService::new())
    }

    pub fn password_only_service_with_session(
        auth_link: MockAuthLinkService,
        user: MockUserRepository,
        session: MockSessionService,
    ) -> UserService<MockAuthLinkService, MockUserRepository, MockSessionService> {
        UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            session,
            AuthMethods::new(false, true),
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
    use crate::{
        domain::models::UserId,
        inbound::auth::multi_user::{
            GenerateAuthLinkResult, UserRegistrationResult,
            test_utils::MockAuthLinkService,
            user::test_utils::{
                MockUserRepository, TEST_EMAIL, mail_and_password_service, mail_only_service,
                password_only_service, registration_request,
            },
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .withf(|link| link.email() == &EmailAddress::try_from("test@mail.test").unwrap())
            .returning(|_| GenerateAuthLinkResult::Success);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, email, password_hash| {
                email == &TEST_EMAIL.try_into().unwrap() && password_hash.is_none()
            })
            .returning(|_, _, _| Ok(()));

        let service = mail_only_service(auth_link, user);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_auth_link_retry() {
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .returning(|_| GenerateAuthLinkResult::Retry);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .returning(|_, _, _| Ok(()));
        let service = mail_only_service(auth_link, user);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }

    #[tokio::test]
    async fn test_store_user_fails() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .returning(|_, _, _| Err(()));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = mail_only_service(auth_link, user);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }

    #[tokio::test]
    async fn test_user_already_exists() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        user.expect_store_user().times(0);
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .withf(|link| link.user() == &UserId::test_default())
            .returning(|_| GenerateAuthLinkResult::Success);

        let service = mail_only_service(auth_link, user);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_cannot_check_if_user_already_exists() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Err(()));
        user.expect_store_user().times(0);
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = mail_only_service(auth_link, user);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_only_ignores_provided_password() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, _email, password_hash| password_hash.is_none())
            .returning(|_, _, _| Ok(()));
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .returning(|_| GenerateAuthLinkResult::Success);

        let service = mail_only_service(auth_link, user);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_mail_and_password_stores_password_hash() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, _email, password_hash| password_hash.is_some())
            .returning(|_, _, _| Ok(()));
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .returning(|_| GenerateAuthLinkResult::Success);

        let service = mail_and_password_service(auth_link, user);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_mail_and_password_without_password_uses_mail_flow() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, _email, password_hash| password_hash.is_none())
            .returning(|_, _, _| Ok(()));
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .returning(|_| GenerateAuthLinkResult::Success);

        let service = mail_and_password_service(auth_link, user);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_password_only_creates_user_without_validating_email() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user()
            .times(1)
            .withf(|_user, email, password_hash| {
                email == &TEST_EMAIL.try_into().unwrap() && password_hash.is_some()
            })
            .returning(|_, _, _| Ok(()));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = password_only_service(auth_link, user);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_password_only_requires_password() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().times(0);
        user.expect_store_user().times(0);
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = password_only_service(auth_link, user);

        let res = service.register_user(registration_request(None)).await;

        assert_eq!(res, UserRegistrationResult::PasswordRequired);
    }

    #[tokio::test]
    async fn test_password_only_with_existing_user_never_overwrites_password() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        user.expect_store_user().times(0);
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = password_only_service(auth_link, user);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_mail_only_with_existing_user_never_stores_password() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        user.expect_store_user().times(0);
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .returning(|_| GenerateAuthLinkResult::Success);

        let service = mail_only_service(auth_link, user);

        let res = service
            .register_user(registration_request(Some("secret")))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
    }
}

#[cfg(test)]
mod test_user_service_login_user {
    use chrono::{TimeDelta, Utc};

    use crate::inbound::auth::multi_user::{
        GenerateSessionTokenResult, HashedPassword, Password, SessionToken,
        test_utils::{MockAuthLinkService, MockSessionService},
        user::test_utils::{
            MockUserRepository, TEST_EMAIL, mail_and_password_service_with_session,
            mail_only_service, password_only_service_with_session,
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
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = mail_only_service(auth_link, user);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::EmailSent);
    }

    #[tokio::test]
    async fn test_mail_only_repo_error_returns_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Err(()));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = mail_only_service(auth_link, user);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_only_found_user_generates_auth_link() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .withf(|link| link.email() == &email())
            .returning(|_| GenerateAuthLinkResult::Success);

        let service = mail_only_service(auth_link, user);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::EmailSent);
    }

    #[tokio::test]
    async fn test_mail_only_auth_link_retry_returns_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .returning(|_| GenerateAuthLinkResult::Retry);

        let service = mail_only_service(auth_link, user);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_and_password_correct_password_creates_session() {
        let hash = password_hash("secret");
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(move |_| Ok(Some((UserId::test_default(), Some(hash.clone())))));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

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

        let service = mail_and_password_service_with_session(auth_link, user, session);

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
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = mail_and_password_service_with_session(auth_link, user, session);

        let res = service.login_user(email(), Some(password("wrong"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_mail_and_password_user_without_registered_password_is_invalid() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = mail_and_password_service_with_session(auth_link, user, session);

        let res = service.login_user(email(), Some(password("secret"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_mail_and_password_unknown_email_is_invalid() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = mail_and_password_service_with_session(auth_link, user, session);

        let res = service.login_user(email(), Some(password("secret"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_mail_and_password_session_creation_failure_returns_retry() {
        let hash = password_hash("secret");
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(move |_| Ok(Some((UserId::test_default(), Some(hash.clone())))));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .returning(|_| Err(()));

        let service = mail_and_password_service_with_session(auth_link, user, session);

        let res = service.login_user(email(), Some(password("secret"))).await;

        assert_eq!(res, UserLoginResult::Retry);
    }

    #[tokio::test]
    async fn test_mail_and_password_without_password_falls_back_to_mail() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some((UserId::test_default(), None))));
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .returning(|_| GenerateAuthLinkResult::Success);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = mail_and_password_service_with_session(auth_link, user, session);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::EmailSent);
    }

    #[tokio::test]
    async fn test_password_only_correct_password_creates_session() {
        let hash = password_hash("secret");
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(move |_| Ok(Some((UserId::test_default(), Some(hash.clone())))));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

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

        let service = password_only_service_with_session(auth_link, user, session);

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
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(auth_link, user, session);

        let res = service.login_user(email(), Some(password("wrong"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_password_only_missing_password_is_invalid() {
        let mut user = MockUserRepository::new();
        // Password-only mode must not even look the user up without a password.
        user.expect_get_user_by_email().times(0);
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(auth_link, user, session);

        let res = service.login_user(email(), None).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }

    #[tokio::test]
    async fn test_password_only_unknown_email_is_invalid() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = password_only_service_with_session(auth_link, user, session);

        let res = service.login_user(email(), Some(password("secret"))).await;

        assert_eq!(res, UserLoginResult::Invalid);
    }
}

#[derive(Debug, Clone)]
pub struct DisabledUserService {}

impl IUserService for DisabledUserService {
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
        AuthLinkValidationResult, AuthToken, GenerateSessionTokenResult, SessionToken,
        test_utils::{MockAuthLinkService, MockSessionService},
        user::test_utils::MockUserRepository,
    };

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let user = MockUserRepository::new();
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_validate_auth_token()
            .returning(|_| Ok(Some(UserId::test_default())));
        let mut session = MockSessionService::new();
        let session_token = SessionToken::new();
        let cloned_session_token = session_token.clone();
        let expire_at = Utc::now() + TimeDelta::minutes(5);
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

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            session,
            AuthMethods::new(true, false),
        );

        let res = service
            .validate_auth_link(AuthToken("a very long and secure auth token".to_string()))
            .await;

        let Ok(AuthLinkValidationResult::Success(session)) = res else {
            unreachable!("Should have return a Ok(AuthLinkValidationResult::Success(_))")
        };
        assert_eq!(session_token.as_hash(), session.token().as_hash());
        assert_eq!(*session.expire_at(), expire_at);
    }

    #[tokio::test]
    async fn test_auth_token_rejected() {
        let user = MockUserRepository::new();
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_validate_auth_token()
            .returning(|_| Ok(None));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            session,
            AuthMethods::new(true, false),
        );

        let res = service
            .validate_auth_link(AuthToken("a very long and secure auth token".to_string()))
            .await;

        let Ok(AuthLinkValidationResult::Invalid) = res else {
            unreachable!("Should have return a Ok(AuthLinkValidationResult::Invalid)")
        };
    }

    #[tokio::test]
    async fn test_cannot_check_auth_token() {
        let user = MockUserRepository::new();
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_validate_auth_token()
            .returning(|_| Err(()));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            session,
            AuthMethods::new(true, false),
        );

        let res = service
            .validate_auth_link(AuthToken("a very long and secure auth token".to_string()))
            .await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_cannot_generate_session_token() {
        let user = MockUserRepository::new();
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_validate_auth_token()
            .returning(|_| Ok(Some(UserId::test_default())));
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .returning(|_| Err(()));

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            session,
            AuthMethods::new(true, false),
        );

        let res = service
            .validate_auth_link(AuthToken("a very long and secure auth token".to_string()))
            .await;

        assert!(res.is_err())
    }
}

#[cfg(test)]
mod test_user_service_check_session_token {
    use crate::inbound::auth::multi_user::{
        SessionToken,
        test_utils::{MockAuthLinkService, MockSessionService},
        user::test_utils::MockUserRepository,
    };

    use super::*;

    #[tokio::test]
    async fn test_token_valid() {
        let mut session = MockSessionService::new();
        session
            .expect_check_session_token()
            .returning(|_| Ok(CheckSessionResult::new(UserId::test_default(), None)));

        let service = UserService::new(
            Arc::new(Mutex::new(MockAuthLinkService::new())),
            Arc::new(Mutex::new(MockUserRepository::new())),
            session,
            AuthMethods::new(true, false),
        );

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

        let service = UserService::new(
            Arc::new(Mutex::new(MockAuthLinkService::new())),
            Arc::new(Mutex::new(MockUserRepository::new())),
            session,
            AuthMethods::new(true, false),
        );

        let res = service
            .check_session_token(&SessionToken(
                "a very long and secure session token".to_string(),
            ))
            .await;

        assert!(res.is_err());
    }
}
