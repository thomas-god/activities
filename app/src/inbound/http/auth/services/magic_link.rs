use std::sync::Arc;

use chrono::{TimeDelta, Utc};
use derive_more::Constructor;
use thiserror::Error;
use tokio::sync::Mutex;

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
    magic_link_repository: Arc<Mutex<SR>>,
    mail_provider: Arc<MP>,
}

impl<SR, MP> IMagicLinkService for MagicLinkService<SR, MP>
where
    SR: MagicLinkRepository,
    MP: MailProvider,
{
    async fn generate_magic_link(&self, req: GenerateMagicLinkRequest) -> GenerateMagicLinkResult {
        let magic_token = MagicToken::new();
        let magic_link = MagicLink::new(
            req.user().clone(),
            magic_token.clone(),
            Utc::now() + TimeDelta::minutes(5),
        );

        let repository = self.magic_link_repository.lock().await;

        let Ok(()) = repository.store_magic_link(&magic_link).await else {
            return GenerateMagicLinkResult::Retry;
        };

        let Ok(()) = self
            .mail_provider
            .send_magic_link_email(req.email(), &magic_link)
            .await
        else {
            let _ = repository.delete_magic_link_by_token(&magic_token).await;
            return GenerateMagicLinkResult::Retry;
        };

        GenerateMagicLinkResult::Success
    }

    async fn validate_magic_token(
        &self,
        token: &MagicToken,
    ) -> Result<Option<crate::domain::models::UserId>, ()> {
        let repository = self.magic_link_repository.lock().await;
        let links = repository.get_all_magic_links().await;

        let mut found = None;

        let now = Utc::now();
        for link in links {
            if link.is_expired(&now) {
                let _ = repository.delete_magic_link_by_token(token).await;
                continue;
            }
            if link.token().match_token_secure(token) {
                found = Some(link);
            }
        }

        if found.is_some() {
            let _ = repository.delete_magic_link_by_token(token).await;
        }

        Ok(found.map(|link| link.user().clone()))
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

    fn get_all_magic_links(&self) -> impl Future<Output = Vec<MagicLink>> + Send;

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

            async fn get_all_magic_links(&self) -> Vec<MagicLink>;

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
mod test_magic_link_service_generate_magic_link {

    use crate::{
        domain::models::UserId,
        inbound::http::auth::{
            GenerateMagicLinkRequest,
            services::magic_link::test_utils::{MockMailProvider, MockSessionRepository},
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_return_failure_if_storing_magic_link_err() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_store_magic_link()
            .returning(|_| Err(MagicLinkRepositoryError::Error));
        let mut email_provider = MockMailProvider::new();
        email_provider.expect_send_magic_link_email().times(0);
        let service =
            MagicLinkService::new(Arc::new(Mutex::new(repository)), Arc::new(email_provider));

        let req = GenerateMagicLinkRequest::new(
            UserId::test_default(),
            EmailAddress::try_from("test@email.test").unwrap(),
        );

        let res = service.generate_magic_link(req).await;

        let GenerateMagicLinkResult::Retry = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Retry")
        };
    }

    #[tokio::test]
    async fn test_return_failure_if_sending_magic_link_err_and_delete_magic_link() {
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
        let service =
            MagicLinkService::new(Arc::new(Mutex::new(repository)), Arc::new(email_provider));

        let req = GenerateMagicLinkRequest::new(
            UserId::test_default(),
            EmailAddress::try_from("test@email.com").unwrap(),
        );

        let res = service.generate_magic_link(req).await;

        let GenerateMagicLinkResult::Retry = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Retry")
        };
    }

    #[tokio::test]
    async fn test_ok_store_link_and_send_email() {
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
                mail == &"test@email.test".try_into().unwrap()
                    && magic_link.user() == &UserId::test_default()
            })
            .returning(|_, _| Ok(()));

        let service =
            MagicLinkService::new(Arc::new(Mutex::new(repository)), Arc::new(email_provider));

        let req = GenerateMagicLinkRequest::new(
            UserId::test_default(),
            EmailAddress::try_from("test@email.test").unwrap(),
        );

        let res = service.generate_magic_link(req).await;

        let GenerateMagicLinkResult::Success = res else {
            unreachable!("Should have return a GenerateMagicLinkResult::Success")
        };
    }
}

#[cfg(test)]
mod test_magic_link_service_validate_magic_link {
    use crate::{
        domain::models::UserId,
        inbound::http::auth::services::magic_link::test_utils::{
            MockMailProvider, MockSessionRepository,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_ok_matching_link_found() {
        let mut repository = MockSessionRepository::new();
        let token = MagicToken::new();
        let token_clone = token.clone();
        repository.expect_get_all_magic_links().returning(move || {
            vec![MagicLink::new(
                UserId::test_default(),
                token_clone.clone(),
                Utc::now() + TimeDelta::minutes(5),
            )]
        });
        let token_clone = token.clone();
        repository
            .expect_delete_magic_link_by_token()
            .times(1)
            .withf(move |token| token.match_token_secure(&token_clone))
            .returning(|_| Ok(()));

        let service = MagicLinkService::new(
            Arc::new(Mutex::new(repository)),
            Arc::new(MockMailProvider::new()),
        );

        let res = service.validate_magic_token(&token).await;

        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Some(UserId::test_default()));
    }

    #[tokio::test]
    async fn test_no_matching_link_found() {
        let mut repository = MockSessionRepository::new();
        let token = MagicToken::new();
        repository.expect_get_all_magic_links().returning(Vec::new);
        repository.expect_delete_magic_link_by_token().times(0);

        let service = MagicLinkService::new(
            Arc::new(Mutex::new(repository)),
            Arc::new(MockMailProvider::new()),
        );

        let res = service.validate_magic_token(&token).await;

        assert!(res.is_ok());
        assert_eq!(res.unwrap(), None);
    }

    #[tokio::test]
    async fn test_matching_link_found_but_expired() {
        let mut repository = MockSessionRepository::new();
        let token = MagicToken::new();
        let token_clone = token.clone();
        repository.expect_get_all_magic_links().returning(move || {
            vec![MagicLink::new(
                UserId::test_default(),
                token_clone.clone(),
                Utc::now() - TimeDelta::minutes(5),
            )]
        });
        let token_clone = token.clone();
        repository
            .expect_delete_magic_link_by_token()
            .times(1)
            .withf(move |token| token.match_token_secure(&token_clone))
            .returning(|_| Ok(()));

        let service = MagicLinkService::new(
            Arc::new(Mutex::new(repository)),
            Arc::new(MockMailProvider::new()),
        );

        let res = service.validate_magic_token(&token).await;

        assert!(res.is_ok());
        assert_eq!(res.unwrap(), None);
    }
}
