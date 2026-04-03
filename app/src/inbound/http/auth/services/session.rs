use std::sync::Arc;

use chrono::{TimeDelta, Utc};
use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::{
    domain::models::UserId,
    inbound::http::auth::{
        CheckSessionResult, GenerateSessionTokenResult, HashedSession, HashedSessionToken,
        ISessionService, Session, SessionToken,
    },
};

const SESSION_DURATION: i64 = 30;
const SESSION_REFRESH_WINDOW: i64 = 7;

#[derive(Debug, Clone, Constructor)]
pub struct SessionService<SR> {
    session_repository: Arc<Mutex<SR>>,
}

impl<SR> ISessionService for SessionService<SR>
where
    SR: SessionRepository,
{
    async fn generate_session_token(
        &self,
        user: &UserId,
    ) -> Result<GenerateSessionTokenResult, ()> {
        let token = SessionToken::new();
        let expire_at = Utc::now() + TimeDelta::days(SESSION_DURATION);
        let session = Session::new(user.clone(), token.clone(), expire_at);
        let Ok(hashed_session) = session.as_hash() else {
            return Err(());
        };

        match self
            .session_repository
            .lock()
            .await
            .store_session(&hashed_session)
            .await
        {
            Ok(()) => Ok(GenerateSessionTokenResult::new(token, expire_at)),
            Err(()) => Err(()),
        }
    }

    async fn check_session_token(&self, token: &SessionToken) -> Result<CheckSessionResult, ()> {
        let repository = self.session_repository.lock().await;

        let sessions = repository.get_all_sessions().await;

        let mut found = None;
        let now = Utc::now();
        for session in sessions {
            if session.is_expired(&now) {
                let _ = repository.delete_session_by_hash(session.hash()).await;
                continue;
            }
            if session.hash().verify_token(token) {
                found = Some(session)
            }
        }

        let Some(session) = found else {
            return Err(());
        };
        let user = session.user.clone();

        let refresh_threshold = *session.expire_at() - TimeDelta::days(SESSION_REFRESH_WINDOW);
        if now >= refresh_threshold {
            let new_token = SessionToken::new();
            let new_expire_at = now + TimeDelta::days(SESSION_DURATION);
            let new_session = Session::new(user.clone(), new_token.clone(), new_expire_at);
            if let Ok(hashed) = new_session.as_hash()
                && repository.store_session(&hashed).await.is_ok()
            {
                let _ = repository.delete_session_by_hash(session.hash()).await;
                let refreshed = Some(GenerateSessionTokenResult::new(new_token, new_expire_at));
                return Ok(CheckSessionResult { user, refreshed });
            }
        }

        Ok(CheckSessionResult {
            user,
            refreshed: None,
        })
    }
}

pub trait SessionRepository: Clone + Send + Sync + 'static {
    fn store_session(&self, session: &HashedSession)
    -> impl Future<Output = Result<(), ()>> + Send;

    fn get_all_sessions(&self) -> impl Future<Output = Vec<HashedSession>> + Send;

    fn delete_session_by_hash(
        &self,
        hash: &HashedSessionToken,
    ) -> impl Future<Output = Result<(), ()>> + Send;
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
            async fn get_all_sessions(&self) -> Vec<HashedSession>;
            async fn delete_session_by_hash(&self, hash: &HashedSessionToken) -> Result<(), ()>;
        }
    }
}

#[cfg(test)]
mod test_session_service_generate_session_token {
    use crate::inbound::http::auth::services::session::test_utils::MockSessionRepository;

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let mut repository = MockSessionRepository::new();
        repository
            .expect_store_session()
            .times(1)
            .withf(|session| session.user() == &UserId::test_default())
            .returning(|_| Ok(()));

        let service = SessionService::new(Arc::new(Mutex::new(repository)));

        let res = service
            .generate_session_token(&UserId::test_default())
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_return_err_if_store_session_fails() {
        let mut repository = MockSessionRepository::new();
        repository.expect_store_session().returning(|_| Err(()));

        let service = SessionService::new(Arc::new(Mutex::new(repository)));

        let res = service
            .generate_session_token(&UserId::test_default())
            .await;

        assert!(res.is_err());
    }
}

#[cfg(test)]
mod test_session_service_check_session_token {

    use crate::inbound::http::auth::services::session::test_utils::MockSessionRepository;

    use super::*;

    #[tokio::test]
    async fn test_ok_path() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        let hashed_token = token.as_hash().unwrap();
        let cloned_hashed_token = hashed_token.clone();
        repository.expect_get_all_sessions().returning(move || {
            vec![HashedSession::new(
                UserId::test_default(),
                cloned_hashed_token.clone(),
                Utc::now() + TimeDelta::days(30),
            )]
        });
        repository.expect_delete_session_by_hash().times(0);

        let service = SessionService::new(Arc::new(Mutex::new(repository)));

        let res = service.check_session_token(&token).await;

        assert_eq!(res.expect("Should ok").user(), &UserId::test_default());
    }

    #[tokio::test]
    async fn test_token_does_not_exist() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        repository.expect_get_all_sessions().returning(Vec::new);
        repository.expect_delete_session_by_hash().times(0);

        let service = SessionService::new(Arc::new(Mutex::new(repository)));

        let res = service.check_session_token(&token).await;

        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_token_is_expired() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        let hashed_token = token.as_hash().unwrap();
        let cloned_hashed_token = hashed_token.clone();
        repository.expect_get_all_sessions().returning(move || {
            vec![HashedSession::new(
                UserId::test_default(),
                cloned_hashed_token.clone(),
                Utc::now() - TimeDelta::minutes(5),
            )]
        });
        let cloned_token = token.clone();
        repository
            .expect_delete_session_by_hash()
            .times(1)
            .withf(move |hash| hash.verify_token(&cloned_token))
            .returning(|_| Ok(()));

        let service = SessionService::new(Arc::new(Mutex::new(repository)));

        let res = service.check_session_token(&token).await;

        assert!(res.is_err());
    }
}
