use std::sync::Arc;

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::{
    domain::models::UserId,
    inbound::http::auth::{
        AuthLinkValidationResult, AuthToken, EmailAddress, GenerateAuthLinkRequest,
        GenerateAuthLinkResult, IAuthLinkService, ISessionService, IUserService, SessionToken,
        UserLoginResult, UserRegistrationResult,
    },
};

#[derive(Debug, Clone, Constructor)]
pub struct UserService<MLS, UR, SS> {
    auth_link_service: Arc<Mutex<MLS>>,
    user_repository: Arc<Mutex<UR>>,
    session_service: Arc<Mutex<SS>>,
}

impl<MLS, UR, SS> IUserService for UserService<MLS, UR, SS>
where
    MLS: IAuthLinkService,
    UR: UserRepository,
    SS: ISessionService,
{
    async fn register_user(&self, email: EmailAddress) -> UserRegistrationResult {
        let repo = self.user_repository.lock().await;
        let user = match repo.get_user_by_email(&email).await {
            Ok(None) => {
                let user = UserId::new();
                let store_res = repo.store_user_with_mail(&user, &email).await;
                if store_res.is_err() {
                    return UserRegistrationResult::Retry;
                }
                user
            }
            Ok(Some(user)) => user,
            Err(()) => return UserRegistrationResult::Retry,
        };

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

    async fn login_user(&self, email: EmailAddress) -> UserLoginResult {
        let user = match self
            .user_repository
            .lock()
            .await
            .get_user_by_email(&email)
            .await
        {
            Ok(Some(user)) => user,
            Ok(None) => return UserLoginResult::Success,
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
            GenerateAuthLinkResult::Success => UserLoginResult::Success,
            GenerateAuthLinkResult::Retry => UserLoginResult::Retry,
        }
    }

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
            .lock()
            .await
            .generate_session_token(&user)
            .await
            .map(AuthLinkValidationResult::Success)
    }

    async fn check_session_token(&self, token: &SessionToken) -> Result<UserId, ()> {
        self.session_service
            .lock()
            .await
            .check_session_token(token)
            .await
    }
}

pub trait UserRepository: Clone + Send + Sync + 'static {
    fn get_user_by_email(
        &self,
        email: &EmailAddress,
    ) -> impl Future<Output = Result<Option<UserId>, ()>> + Send;

    fn store_user_with_mail(
        &self,
        user: &UserId,
        email: &EmailAddress,
    ) -> impl Future<Output = Result<(), ()>> + Send;
}

#[cfg(test)]
mod test_utils {
    use mockall::mock;

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
            ) -> Result<Option<UserId>, ()>;

            async fn store_user_with_mail(
                &self,
                user: &UserId,
                email: &EmailAddress
            ) -> Result<(), ()>;
        }
    }
}

#[cfg(test)]
mod test_user_service_register_new_user {
    use crate::{
        domain::models::UserId,
        inbound::http::auth::{
            EmailAddress, GenerateAuthLinkResult, UserRegistrationResult,
            services::user::test_utils::MockUserRepository,
            test_utils::{MockAuthLinkService, MockSessionService},
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
        user.expect_store_user_with_mail()
            .times(1)
            .withf(|_user, email| email == &"test@mail.test".try_into().unwrap())
            .returning(|_, _| Ok(()));

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .register_user(EmailAddress("test@mail.test".to_string()))
            .await;

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
        user.expect_store_user_with_mail()
            .times(1)
            .returning(|_, __| Ok(()));
        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .register_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }

    #[tokio::test]
    async fn test_store_user_fails() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user_with_mail()
            .times(1)
            .returning(|_, __| Err(()));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .register_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }

    #[tokio::test]
    async fn test_user_already_exists() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some(UserId::test_default())));
        user.expect_store_user_with_mail().times(0);
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .withf(|link| link.user() == &UserId::test_default())
            .returning(|_| GenerateAuthLinkResult::Success);

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .register_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_cannot_check_if_user_already_exists() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Err(()));
        user.expect_store_user_with_mail().times(0);
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .register_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }
}

#[cfg(test)]
mod test_user_service_login_user {
    use crate::inbound::http::auth::{
        services::user::test_utils::MockUserRepository,
        test_utils::{MockAuthLinkService, MockSessionService},
    };

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some(UserId::test_default())));
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .times(1)
            .withf(|link| link.email() == &"test@mail.test".try_into().unwrap())
            .returning(|_| GenerateAuthLinkResult::Success);

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .login_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserLoginResult::Success);
    }

    #[tokio::test]
    async fn test_user_does_not_exist() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .login_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserLoginResult::Success);
    }

    #[tokio::test]
    async fn test_cannot_check_if_user_exist() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Err(()));
        let mut auth_link = MockAuthLinkService::new();
        auth_link.expect_generate_auth_link().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .login_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserLoginResult::Retry);
    }

    #[tokio::test]
    async fn test_when_auth_link_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some(UserId::test_default())));
        let mut auth_link = MockAuthLinkService::new();
        auth_link
            .expect_generate_auth_link()
            .returning(|_| GenerateAuthLinkResult::Retry);

        let service = UserService::new(
            Arc::new(Mutex::new(auth_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .login_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserLoginResult::Retry);
    }
}

#[derive(Debug, Clone)]
pub struct DisabledUserService {}

impl IUserService for DisabledUserService {
    async fn check_session_token(
        &self,
        _token: &SessionToken,
    ) -> Result<crate::domain::models::UserId, ()> {
        panic!("User service is disabled")
    }

    async fn login_user(&self, _email: EmailAddress) -> crate::inbound::http::UserLoginResult {
        panic!("User service is disabled")
    }

    async fn register_user(&self, _email: EmailAddress) -> UserRegistrationResult {
        panic!("User service is disabled")
    }

    async fn validate_auth_link(
        &self,
        _token: AuthToken,
    ) -> Result<crate::inbound::http::AuthLinkValidationResult, ()> {
        panic!("User service is disabled")
    }
}

#[cfg(test)]
mod test_user_service_validate_auth_link {
    use chrono::{TimeDelta, Utc};

    use crate::inbound::http::auth::{
        AuthLinkValidationResult, AuthToken, GenerateSessionTokenResult, SessionToken,
        services::user::test_utils::MockUserRepository,
        test_utils::{MockAuthLinkService, MockSessionService},
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
            Arc::new(Mutex::new(session)),
        );

        let res = service
            .validate_auth_link(AuthToken("a very long and secure auth token".to_string()))
            .await;

        let Ok(AuthLinkValidationResult::Success(session)) = res else {
            unreachable!("Should have return a Ok(AuthLinkValidationResult::Success(_))")
        };
        assert!(
            session_token
                .as_hash()
                .unwrap()
                .verify_token(session.token())
        );
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
            Arc::new(Mutex::new(session)),
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
            Arc::new(Mutex::new(session)),
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
            Arc::new(Mutex::new(session)),
        );

        let res = service
            .validate_auth_link(AuthToken("a very long and secure auth token".to_string()))
            .await;

        assert!(res.is_err())
    }
}

#[cfg(test)]
mod test_user_service_check_session_token {
    use crate::inbound::http::auth::{
        SessionToken,
        services::user::test_utils::MockUserRepository,
        test_utils::{MockAuthLinkService, MockSessionService},
    };

    use super::*;

    #[tokio::test]
    async fn test_token_valid() {
        let mut session = MockSessionService::new();
        session
            .expect_check_session_token()
            .returning(|_| Ok(UserId::test_default()));

        let service = UserService::new(
            Arc::new(Mutex::new(MockAuthLinkService::new())),
            Arc::new(Mutex::new(MockUserRepository::new())),
            Arc::new(Mutex::new(session)),
        );

        let res = service
            .check_session_token(&SessionToken(
                "a very long and secure session token".to_string(),
            ))
            .await;

        assert_eq!(res.unwrap(), UserId::test_default());
    }

    #[tokio::test]
    async fn test_token_invalid_or_does_not_exist_or_else() {
        let mut session = MockSessionService::new();
        session.expect_check_session_token().returning(|_| Err(()));

        let service = UserService::new(
            Arc::new(Mutex::new(MockAuthLinkService::new())),
            Arc::new(Mutex::new(MockUserRepository::new())),
            Arc::new(Mutex::new(session)),
        );

        let res = service
            .check_session_token(&SessionToken(
                "a very long and secure session token".to_string(),
            ))
            .await;

        assert!(res.is_err());
    }
}
