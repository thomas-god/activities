use std::{collections::HashMap, sync::Arc};

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::{
    domain::models::UserId,
    inbound::http::auth::{
        EmailAddress, MagicLink, MagicToken,
        services::magic_link::{MagicLinkRepository, MagicLinkRepositoryError, MailProvider},
    },
};

#[derive(Debug, Clone, Constructor)]
pub struct InMemorySessionRepository {
    users_by_emails: Arc<Mutex<HashMap<EmailAddress, UserId>>>,
    magic_links: Arc<Mutex<Vec<MagicLink>>>,
}

impl MagicLinkRepository for InMemorySessionRepository {
    async fn get_user_by_email_address(
        &self,
        email: &EmailAddress,
    ) -> Result<Option<UserId>, MagicLinkRepositoryError> {
        Ok(self.users_by_emails.lock().await.get(email).cloned())
    }

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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_get_user_by_email_address() {
        let users_by_emails = Arc::new(Mutex::new(HashMap::from([(
            "test@mail.test".into(),
            UserId::test_default(),
        )])));
        let repository =
            InMemorySessionRepository::new(users_by_emails, Arc::new(Mutex::new(Vec::new())));

        assert_eq!(
            repository
                .get_user_by_email_address(&"test@mail.test".into())
                .await,
            Ok(Some(UserId::test_default()))
        );

        assert_eq!(
            repository
                .get_user_by_email_address(&"another_email@mail.test".into())
                .await,
            Ok(None)
        );
    }
}
