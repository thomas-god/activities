use chrono::{DateTime, TimeDelta, Utc};
use derive_more::Constructor;

use crate::{
    domain::models::UserId,
    inbound::auth::email_based::{
        CheckSessionResult, GenerateSessionTokenResult, HashedSession, HashedSessionToken,
        ISessionService, Session, SessionToken,
    },
};

const SESSION_DURATION: i64 = 30;
const SESSION_REFRESH_WINDOW: i64 = 7;

#[derive(Debug, Clone, Constructor)]
pub struct SessionService<SR> {
    session_repository: SR,
}

impl<SR> ISessionService for SessionService<SR>
where
    SR: SessionRepository,
{
    #[tracing::instrument(skip_all, err(Debug))]
    async fn generate_session_token(
        &self,
        user: &UserId,
    ) -> Result<GenerateSessionTokenResult, ()> {
        let token = SessionToken::new();
        let expire_at = Utc::now() + TimeDelta::days(SESSION_DURATION);
        let session = Session::new(user.clone(), token.clone(), expire_at);
        let hashed_session = session.as_hash();

        match self.session_repository.store_session(&hashed_session).await {
            Ok(()) => Ok(GenerateSessionTokenResult::new(token, expire_at)),
            Err(()) => Err(()),
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn check_session_token(&self, token: &SessionToken) -> Result<CheckSessionResult, ()> {
        let now = Utc::now();

        let Some(session) = self
            .session_repository
            .get_session_by_hash(&token.as_hash())
            .await
        else {
            return Err(());
        };
        if session.is_expired(&now) {
            return Err(());
        }
        let user = session.user.clone();

        let refresh_threshold = *session.expire_at() - TimeDelta::days(SESSION_REFRESH_WINDOW);
        if now >= refresh_threshold {
            let new_token = SessionToken::new();
            let new_expire_at = now + TimeDelta::days(SESSION_DURATION);
            let new_session = Session::new(user.clone(), new_token.clone(), new_expire_at);
            let hashed = new_session.as_hash();
            if self.session_repository.store_session(&hashed).await.is_ok() {
                let _ = self
                    .session_repository
                    .delete_session_by_hash(session.hash())
                    .await;
                let refreshed = Some(GenerateSessionTokenResult::new(new_token, new_expire_at));
                return Ok(CheckSessionResult { user, refreshed });
            }
        }

        Ok(CheckSessionResult {
            user,
            refreshed: None,
        })
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn logout(&self, token: &SessionToken) -> Result<(), ()> {
        self.session_repository
            .delete_session_by_hash(&token.as_hash())
            .await
    }
}

pub trait SessionRepository: Clone + Send + Sync + 'static {
    fn store_session(&self, session: &HashedSession)
    -> impl Future<Output = Result<(), ()>> + Send;

    fn get_session_by_hash(
        &self,
        hash: &HashedSessionToken,
    ) -> impl Future<Output = Option<HashedSession>> + Send;

    fn delete_session_by_hash(
        &self,
        hash: &HashedSessionToken,
    ) -> impl Future<Output = Result<(), ()>> + Send;

    fn delete_expired_sessions(
        &self,
        reference: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), ()>> + Send;
}

pub fn spawn_expired_sessions_cleanup<SR: SessionRepository>(
    repository: SR,
    interval: std::time::Duration,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if repository
                .delete_expired_sessions(Utc::now())
                .await
                .is_err()
            {
                tracing::warn!("Failed to clean up expired sessions");
            }
        }
    });
}

#[cfg(test)]
mod test_utils {
    use mockall::mock;

    use super::*;

    mock! {
        pub SessionRepository {}

        impl Clone for SessionRepository {
            fn clone(&self) -> Self;
        }

        impl SessionRepository for SessionRepository {
            async fn store_session(&self, session: &HashedSession) -> Result<(), ()>;
            async fn get_session_by_hash(&self, hash: &HashedSessionToken) -> Option<HashedSession>;
            async fn delete_session_by_hash(&self, hash: &HashedSessionToken) -> Result<(), ()>;
            async fn delete_expired_sessions(&self, reference: DateTime<Utc>) -> Result<(), ()>;
        }
    }
}

#[cfg(test)]
mod test_session_service_generate_session_token {
    use crate::inbound::auth::email_based::session::test_utils::MockSessionRepository;

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_store_session()
            .times(1)
            .withf(|session| session.user() == &UserId::test_default())
            .returning(|_| Ok(()));

        let service = SessionService::new(repository);

        let res = service
            .generate_session_token(&UserId::test_default())
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_return_err_if_store_session_fails() {
        let mut repository = MockSessionRepository::new();
        repository.expect_store_session().returning(|_| Err(()));

        let service = SessionService::new(repository);

        let res = service
            .generate_session_token(&UserId::test_default())
            .await;

        assert!(res.is_err());
    }
}

#[cfg(test)]
mod test_session_service_check_session_token {

    use crate::inbound::auth::email_based::session::test_utils::MockSessionRepository;

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        let hashed_token = token.as_hash();
        let expected_hash = hashed_token.clone();
        repository
            .expect_get_session_by_hash()
            .withf(move |hash| hash == &expected_hash)
            .returning(move |_| {
                Some(HashedSession::new(
                    UserId::test_default(),
                    hashed_token.clone(),
                    Utc::now() + TimeDelta::days(30),
                ))
            });
        repository.expect_delete_session_by_hash().times(0);

        let service = SessionService::new(repository);

        let res = service.check_session_token(&token).await;

        assert_eq!(res.expect("Should ok").user(), &UserId::test_default());
    }

    #[tokio::test]
    async fn test_token_does_not_exist() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        repository.expect_get_session_by_hash().returning(|_| None);
        repository.expect_delete_session_by_hash().times(0);

        let service = SessionService::new(repository);

        let res = service.check_session_token(&token).await;

        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_token_is_expired() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        let hashed_token = token.as_hash();
        repository.expect_get_session_by_hash().returning(move |_| {
            Some(HashedSession::new(
                UserId::test_default(),
                hashed_token.clone(),
                Utc::now() - TimeDelta::minutes(5),
            ))
        });
        repository.expect_delete_session_by_hash().times(0);

        let service = SessionService::new(repository);

        let res = service.check_session_token(&token).await;

        assert!(res.is_err());
    }
}

#[cfg(test)]
mod test_session_service_logout {
    use crate::inbound::auth::email_based::session::test_utils::MockSessionRepository;

    use super::*;

    #[tokio::test]
    async fn test_deletes_matching_session() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        let expected_hash = token.as_hash();
        repository
            .expect_delete_session_by_hash()
            .times(1)
            .withf(move |hash| hash == &expected_hash)
            .returning(|_| Ok(()));

        let service = SessionService::new(repository);

        let res = service.logout(&token).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_ok_when_no_matching_session() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        repository
            .expect_delete_session_by_hash()
            .times(1)
            .returning(|_| Ok(()));

        let service = SessionService::new(repository);

        let res = service.logout(&token).await;

        assert!(res.is_ok());
    }
}
