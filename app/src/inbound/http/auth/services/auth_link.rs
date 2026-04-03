use std::sync::Arc;

use chrono::{TimeDelta, Utc};
use derive_more::Constructor;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::inbound::http::auth::{
    AuthLink, AuthToken, EmailAddress, GenerateAuthLinkRequest, GenerateAuthLinkResult,
    HashedAuthLink, HashedAuthToken, IAuthLinkService,
};

#[derive(Debug, Clone, Constructor)]
pub struct AuthLinkService<SR, MP>
where
    SR: AuthLinkRepository,
    MP: MailProvider,
{
    auth_link_repository: Arc<Mutex<SR>>,
    mail_provider: Arc<MP>,
}

impl<SR, MP> IAuthLinkService for AuthLinkService<SR, MP>
where
    SR: AuthLinkRepository,
    MP: MailProvider,
{
    async fn generate_auth_link(&self, req: GenerateAuthLinkRequest) -> GenerateAuthLinkResult {
        let auth_token = AuthToken::new();
        let auth_link = AuthLink::new(
            req.user().clone(),
            auth_token.clone(),
            Utc::now() + TimeDelta::minutes(15),
        );
        let Ok(hashed_auth_link) = auth_link.as_hash() else {
            return GenerateAuthLinkResult::Retry;
        };

        let repository = self.auth_link_repository.lock().await;

        let Ok(()) = repository.store_auth_link(&hashed_auth_link).await else {
            return GenerateAuthLinkResult::Retry;
        };

        let Ok(()) = self
            .mail_provider
            .send_auth_link_email(req.email(), &auth_link)
            .await
        else {
            let _ = repository
                .delete_auth_link_by_hash(hashed_auth_link.hash())
                .await;
            return GenerateAuthLinkResult::Retry;
        };

        GenerateAuthLinkResult::Success
    }

    async fn validate_auth_token(
        &self,
        token: &AuthToken,
    ) -> Result<Option<crate::domain::models::UserId>, ()> {
        let repository = self.auth_link_repository.lock().await;
        let links = repository.get_all_auth_links().await;

        let mut found = None;

        let now = Utc::now();
        for link in links {
            if link.is_expired(&now) {
                let _ = repository.delete_auth_link_by_hash(link.hash()).await;
                continue;
            }
            if link.hash().verify_token(token) {
                found = Some(link);
            }
        }

        if let Some(link) = &found {
            let _ = repository.delete_auth_link_by_hash(link.hash()).await;
        }

        Ok(found.map(|link| link.user().clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AuthLinkRepositoryError {
    #[error("An error occured with the session repository")]
    Error,
}

pub trait AuthLinkRepository: Clone + Send + Sync + 'static {
    fn store_auth_link(
        &self,
        link: &HashedAuthLink,
    ) -> impl Future<Output = Result<(), AuthLinkRepositoryError>> + Send;

    fn get_all_auth_links(&self) -> impl Future<Output = Vec<HashedAuthLink>> + Send;

    fn delete_auth_link_by_hash(
        &self,
        hash: &HashedAuthToken,
    ) -> impl Future<Output = Result<(), AuthLinkRepositoryError>> + Send;
}

pub trait MailProvider: Clone + Send + Sync + 'static {
    fn send_auth_link_email(
        &self,
        email: &EmailAddress,
        auth_link: &AuthLink,
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

        impl AuthLinkRepository for SessionRepository {
            async fn store_auth_link(
                &self,
                link: &HashedAuthLink,
            ) -> Result<(), AuthLinkRepositoryError>;

            async fn get_all_auth_links(&self) -> Vec<HashedAuthLink>;

            async fn delete_auth_link_by_hash(
                &self,
                hash: &HashedAuthToken,
            ) -> Result<(), AuthLinkRepositoryError>;
        }
    }

    mock! {
        pub MailProvider {}

        impl Clone for MailProvider {
            fn clone(&self) -> Self;
        }

        impl MailProvider for MailProvider {
            async fn send_auth_link_email(
                &self,
                email: &EmailAddress,
                auth_link: &AuthLink,
            ) -> Result<(), ()>;
        }
    }
}

#[cfg(test)]
mod test_auth_link_service_generate_auth_link {

    use crate::{
        domain::models::UserId,
        inbound::http::auth::{
            GenerateAuthLinkRequest,
            services::auth_link::test_utils::{MockMailProvider, MockSessionRepository},
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_return_failure_if_storing_auth_link_err() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_store_auth_link()
            .returning(|_| Err(AuthLinkRepositoryError::Error));
        let mut email_provider = MockMailProvider::new();
        email_provider.expect_send_auth_link_email().times(0);
        let service =
            AuthLinkService::new(Arc::new(Mutex::new(repository)), Arc::new(email_provider));

        let req = GenerateAuthLinkRequest::new(
            UserId::test_default(),
            EmailAddress::try_from("test@email.test").unwrap(),
        );

        let res = service.generate_auth_link(req).await;

        let GenerateAuthLinkResult::Retry = res else {
            unreachable!("Should have return a GenerateAuthLinkResult::Retry")
        };
    }

    #[tokio::test]
    async fn test_return_failure_and_delete_auth_link_if_sending_auth_link_err() {
        let mut repository = MockSessionRepository::new();
        repository.expect_store_auth_link().returning(|_| Ok(()));
        repository
            .expect_delete_auth_link_by_hash()
            .times(1)
            .returning(|_| Ok(()));
        let mut email_provider = MockMailProvider::new();
        email_provider
            .expect_send_auth_link_email()
            .returning(|_, _| Err(()));
        let service =
            AuthLinkService::new(Arc::new(Mutex::new(repository)), Arc::new(email_provider));

        let req = GenerateAuthLinkRequest::new(
            UserId::test_default(),
            EmailAddress::try_from("test@email.com").unwrap(),
        );

        let res = service.generate_auth_link(req).await;

        let GenerateAuthLinkResult::Retry = res else {
            unreachable!("Should have return a GenerateAuthLinkResult::Retry")
        };
    }

    #[tokio::test]
    async fn test_ok_store_link_and_send_email() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_store_auth_link()
            .times(1)
            .withf(|link| link.user() == &UserId::test_default())
            .returning(|_| Ok(()));

        let mut email_provider = MockMailProvider::new();
        email_provider
            .expect_send_auth_link_email()
            .times(1)
            .withf(|mail, auth_link| {
                mail == &"test@email.test".try_into().unwrap()
                    && auth_link.user() == &UserId::test_default()
            })
            .returning(|_, _| Ok(()));

        let service =
            AuthLinkService::new(Arc::new(Mutex::new(repository)), Arc::new(email_provider));

        let req = GenerateAuthLinkRequest::new(
            UserId::test_default(),
            EmailAddress::try_from("test@email.test").unwrap(),
        );

        let res = service.generate_auth_link(req).await;

        let GenerateAuthLinkResult::Success = res else {
            unreachable!("Should have return a GenerateAuthLinkResult::Success")
        };
    }
}

#[cfg(test)]
mod test_auth_link_service_validate_auth_link {
    use crate::{
        domain::models::UserId,
        inbound::http::auth::services::auth_link::test_utils::{
            MockMailProvider, MockSessionRepository,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_ok_matching_link_found() {
        let mut repository = MockSessionRepository::new();
        let token = AuthToken::new();
        let hashed_token = token.as_hash().unwrap();
        let hashed_token_clone = hashed_token.clone();
        repository.expect_get_all_auth_links().returning(move || {
            vec![HashedAuthLink::new(
                UserId::test_default(),
                hashed_token_clone.clone(),
                Utc::now() + TimeDelta::minutes(5),
            )]
        });
        let token_clone = token.clone();
        repository
            .expect_delete_auth_link_by_hash()
            .times(1)
            .withf(move |hash| hash.verify_token(&token_clone))
            .returning(|_| Ok(()));

        let service = AuthLinkService::new(
            Arc::new(Mutex::new(repository)),
            Arc::new(MockMailProvider::new()),
        );

        let res = service.validate_auth_token(&token).await;

        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Some(UserId::test_default()));
    }

    #[tokio::test]
    async fn test_no_matching_link_found() {
        let mut repository = MockSessionRepository::new();
        let token = AuthToken::new();
        repository.expect_get_all_auth_links().returning(Vec::new);
        repository.expect_delete_auth_link_by_hash().times(0);

        let service = AuthLinkService::new(
            Arc::new(Mutex::new(repository)),
            Arc::new(MockMailProvider::new()),
        );

        let res = service.validate_auth_token(&token).await;

        assert!(res.is_ok());
        assert_eq!(res.unwrap(), None);
    }

    #[tokio::test]
    async fn test_matching_link_found_but_expired() {
        let mut repository = MockSessionRepository::new();
        let token = AuthToken::new();
        let hashed_token = token.as_hash().unwrap();
        let hashed_token_clone = hashed_token.clone();
        repository.expect_get_all_auth_links().returning(move || {
            vec![HashedAuthLink::new(
                UserId::test_default(),
                hashed_token_clone.clone(),
                Utc::now() - TimeDelta::minutes(5),
            )]
        });
        let token_clone = token.clone();
        repository
            .expect_delete_auth_link_by_hash()
            .times(1)
            .withf(move |hash| hash.verify_token(&token_clone))
            .returning(|_| Ok(()));

        let service = AuthLinkService::new(
            Arc::new(Mutex::new(repository)),
            Arc::new(MockMailProvider::new()),
        );

        let res = service.validate_auth_token(&token).await;

        assert!(res.is_ok());
        assert_eq!(res.unwrap(), None);
    }
}
