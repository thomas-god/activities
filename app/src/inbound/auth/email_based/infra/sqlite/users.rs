use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};

use crate::{
    domain::models::UserId,
    inbound::auth::email_based::{
        EmailAddress, HashedAuthLink, HashedAuthToken,
        auth_link::{AuthLinkRepository, AuthLinkRepositoryError},
        user::UserRepository,
    },
};

/// SQLite repository backed by the user database (`user.db`).
///
/// It implements both [`UserRepository`] and [`AuthLinkRepository`] domain traits.
#[derive(Debug, Clone)]
pub struct SqliteUsersRepository {
    pool: SqlitePool,
}

impl SqliteUsersRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migrations
        sqlx::migrate!("migrations/auth/users").run(&pool).await?;

        Ok(Self { pool })
    }
}

impl UserRepository for SqliteUsersRepository {
    #[tracing::instrument(skip_all, err(Debug))]
    async fn store_user_with_mail(&self, user: &UserId, email: &EmailAddress) -> Result<(), ()> {
        match sqlx::query(
            r#"
        INSERT INTO t_users (email, user_id) VALUES (
            ?1, ?2
        );"#,
        )
        .bind(email.to_string())
        .bind(user.to_string())
        .execute(&self.pool)
        .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::warn!("Unable to insert user into database");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn get_user_by_email(&self, email: &EmailAddress) -> Result<Option<UserId>, ()> {
        let res: Result<String, sqlx::Error> =
            sqlx::query_scalar("SELECT user_id FROM t_users WHERE email = ?1")
                .bind(email.to_string())
                .fetch_one(&self.pool)
                .await;
        match res {
            Ok(res) => Ok(Some(UserId::from(res))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => {
                tracing::warn!("Cannot fetch users from database");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }
}

impl AuthLinkRepository for SqliteUsersRepository {
    #[tracing::instrument(skip_all, err)]
    async fn store_auth_link(&self, link: &HashedAuthLink) -> Result<(), AuthLinkRepositoryError> {
        let res = sqlx::query(
            r#"
        INSERT INTO t_auth_links (user, token_hash, expire_at)
        SELECT rowid, ?2, ?3 FROM t_users WHERE user_id = ?1
        ;"#,
        )
        .bind(link.user().to_string())
        .bind(link.hash().to_string())
        .bind(link.expire_at())
        .execute(&self.pool)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to insert auth link into database");
            tracing::warn!("{}", err);
            AuthLinkRepositoryError::Error
        })?;

        if res.rows_affected() == 0 {
            tracing::warn!("Unable to insert auth link into database: unknown user");
            return Err(AuthLinkRepositoryError::Error);
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn get_all_auth_links(&self) -> Vec<HashedAuthLink> {
        let res: Vec<(String, String, DateTime<Utc>)> = match sqlx::query_as(
            r#"
        SELECT u.user_id, a.token_hash, a.expire_at
        FROM t_auth_links a
        JOIN t_users u ON u.rowid = a.user
        ;"#,
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(res) => res,
            Err(err) => {
                tracing::warn!("Cannot fetch auth links from database");
                tracing::warn!("{}", err);
                return Vec::new();
            }
        };
        res.iter()
            .map(|(user, token, expire_at)| {
                HashedAuthLink::new(
                    UserId::from(user.clone()),
                    HashedAuthToken::new(token.clone()),
                    *expire_at,
                )
            })
            .collect()
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_auth_link_by_hash(
        &self,
        hash: &HashedAuthToken,
    ) -> Result<(), AuthLinkRepositoryError> {
        sqlx::query("DELETE FROM t_auth_links WHERE token_hash = ?1;")
            .bind(hash.to_string())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|_| AuthLinkRepositoryError::Error)
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_expired_auth_links(
        &self,
        reference: DateTime<Utc>,
    ) -> Result<(), AuthLinkRepositoryError> {
        sqlx::query("DELETE FROM t_auth_links WHERE expire_at <= ?1;")
            .bind(reference)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|_| AuthLinkRepositoryError::Error)
    }
}

#[cfg(test)]
mod test_sqlite_users_repository {
    use chrono::{TimeDelta, Utc};
    use tempfile::NamedTempFile;

    use crate::{
        domain::models::UserId,
        inbound::auth::email_based::{
            AuthLink, AuthToken, EmailAddress, auth_link::AuthLinkRepository, user::UserRepository,
        },
    };

    use super::*;

    fn test_email() -> EmailAddress {
        EmailAddress::try_from("test@mail.test").unwrap()
    }

    /// Creates a repository and registers a single test user in it.
    async fn repository_with_test_user() -> (NamedTempFile, SqliteUsersRepository) {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        repository
            .store_user_with_mail(&UserId::test_default(), &test_email())
            .await
            .expect("should have stored user with email");

        (db_file, repository)
    }

    #[tokio::test]
    async fn test_init_tables() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        sqlx::query("select count(*) from t_users;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        sqlx::query("select count(*) from t_auth_links;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_store_user_with_email() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        repository
            .store_user_with_mail(
                &UserId::test_default(),
                &EmailAddress::try_from("test@mail.test").unwrap(),
            )
            .await
            .expect("should have store user with email");
    }

    #[tokio::test]
    async fn test_store_user_with_email_reject_if_emails_exists() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        repository
            .store_user_with_mail(
                &UserId::test_default(),
                &EmailAddress::try_from("test@mail.test").unwrap(),
            )
            .await
            .unwrap();

        repository
            .store_user_with_mail(
                &UserId::from("another_user".to_string()),
                &EmailAddress::try_from("test@mail.test").unwrap(),
            )
            .await
            .expect_err("Should have rejected duplicated email");
    }

    #[tokio::test]
    async fn test_get_user_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert!(
            repository
                .get_user_by_email(&EmailAddress::try_from("test@mail.test").unwrap())
                .await
                .expect("Should not have err")
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_get_user_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        repository
            .store_user_with_mail(
                &UserId::test_default(),
                &EmailAddress::try_from("test@mail.test").unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            repository
                .get_user_by_email(&EmailAddress::try_from("test@mail.test").unwrap())
                .await
                .expect("Should not have err"),
            Some(UserId::test_default())
        );
    }

    #[tokio::test]
    async fn test_store_auth_link() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let hashed_auth_link = AuthLink::new(
            UserId::test_default(),
            AuthToken::from("test_token".to_string()),
            expire_at,
        )
        .as_hash()
        .unwrap();

        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect("store_auth_link should have succeeded");

        let links = repository.get_all_auth_links().await;
        assert_eq!(links.len(), 1);

        let link = links.first().unwrap();
        assert_eq!(link.user(), &UserId::test_default());
        assert_eq!(link.hash(), hashed_auth_link.hash());
        assert_eq!(link.expire_at(), &expire_at);
    }

    #[tokio::test]
    async fn test_store_auth_link_token_for_that_user_already_exists() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);

        // Store a first auth link
        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    AuthToken::from("test_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        // Store a second auth link, for the same user, different token
        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    AuthToken::from("another_test_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        assert_eq!(repository.get_all_auth_links().await.len(), 2);
    }

    #[tokio::test]
    async fn test_store_auth_link_reject_if_token_already_exists() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_auth_link = AuthLink::new(
            UserId::test_default(),
            AuthToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash()
        .unwrap();

        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect("store_auth_link should have succeeded");

        // Store operation should fail for same hash
        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect_err("Should have returned an err");

        assert_eq!(repository.get_all_auth_links().await.len(), 1);

        // Existing token should not be changed
        let link = repository
            .get_all_auth_links()
            .await
            .first()
            .unwrap()
            .clone();
        assert_eq!(link.hash(), hashed_auth_link.hash());
    }

    #[tokio::test]
    async fn test_store_auth_link_reject_if_user_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let hashed_auth_link = AuthLink::new(
            UserId::test_default(),
            AuthToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash()
        .unwrap();

        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect_err("Should have rejected auth link for unknown user");

        assert!(repository.get_all_auth_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_deleting_user_cascades_to_auth_links() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    AuthToken::from("test_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        sqlx::query("DELETE FROM t_users WHERE user_id = ?1;")
            .bind(UserId::test_default().to_string())
            .execute(&repository.pool)
            .await
            .unwrap();

        assert!(repository.get_all_auth_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_tokens_empty() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert!(repository.get_all_auth_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_tokens() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    AuthToken::from("a_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        repository
            .store_auth_link(
                &AuthLink::new(
                    UserId::test_default(),
                    AuthToken::from("another_token".to_string()),
                    expire_at,
                )
                .as_hash()
                .unwrap(),
            )
            .await
            .expect("store_auth_link should have succeeded");

        let res = repository.get_all_auth_links().await;

        assert_eq!(res.len(), 2);

        let first_token = res.first().unwrap();
        assert_eq!(first_token.user(), &UserId::test_default());
        assert!(
            first_token
                .hash()
                .verify_token(&AuthToken::from("a_token".to_string()))
        );
        assert_eq!(first_token.expire_at(), &expire_at);
    }

    #[tokio::test]
    async fn test_delete_auth_link_ok() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_auth_link = AuthLink::new(
            UserId::test_default(),
            AuthToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash()
        .unwrap();

        repository
            .store_auth_link(&hashed_auth_link)
            .await
            .expect("store_auth_link should have succeeded");

        assert_eq!(repository.get_all_auth_links().await.len(), 1);

        // Delete operation
        repository
            .delete_auth_link_by_hash(hashed_auth_link.hash())
            .await
            .unwrap();

        assert!(repository.get_all_auth_links().await.is_empty());
    }

    #[tokio::test]
    async fn test_delete_expired_auth_links_only_deletes_expired_ones() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expired = AuthLink::new(
            UserId::test_default(),
            AuthToken::from("expired_token".to_string()),
            Utc::now() - TimeDelta::minutes(5),
        )
        .as_hash()
        .unwrap();
        let still_valid = AuthLink::new(
            UserId::test_default(),
            AuthToken::from("valid_token".to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash()
        .unwrap();
        repository.store_auth_link(&expired).await.unwrap();
        repository.store_auth_link(&still_valid).await.unwrap();

        repository
            .delete_expired_auth_links(Utc::now())
            .await
            .unwrap();

        let res = repository.get_all_auth_links().await;
        assert_eq!(res.len(), 1);
        assert_eq!(res.first().unwrap().hash(), still_valid.hash());
    }

    #[tokio::test]
    async fn test_delete_auth_link_ok_when_token_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Delete operation
        repository
            .delete_auth_link_by_hash(&HashedAuthToken::new("test_token".to_string()))
            .await
            .unwrap();

        assert!(repository.get_all_auth_links().await.is_empty());
    }
}
