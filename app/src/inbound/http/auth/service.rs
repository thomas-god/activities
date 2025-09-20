use base64::{Engine, engine::general_purpose};
use chrono::{TimeDelta, Utc};
use derive_more::Constructor;
use rand::Rng;
use thiserror::Error;

use crate::{
    domain::models::UserId,
    inbound::http::auth::{
        EmailAddress, GenerateMagicLinkResult, ISessionService, MagicLink, MagicToken,
        SessionTokenError,
    },
};

#[derive(Debug, Clone, Constructor)]
pub struct SessionService<SR, MP>
where
    SR: SessionRepository,
    MP: MailProvider,
{
    session_repository: SR,
    mail_provider: MP,
}

impl<SR, MP> ISessionService for SessionService<SR, MP>
where
    SR: SessionRepository,
    MP: MailProvider,
{
    async fn generate_magic_link(&self, email: &EmailAddress) -> GenerateMagicLinkResult {
        let user = match self
            .session_repository
            .get_user_by_email_address(email)
            .await
        {
            Ok(Some(user)) => user,
            Ok(None) => {
                // To not leak the fact the user does not exist
                return GenerateMagicLinkResult::Success;
            }
            Err(_) => return GenerateMagicLinkResult::Retry,
        };

        let magic_token = self.generate_magic_token();
        let magic_link = MagicLink::new(
            user,
            magic_token.clone(),
            Utc::now() + TimeDelta::minutes(5),
        );

        let Ok(()) = self.session_repository.store_magic_link(&magic_link).await else {
            return GenerateMagicLinkResult::Retry;
        };

        let Ok(()) = self
            .mail_provider
            .send_magic_link_email(email, &magic_link)
            .await
        else {
            let _ = self
                .session_repository
                .delete_magic_link_by_token(&magic_token)
                .await;
            return GenerateMagicLinkResult::Retry;
        };

        GenerateMagicLinkResult::Success
    }

    async fn check_session_token(&self, _token: &str) -> Result<UserId, SessionTokenError> {
        todo!()
    }
}
impl<SR, MP> SessionService<SR, MP>
where
    SR: SessionRepository,
    MP: MailProvider,
{
    fn generate_magic_token(&self) -> MagicToken {
        let mut rng = rand::rng();
        let mut random_bytes = [0u8; 24];
        rng.fill(&mut random_bytes);

        general_purpose::URL_SAFE_NO_PAD.encode(random_bytes).into()
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum SessionRepositoryError {
    #[error("An error occured with the session repository")]
    Error,
}

pub trait SessionRepository: Clone + Send + Sync + 'static {
    fn get_user_by_email_address(
        &self,
        email: &EmailAddress,
    ) -> impl Future<Output = Result<Option<UserId>, SessionRepositoryError>> + Send;

    fn store_magic_link(
        &self,
        link: &MagicLink,
    ) -> impl Future<Output = Result<(), SessionRepositoryError>> + Send;

    fn delete_magic_link_by_token(
        &self,
        token: &MagicToken,
    ) -> impl Future<Output = Result<(), SessionRepositoryError>> + Send;
}

pub trait MailProvider: Clone + Send + Sync + 'static {
    fn send_magic_link_email(
        &self,
        email: &EmailAddress,
        magic_link: &MagicLink,
    ) -> impl Future<Output = Result<(), ()>> + Send;
}

#[cfg(test)]
mod test_utils {
    use super::*;

    use mockall::mock;

    mock! {
        pub SessionRepository {}

        impl Clone for SessionRepository {
            fn clone(&self) -> Self;
        }

        impl SessionRepository for SessionRepository {
            async fn get_user_by_email_address(
                &self,
                email: &EmailAddress
            ) -> Result<Option<UserId>, SessionRepositoryError>;

            async fn store_magic_link(
                &self,
                link: &MagicLink,
            ) -> Result<(), SessionRepositoryError>;

            async fn delete_magic_link_by_token(
                &self,
                token: &MagicToken,
            ) -> Result<(), SessionRepositoryError>;
        }
    }

    mock! {
        pub MailProvider {}

        impl Clone for MailProvider {
            fn clone(&self) -> Self;
        }

        impl MailProvider for MailProvider {
            async fn send_magic_link_email(
                &self,
                email: &EmailAddress,
                magic_link: &MagicLink,
            ) -> Result<(), ()>;
        }
    }
}

#[cfg(test)]
mod test {

    use crate::inbound::http::auth::service::test_utils::{
        MockMailProvider, MockSessionRepository,
    };

    use super::*;

    #[tokio::test]
    async fn test_generate_magic_link_return_success_if_user_does_not_match_email() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_get_user_by_email_address()
            .returning(|_| Ok(None));
        let service = SessionService::new(repository, MockMailProvider::new());
        let email = "test_email".into();

        let res = service.generate_magic_link(&email).await;

        let GenerateMagicLinkResult::Success = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Success")
        };
    }

    #[tokio::test]
    async fn test_generate_magic_link_return_failure_if_repository_err() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_get_user_by_email_address()
            .returning(|_| Err(SessionRepositoryError::Error));
        let service = SessionService::new(repository, MockMailProvider::new());
        let email = "test_email".into();

        let res = service.generate_magic_link(&email).await;

        let GenerateMagicLinkResult::Retry = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Retry")
        };
    }

    #[tokio::test]
    async fn test_generate_magic_link_return_failure_if_storing_magic_link_err() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_get_user_by_email_address()
            .returning(|_| Ok(Some(UserId::test_default())));
        repository
            .expect_store_magic_link()
            .returning(|_| Err(SessionRepositoryError::Error));
        let mut email_provider = MockMailProvider::new();
        email_provider.expect_send_magic_link_email().times(0);
        let service = SessionService::new(repository, email_provider);
        let email = "test_email".into();

        let res = service.generate_magic_link(&email).await;

        let GenerateMagicLinkResult::Retry = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Retry")
        };
    }

    #[tokio::test]
    async fn test_generate_magic_link_return_failure_if_sending_magic_link_err_and_delete_magic_link()
     {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_get_user_by_email_address()
            .returning(|_| Ok(Some(UserId::test_default())));
        repository.expect_store_magic_link().returning(|_| Ok(()));
        repository
            .expect_delete_magic_link_by_token()
            .times(1)
            .returning(|_| Ok(()));
        let mut email_provider = MockMailProvider::new();
        email_provider
            .expect_send_magic_link_email()
            .returning(|_, _| Err(()));
        let service = SessionService::new(repository, email_provider);
        let email = "test_email".into();

        let res = service.generate_magic_link(&email).await;

        let GenerateMagicLinkResult::Retry = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Retry")
        };
    }

    #[tokio::test]
    async fn test_generate_magic_link_ok_store_link_and_send_email() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_get_user_by_email_address()
            .returning(|_| Ok(Some(UserId::test_default())));
        repository
            .expect_store_magic_link()
            .times(1)
            .withf(|link| link.user() == &UserId::test_default())
            .returning(|_| Ok(()));

        let mut email_provider = MockMailProvider::new();
        email_provider
            .expect_send_magic_link_email()
            .times(1)
            .withf(|mail, magic_link| {
                mail == &"test_email".into() && magic_link.user() == &UserId::test_default()
            })
            .returning(|_, _| Ok(()));

        let servide = SessionService::new(repository, email_provider);

        let res = servide.generate_magic_link(&"test_email".into()).await;

        let GenerateMagicLinkResult::Success = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Success")
        };
    }
}
