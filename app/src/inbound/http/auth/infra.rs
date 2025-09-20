use std::sync::Arc;

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::inbound::http::auth::{
    EmailAddress, MagicLink, MagicToken,
    services::{
        magic_link::{MagicLinkRepository, MagicLinkRepositoryError, MailProvider},
        user::UserRepository,
    },
};

#[derive(Debug, Clone, Constructor)]
pub struct InMemoryMagicLinkRepository {
    magic_links: Arc<Mutex<Vec<MagicLink>>>,
}

impl MagicLinkRepository for InMemoryMagicLinkRepository {
    async fn store_magic_link(&self, link: &MagicLink) -> Result<(), MagicLinkRepositoryError> {
        self.magic_links.lock().await.push(link.clone());
        Ok(())
    }

    async fn delete_magic_link_by_token(
        &self,
        token: &MagicToken,
    ) -> Result<(), MagicLinkRepositoryError> {
        self.magic_links
            .lock()
            .await
            .retain(|link| link.token() != token);

        Ok(())
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct DoNothingMailProvider {}

impl MailProvider for DoNothingMailProvider {
    async fn send_magic_link_email(
        &self,
        email: &EmailAddress,
        _magic_link: &MagicLink,
    ) -> Result<(), ()> {
        tracing::info!("Dummy send to {email:?}");
        Ok(())
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct InMemoryUserRepository {}

impl UserRepository for InMemoryUserRepository {
    async fn get_user_by_email(
        &self,
        email: &EmailAddress,
    ) -> Result<Option<crate::domain::models::UserId>, ()> {
        todo!()
    }

    async fn store_user_with_mail(
        &self,
        user: &crate::domain::models::UserId,
        email: &EmailAddress,
    ) -> Result<(), ()> {
        todo!()
    }
}
