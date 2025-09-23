use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};

use crate::{
    domain::models::UserId,
    inbound::http::auth::{
        MagicLink, MagicToken,
        services::magic_link::{MagicLinkRepository, MagicLinkRepositoryError},
    },
};

#[derive(Debug, Clone)]
pub struct SqliteMagicLinkRepository {
    pool: SqlitePool,
}

impl SqliteMagicLinkRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migration here for now
        sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS t_magic_link (
            user TEXT,
            token TEXT UNIQUE,
            expire_at TIMESTAMP
        );"#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}

impl MagicLinkRepository for SqliteMagicLinkRepository {
    async fn store_magic_link(&self, link: &MagicLink) -> Result<(), MagicLinkRepositoryError> {
        sqlx::query(
            r#"
        INSERT INTO t_magic_link VALUES (
            ?1, ?2, ?3
        );"#,
        )
        .bind(link.user().to_string())
        .bind(link.token().to_string())
        .bind(link.expire_at())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|_| MagicLinkRepositoryError::Error)
    }

    async fn get_all_magic_links(&self) -> Vec<MagicLink> {
        let res: Vec<(String, String, DateTime<Utc>)> =
            match sqlx::query_as("SELECT user, token, expire_at FROM t_magic_link")
                .fetch_all(&self.pool)
                .await
            {
                Ok(res) => res,
                Err(err) => {
                    tracing::warn!("Cannot fetch magic links from database");
                    tracing::warn!("{}", err);
                    return Vec::new();
                }
            };
        res.iter()
            .map(|(user, token, expire_at)| {
                MagicLink::new(
                    UserId::from(user.clone()),
                    MagicToken::from(token.clone()),
                    *expire_at,
                )
            })
            .collect()
    }

    async fn delete_magic_link_by_token(
        &self,
        token: &MagicToken,
    ) -> Result<(), MagicLinkRepositoryError> {
        sqlx::query("DELETE FROM t_magic_link WHERE token = ?1;")
            .bind(token.to_string())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|_| MagicLinkRepositoryError::Error)
    }
}

#[cfg(test)]
mod test_sqlite_magic_link_repository {

    use chrono::{DateTime, TimeDelta, Utc};
    use tempfile::NamedTempFile;

    use crate::domain::models::UserId;

    use super::*;

    #[tokio::test]
    async fn test_init_table() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteMagicLinkRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        sqlx::query("select count(*) from t_magic_link;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_store_magic_link() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteMagicLinkRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let magic_link = MagicLink::new(
            UserId::test_default(),
            MagicToken::from("test_token".to_string()),
            expire_at,
        );

        repository
            .store_magic_link(&magic_link)
            .await
            .expect("store_magic_link should have succeeded");

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_magic_link;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 1);

        let res: (String, String, DateTime<Utc>) =
            sqlx::query_as("select user, token, expire_at from t_magic_link limit 1;")
                .fetch_one(&repository.pool)
                .await
                .unwrap();

        assert_eq!(res.0, UserId::test_default().to_string());
        assert_eq!(res.1, "test_token".to_string());
        assert_eq!(res.2, expire_at);
    }

    #[tokio::test]
    async fn test_store_magic_link_token_for_that_user_already_exists() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteMagicLinkRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Store a first magic link
        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let magic_link = MagicLink::new(
            UserId::test_default(),
            MagicToken::from("test_token".to_string()),
            expire_at,
        );

        repository
            .store_magic_link(&magic_link)
            .await
            .expect("store_magic_link should have succeeded");

        // Store a second magic link, for the same user, different token
        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let magic_link = MagicLink::new(
            UserId::test_default(),
            MagicToken::from("another_test_token".to_string()),
            expire_at,
        );

        repository
            .store_magic_link(&magic_link)
            .await
            .expect("store_magic_link should have succeeded");

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_magic_link;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 2);
    }

    #[tokio::test]
    async fn test_store_magic_link_reject_if_token_already_exists() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteMagicLinkRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Store a first magic link
        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let magic_link = MagicLink::new(
            UserId::test_default(),
            MagicToken::from("test_token".to_string()),
            expire_at,
        );

        repository
            .store_magic_link(&magic_link)
            .await
            .expect("store_magic_link should have succeeded");

        // Store a second magic link with the same token value
        let another_expire_at = Utc::now() + TimeDelta::minutes(5);
        let magic_link = MagicLink::new(
            UserId::from("another_user".to_string()),
            MagicToken::from("test_token".to_string()),
            another_expire_at,
        );

        // Store operation should fail
        let _ = repository
            .store_magic_link(&magic_link)
            .await
            .expect_err("Should have returned an err");

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_magic_link;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 1);

        // Existing token should not be changed
        let res: (String, String, DateTime<Utc>) =
            sqlx::query_as("select user, token, expire_at from t_magic_link limit 1;")
                .fetch_one(&repository.pool)
                .await
                .unwrap();

        assert_eq!(res.0, UserId::test_default().to_string());
        assert_eq!(res.1, "test_token".to_string());
        assert_eq!(res.2, expire_at);
    }

    #[tokio::test]
    async fn test_get_all_tokens_empty() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteMagicLinkRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert!(repository.get_all_magic_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_tokens() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteMagicLinkRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        repository
            .store_magic_link(&MagicLink::new(
                UserId::test_default(),
                MagicToken::from("a_token".to_string()),
                expire_at,
            ))
            .await
            .expect("store_magic_link should have succeeded");

        repository
            .store_magic_link(&MagicLink::new(
                UserId::test_default(),
                MagicToken::from("another_token".to_string()),
                expire_at,
            ))
            .await
            .expect("store_magic_link should have succeeded");

        let res = repository.get_all_magic_links().await;

        assert_eq!(res.len(), 2);

        let first_token = res.first().unwrap();
        assert_eq!(first_token.user(), &UserId::test_default());
        assert!(
            first_token
                .token()
                .match_token_secure(&MagicToken::from("a_token".to_string()))
        );
        assert_eq!(first_token.expire_at(), &expire_at);
    }

    #[tokio::test]
    async fn test_delete_magic_link_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteMagicLinkRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let magic_link = MagicLink::new(
            UserId::test_default(),
            MagicToken::from("test_token".to_string()),
            expire_at,
        );

        repository
            .store_magic_link(&magic_link)
            .await
            .expect("store_magic_link should have succeeded");

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_magic_link;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 1);

        // Delete operation
        repository
            .delete_magic_link_by_token(&MagicToken::from("test_token".to_string()))
            .await
            .unwrap();

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_magic_link;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 0);
    }

    #[tokio::test]
    async fn test_delete_magic_link_ok_when_token_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteMagicLinkRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Delete operation
        repository
            .delete_magic_link_by_token(&MagicToken::from("test_token".to_string()))
            .await
            .unwrap();

        let n_rows: u64 = sqlx::query_scalar("select count(user) from t_magic_link;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(n_rows, 0);
    }
}
