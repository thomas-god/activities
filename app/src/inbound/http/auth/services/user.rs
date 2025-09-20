use std::sync::Arc;

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::{
    domain::models::UserId,
    inbound::http::auth::{
        EmailAddress, GenerateMagicLinkRequest, GenerateMagicLinkResult, IMagicLinkService,
        IUserService, UserRegistrationResult,
    },
};

#[derive(Debug, Clone, Constructor)]
pub struct UserService<MLS, UR> {
    magic_link_service: Arc<Mutex<MLS>>,
    user_repository: Arc<Mutex<UR>>,
}

impl<MLS, UR> IUserService for UserService<MLS, UR>
where
    MLS: IMagicLinkService,
    UR: UserRepository,
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
            services::user::test_utils::MockUserRepository, test_utils::MockMagicLinkService,
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

        let service =
            UserService::new(Arc::new(Mutex::new(magic_link)), Arc::new(Mutex::new(user)));

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

        let service =
            UserService::new(Arc::new(Mutex::new(magic_link)), Arc::new(Mutex::new(user)));

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

        let service =
            UserService::new(Arc::new(Mutex::new(magic_link)), Arc::new(Mutex::new(user)));

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

        let service =
            UserService::new(Arc::new(Mutex::new(magic_link)), Arc::new(Mutex::new(user)));

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

        let service =
            UserService::new(Arc::new(Mutex::new(magic_link)), Arc::new(Mutex::new(user)));

        let res = service
            .register_user(EmailAddress("test@mail.test".to_string()))
            .await;

        assert_eq!(res, UserRegistrationResult::Retry);
    }
}
