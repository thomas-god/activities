use base64::{Engine, engine::general_purpose};
use chrono::{TimeDelta, Utc};
use derive_more::Constructor;
use rand::Rng;
use thiserror::Error;

use crate::inbound::http::auth::{
    EmailAddress, GenerateMagicLinkRequest, GenerateMagicLinkResult, IMagicLinkService, MagicLink,
    MagicToken,
};

#[derive(Debug, Clone, Constructor)]
pub struct MagicLinkService<SR, MP>
where
    SR: MagicLinkRepository,
    MP: MailProvider,
{
    magic_link_repository: SR,
    mail_provider: MP,
}

impl<SR, MP> IMagicLinkService for MagicLinkService<SR, MP>
where
    SR: MagicLinkRepository,
    MP: MailProvider,
{
    async fn generate_magic_link(&self, req: GenerateMagicLinkRequest) -> GenerateMagicLinkResult {
        let magic_token = self.generate_magic_token();
        let magic_link = MagicLink::new(
            req.user().clone(),
            magic_token.clone(),
            Utc::now() + TimeDelta::minutes(5),
        );

        let Ok(()) = self
            .magic_link_repository
            .store_magic_link(&magic_link)
            .await
        else {
            return GenerateMagicLinkResult::Retry;
        };

        let Ok(()) = self
            .mail_provider
            .send_magic_link_email(req.email(), &magic_link)
            .await
        else {
            let _ = self
                .magic_link_repository
                .delete_magic_link_by_token(&magic_token)
                .await;
            return GenerateMagicLinkResult::Retry;
        };

        GenerateMagicLinkResult::Success
    }
}

impl<SR, MP> MagicLinkService<SR, MP>
where
    SR: MagicLinkRepository,
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
pub enum MagicLinkRepositoryError {
    #[error("An error occured with the session repository")]
    Error,
}

pub trait MagicLinkRepository: Clone + Send + Sync + 'static {
    fn store_magic_link(
        &self,
        link: &MagicLink,
    ) -> impl Future<Output = Result<(), MagicLinkRepositoryError>> + Send;

    fn delete_magic_link_by_token(
        &self,
        token: &MagicToken,
    ) -> impl Future<Output = Result<(), MagicLinkRepositoryError>> + Send;
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

        impl MagicLinkRepository for SessionRepository {
            async fn store_magic_link(
                &self,
                link: &MagicLink,
            ) -> Result<(), MagicLinkRepositoryError>;

            async fn delete_magic_link_by_token(
                &self,
                token: &MagicToken,
            ) -> Result<(), MagicLinkRepositoryError>;
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

    use crate::{
        domain::models::UserId,
        inbound::http::auth::{
            GenerateMagicLinkRequest,
            services::magic_link::test_utils::{MockMailProvider, MockSessionRepository},
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_generate_magic_link_return_failure_if_storing_magic_link_err() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_store_magic_link()
            .returning(|_| Err(MagicLinkRepositoryError::Error));
        let mut email_provider = MockMailProvider::new();
        email_provider.expect_send_magic_link_email().times(0);
        let service = MagicLinkService::new(repository, email_provider);

        let req = GenerateMagicLinkRequest::new(
            UserId::test_default(),
            EmailAddress::new("test_email".to_string()),
        );

        let res = service.generate_magic_link(req).await;

        let GenerateMagicLinkResult::Retry = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Retry")
        };
    }

    #[tokio::test]
    async fn test_generate_magic_link_return_failure_if_sending_magic_link_err_and_delete_magic_link()
     {
        let mut repository = MockSessionRepository::new();
        repository.expect_store_magic_link().returning(|_| Ok(()));
        repository
            .expect_delete_magic_link_by_token()
            .times(1)
            .returning(|_| Ok(()));
        let mut email_provider = MockMailProvider::new();
        email_provider
            .expect_send_magic_link_email()
            .returning(|_, _| Err(()));
        let service = MagicLinkService::new(repository, email_provider);

        let req = GenerateMagicLinkRequest::new(
            UserId::test_default(),
            EmailAddress::new("test_email".to_string()),
        );

        let res = service.generate_magic_link(req).await;

        let GenerateMagicLinkResult::Retry = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Retry")
        };
    }

    #[tokio::test]
    async fn test_generate_magic_link_ok_store_link_and_send_email() {
        let mut repository = MockSessionRepository::new();
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

        let service = MagicLinkService::new(repository, email_provider);

        let req = GenerateMagicLinkRequest::new(
            UserId::test_default(),
            EmailAddress::new("test_email".to_string()),
        );

        let res = service.generate_magic_link(req).await;

        let GenerateMagicLinkResult::Success = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Success")
        };
    }
}
