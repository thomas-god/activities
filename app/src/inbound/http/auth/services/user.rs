use std::sync::Arc;

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::{
    domain::models::UserId,
    inbound::http::auth::{
        EmailAddress, GenerateMagicLinkRequest, GenerateMagicLinkResult, IMagicLinkService,
        ISessionService, IUserService, MagicLinkValidationResult, MagicToken, UserLoginResult,
        UserRegistrationResult,
    },
};

#[derive(Debug, Clone, Constructor)]
pub struct UserService<MLS, UR, SS> {
    magic_link_service: Arc<Mutex<MLS>>,
    user_repository: Arc<Mutex<UR>>,
    session_service: Arc<Mutex<SS>>,
}

impl<MLS, UR, SS> IUserService for UserService<MLS, UR, SS>
where
    MLS: IMagicLinkService,
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

        let req = GenerateMagicLinkRequest::new(user, email);
        let res = self
            .magic_link_service
            .lock()
            .await
            .generate_magic_link(req)
            .await;

        match res {
            GenerateMagicLinkResult::Success => UserRegistrationResult::Success,
            GenerateMagicLinkResult::Retry => UserRegistrationResult::Retry,
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

        let req = GenerateMagicLinkRequest::new(user, email);
        match self
            .magic_link_service
            .lock()
            .await
            .generate_magic_link(req)
            .await
        {
            GenerateMagicLinkResult::Success => UserLoginResult::Success,
            GenerateMagicLinkResult::Retry => UserLoginResult::Retry,
        }
    }

    async fn validate_magic_link(
        &self,
        magic_token: MagicToken,
    ) -> Result<MagicLinkValidationResult, ()> {
        let user = match self
            .magic_link_service
            .lock()
            .await
            .validate_magic_token(&magic_token)
            .await
        {
            Ok(Some(user)) => user,
            Ok(None) => return Ok(MagicLinkValidationResult::Invalid),
            _ => return Err(()),
        };

        self.session_service
            .lock()
            .await
            .generate_session_token(&user)
            .await
            .map(|token| MagicLinkValidationResult::Success(token))
    }

    async fn check_session_token(
        &self,
        _token: &str,
    ) -> Result<crate::domain::models::UserId, crate::inbound::http::auth::SessionTokenError> {
        todo!()
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
            EmailAddress, GenerateMagicLinkResult, UserRegistrationResult,
            services::user::test_utils::MockUserRepository,
            test_utils::{MockMagicLinkService, MockSessionService},
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let mut magic_link = MockMagicLinkService::new();
        magic_link
            .expect_generate_magic_link()
            .times(1)
            .withf(|link| link.email() == &EmailAddress::new("test@mail.test".to_string()))
            .returning(|_| GenerateMagicLinkResult::Success);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user_with_mail()
            .times(1)
            .withf(|_user, email| email == &"test@mail.test".into())
            .returning(|_, _| Ok(()));

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .register_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserRegistrationResult::Success);
    }

    #[tokio::test]
    async fn test_magic_link_retry() {
        let mut magic_link = MockMagicLinkService::new();
        magic_link
            .expect_generate_magic_link()
            .times(1)
            .returning(|_| GenerateMagicLinkResult::Retry);
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email().returning(|_| Ok(None));
        user.expect_store_user_with_mail()
            .times(1)
            .returning(|_, __| Ok(()));
        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
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
        let mut magic_link = MockMagicLinkService::new();
        magic_link.expect_generate_magic_link().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
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
        let mut magic_link = MockMagicLinkService::new();
        magic_link
            .expect_generate_magic_link()
            .withf(|link| link.user() == &UserId::test_default())
            .returning(|_| GenerateMagicLinkResult::Success);

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
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
        let mut magic_link = MockMagicLinkService::new();
        magic_link.expect_generate_magic_link().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
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
        test_utils::{MockMagicLinkService, MockSessionService},
    };

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some(UserId::test_default())));
        let mut magic_link = MockMagicLinkService::new();
        magic_link
            .expect_generate_magic_link()
            .times(1)
            .withf(|link| link.email() == &"test@mail.test".into())
            .returning(|_| GenerateMagicLinkResult::Success);

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
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
        let mut magic_link = MockMagicLinkService::new();
        magic_link.expect_generate_magic_link().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
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
        let mut magic_link = MockMagicLinkService::new();
        magic_link.expect_generate_magic_link().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .login_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserLoginResult::Retry);
    }

    #[tokio::test]
    async fn test_when_magic_link_retry() {
        let mut user = MockUserRepository::new();
        user.expect_get_user_by_email()
            .returning(|_| Ok(Some(UserId::test_default())));
        let mut magic_link = MockMagicLinkService::new();
        magic_link
            .expect_generate_magic_link()
            .returning(|_| GenerateMagicLinkResult::Retry);

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(MockSessionService::new())),
        );

        let res = service
            .login_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserLoginResult::Retry);
    }
}

#[cfg(test)]
mod test_user_service_validate_magic_link {
    use crate::inbound::http::auth::{
        MagicLinkValidationResult, MagicToken, SessionToken,
        services::user::test_utils::MockUserRepository,
        test_utils::{MockMagicLinkService, MockSessionService},
    };

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let user = MockUserRepository::new();
        let mut magic_link = MockMagicLinkService::new();
        magic_link
            .expect_validate_magic_token()
            .returning(|_| Ok(Some(UserId::test_default())));
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(|_| Ok(SessionToken::new("a session token".to_string())));

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(session)),
        );

        let res = service
            .validate_magic_link(MagicToken(
                "a very long and secure magic token token".to_string(),
            ))
            .await;

        let Ok(MagicLinkValidationResult::Success(token)) = res else {
            unreachable!("Should have return a Ok(MagicLinkValidationResult::Success(_))")
        };
        assert_eq!(token, SessionToken::new("a session token".to_string()));
    }

    #[tokio::test]
    async fn test_magic_token_rejected() {
        let user = MockUserRepository::new();
        let mut magic_link = MockMagicLinkService::new();
        magic_link
            .expect_validate_magic_token()
            .returning(|_| Ok(None));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(session)),
        );

        let res = service
            .validate_magic_link(MagicToken(
                "a very long and secure magic token token".to_string(),
            ))
            .await;

        let Ok(MagicLinkValidationResult::Invalid) = res else {
            unreachable!("Should have return a Ok(MagicLinkValidationResult::Invalid)")
        };
    }

    #[tokio::test]
    async fn test_cannot_check_magic_token() {
        let user = MockUserRepository::new();
        let mut magic_link = MockMagicLinkService::new();
        magic_link
            .expect_validate_magic_token()
            .returning(|_| Err(()));
        let mut session = MockSessionService::new();
        session.expect_generate_session_token().times(0);

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(session)),
        );

        let res = service
            .validate_magic_link(MagicToken(
                "a very long and secure magic token token".to_string(),
            ))
            .await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_cannot_generate_session_token() {
        let user = MockUserRepository::new();
        let mut magic_link = MockMagicLinkService::new();
        magic_link
            .expect_validate_magic_token()
            .returning(|_| Ok(Some(UserId::test_default())));
        let mut session = MockSessionService::new();
        session
            .expect_generate_session_token()
            .returning(|_| Err(()));

        let service = UserService::new(
            Arc::new(Mutex::new(magic_link)),
            Arc::new(Mutex::new(user)),
            Arc::new(Mutex::new(session)),
        );

        let res = service
            .validate_magic_link(MagicToken(
                "a very long and secure magic token token".to_string(),
            ))
            .await;

        assert!(res.is_err())
    }
}
