use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::{
    domain::models::UserId,
    inbound::http::auth::{
        HashedSession, HashedSessionToken, services::session::SessionRepository,
    },
};

#[derive(Debug, Clone)]
pub struct SqliteSessionRepository {
    pool: SqlitePool,
}

impl SqliteSessionRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migrations
        sqlx::migrate!("migrations/auth/sessions")
            .run(&pool)
            .await?;
        Ok(Self { pool })
    }
}

impl SessionRepository for SqliteSessionRepository {
    async fn store_session(&self, session: &HashedSession) -> Result<(), ()> {
        sqlx::query(
            r#"
        INSERT INTO t_sessions VALUES (
            ?1, ?2, ?3
        );"#,
        )
        .bind(session.user().to_string())
        .bind(session.hash().to_string())
        .bind(session.expire_at())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|_| ())
    }

    async fn get_all_sessions(&self) -> Vec<HashedSession> {
        let res: Vec<(String, String, DateTime<Utc>)> =
            match sqlx::query_as("SELECT user, token_hash, expire_at FROM t_sessions")
                .fetch_all(&self.pool)
                .await
            {
                Ok(res) => res,
                Err(err) => {
                    tracing::warn!("Cannot fetch sessions from database");
                    tracing::warn!("{}", err);
                    return Vec::new();
                }
            };
        res.iter()
            .map(|(user, token, expire_at)| {
                HashedSession::new(
                    UserId::from(user.clone()),
                    HashedSessionToken::new(token.clone()),
                    *expire_at,
                )
            })
            .collect()
    }

    async fn delete_session_by_hash(&self, hash: &HashedSessionToken) -> Result<(), ()> {
        sqlx::query("DELETE FROM t_sessions WHERE token_hash = ?1;")
            .bind(hash.to_string())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }
}

#[cfg(test)]
mod test_sqlite_session_repository {

    use chrono::{DateTime, TimeDelta, Utc};
    use tempfile::NamedTempFile;

    use crate::{
        domain::models::UserId,
        inbound::http::auth::{HashedSessionToken, Session, SessionToken},
    };

    use super::*;

    #[tokio::test]
    async fn test_init_table() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteSessionRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        sqlx::query("select count(*) from t_sessions;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_store_session() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteSessionRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            expire_at,
        );
        let hashed_session = session.as_hash().unwrap();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_sessions;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 1);

        let res: (String, String, DateTime<Utc>) =
            sqlx::query_as("select user, token_hash, expire_at from t_sessions limit 1;")
                .fetch_one(&repository.pool)
                .await
                .unwrap();

        assert_eq!(res.0, UserId::test_default().to_string());
        assert_eq!(res.1, hashed_session.hash().to_string());
        assert_eq!(res.2, expire_at);
    }

    #[tokio::test]
    async fn test_store_session_token_for_that_user_already_exists() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteSessionRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Store a first session
        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            expire_at,
        );
        let hashed_session = session.as_hash().unwrap();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        // Store a second session, for the same user, different token
        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let session = Session::new(
            UserId::test_default(),
            SessionToken::from("another_test_token".to_string()),
            expire_at,
        );
        let hashed_session = session.as_hash().unwrap();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_sessions;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 2);
    }

    #[tokio::test]
    async fn test_store_session_reject_if_token_already_exists() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteSessionRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            expire_at,
        );
        let hashed_session = session.as_hash().unwrap();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        // Store operation should fail for same hash
        repository
            .store_session(&hashed_session)
            .await
            .expect_err("Should have returned an err");

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_sessions;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 1);

        // Existing token should not be changed
        let res: (String, String, DateTime<Utc>) =
            sqlx::query_as("select user, token_hash, expire_at from t_sessions limit 1;")
                .fetch_one(&repository.pool)
                .await
                .unwrap();

        assert_eq!(res.0, UserId::test_default().to_string());
        assert_eq!(res.1, hashed_session.hash().to_string());
        assert_eq!(res.2, expire_at);
    }

    #[tokio::test]
    async fn test_get_all_tokens_empty() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteSessionRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert!(repository.get_all_sessions().await.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_tokens() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteSessionRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let session = Session::new(
            UserId::test_default(),
            SessionToken::from("a_token".to_string()),
            expire_at,
        );
        repository
            .store_session(&session.as_hash().unwrap())
            .await
            .expect("store_session should have succeeded");

        repository
            .store_session(
                &Session::new(
                    UserId::test_default(),
                    SessionToken::from("another_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_session should have succeeded");

        let res = repository.get_all_sessions().await;

        assert_eq!(res.len(), 2);

        let first_token = res.first().unwrap();
        assert_eq!(first_token.user(), &UserId::test_default());
        assert!(
            first_token
                .hash()
                .verify_token(&SessionToken::from("a_token".to_string()))
        );
        assert_eq!(first_token.expire_at(), &expire_at);
    }

    #[tokio::test]
    async fn test_delete_session_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteSessionRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            expire_at,
        );
        let hashed_session = session.as_hash().unwrap();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_sessions;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 1);

        // Delete operation
        repository
            .delete_session_by_hash(hashed_session.hash())
            .await
            .unwrap();

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_sessions;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 0);
    }

    #[tokio::test]
    async fn test_delete_session_ok_when_token_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteSessionRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Delete operation
        repository
            .delete_session_by_hash(&HashedSessionToken::new("test_token".to_string()))
            .await
            .unwrap();

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_sessions;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 0);
    }
}
