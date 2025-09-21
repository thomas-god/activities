use std::sync::Arc;

use chrono::{TimeDelta, Utc};
use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::{
    domain::models::UserId,
    inbound::http::auth::{GenerateSessionTokenResult, ISessionService, Session, SessionToken},
};

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
        let expire_at = Utc::now() + TimeDelta::days(30);
        let session = Session::new(user.clone(), token.clone(), expire_at);

        match self
            .session_repository
            .lock()
            .await
            .store_session(&session)
            .await
        {
            Ok(()) => Ok(GenerateSessionTokenResult::new(token, expire_at)),
            Err(()) => Err(()),
        }
    }

    async fn check_session_token(&self, token: &SessionToken) -> Result<UserId, ()> {
        let repository = self.session_repository.lock().await;

        let sessions = repository.get_all_sessions().await;

        let mut found = None;
        let now = Utc::now();
        for session in sessions {
            if session.is_expired(&now) {
                let _ = repository.delete_session_by_token(token).await;
                continue;
            }
            if session.token().match_token_secure(token) {
                found = Some(session)
            }
        }

        match found {
            Some(session) => Ok(session.user().clone()),
            None => Err(()),
        }
    }
}

pub trait SessionRepository: Clone + Send + Sync + 'static {
    fn store_session(&self, session: &Session) -> impl Future<Output = Result<(), ()>> + Send;

    fn get_all_sessions(&self) -> impl Future<Output = Vec<Session>> + Send;

    fn delete_session_by_token(
        &self,
        token: &SessionToken,
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
            async fn store_session(&self, session: &Session) -> Result<(), ()>;
            async fn get_all_sessions(&self) -> Vec<Session>;
            async fn delete_session_by_token(&self, token: &SessionToken) -> Result<(), ()>;
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
        let cloned_token = token.clone();
        repository.expect_get_all_sessions().returning(move || {
            vec![Session::new(
                UserId::test_default(),
                cloned_token.clone(),
                Utc::now() + TimeDelta::minutes(5),
            )]
        });
        repository.expect_delete_session_by_token().times(0);

        let service = SessionService::new(Arc::new(Mutex::new(repository)));

        let res = service.check_session_token(&token).await;

        assert_eq!(res, Ok(UserId::test_default()));
    }

    #[tokio::test]
    async fn test_token_does_not_exist() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        repository.expect_get_all_sessions().returning(Vec::new);
        repository.expect_delete_session_by_token().times(0);

        let service = SessionService::new(Arc::new(Mutex::new(repository)));

        let res = service.check_session_token(&token).await;

        assert_eq!(res, Err(()));
    }

    #[tokio::test]
    async fn test_token_is_expired() {
        let mut repository = MockSessionRepository::new();
        let token = SessionToken::new();
        let cloned_token = token.clone();
        repository.expect_get_all_sessions().returning(move || {
            vec![Session::new(
                UserId::test_default(),
                cloned_token.clone(),
                Utc::now() - TimeDelta::minutes(5),
            )]
        });
        let cloned_token = token.clone();
        repository
            .expect_delete_session_by_token()
            .times(1)
            .withf(move |token| token.match_token_secure(&cloned_token))
            .returning(|_| Ok(()));

        let service = SessionService::new(Arc::new(Mutex::new(repository)));

        let res = service.check_session_token(&token).await;

        assert_eq!(res, Err(()));
    }
}
