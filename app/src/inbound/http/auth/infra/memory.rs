use std::{collections::HashMap, sync::Arc};

use derive_more::Constructor;

use tokio::sync::Mutex;

use crate::{
    domain::models::UserId,
    inbound::http::auth::{
        EmailAddress, HashedMagicLink, HashedMagicToken, MagicLink, Session, SessionToken,
        services::{
            magic_link::{MagicLinkRepository, MagicLinkRepositoryError, MailProvider},
            session::SessionRepository,
            user::UserRepository,
        },
    },
};

#[derive(Debug, Clone, Constructor)]
pub struct InMemoryMagicLinkRepository {
    magic_links: Arc<Mutex<Vec<HashedMagicLink>>>,
}

impl MagicLinkRepository for InMemoryMagicLinkRepository {
    async fn store_magic_link(
        &self,
        link: &HashedMagicLink,
    ) -> Result<(), MagicLinkRepositoryError> {
        self.magic_links.lock().await.push(link.clone());
        Ok(())
    }

    async fn get_all_magic_links(&self) -> Vec<HashedMagicLink> {
        self.magic_links.lock().await.clone()
    }

    async fn delete_magic_link_by_hash(
        &self,
        hash: &HashedMagicToken,
    ) -> Result<(), MagicLinkRepositoryError> {
        self.magic_links
            .lock()
            .await
            .retain(|link| link.hash() != hash);

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
        tracing::info!(
            "Dummy send to {email:?} for token: {:?} to user {:?}",
            _magic_link.token(),
            _magic_link.user()
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct InMemoryUserRepository {
    users_by_email: Arc<Mutex<HashMap<EmailAddress, UserId>>>,
}

impl UserRepository for InMemoryUserRepository {
    async fn get_user_by_email(&self, email: &EmailAddress) -> Result<Option<UserId>, ()> {
        Ok(self.users_by_email.lock().await.get(email).cloned())
    }

    async fn store_user_with_mail(
        &self,
        user: &crate::domain::models::UserId,
        email: &EmailAddress,
    ) -> Result<(), ()> {
        self.users_by_email
            .lock()
            .await
            .insert(email.clone(), user.clone());
        Ok(())
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct InMemorySessionRepository {
    sessions: Arc<Mutex<Vec<Session>>>,
}

impl SessionRepository for InMemorySessionRepository {
    async fn store_session(&self, session: &Session) -> Result<(), ()> {
        self.sessions.lock().await.push(session.clone());
        Ok(())
    }

    async fn get_all_sessions(&self) -> Vec<Session> {
        self.sessions.lock().await.clone()
    }

    async fn delete_session_by_token(&self, token: &SessionToken) -> Result<(), ()> {
        self.sessions
            .lock()
            .await
            .retain(|session| !session.token().match_token_secure(token));
        Ok(())
    }
}
