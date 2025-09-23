use std::str::FromStr;

use sqlx::Error;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};

use crate::{
    domain::models::UserId,
    inbound::http::auth::{EmailAddress, services::user::UserRepository},
};

#[derive(Debug, Clone)]
pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migration here for now
        sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS t_users (
            email TEXT UNIQUE,
            user_id TEXT
        );"#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}

impl UserRepository for SqliteUserRepository {
    async fn store_user_with_mail(&self, user: &UserId, email: &EmailAddress) -> Result<(), ()> {
        match sqlx::query(
            r#"
        INSERT INTO t_users VALUES (
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

    async fn get_user_by_email(&self, email: &EmailAddress) -> Result<Option<UserId>, ()> {
        let res: Result<String, Error> =
            sqlx::query_scalar("SELECT user_id FROM t_users WHERE email = ?1")
                .bind(email.to_string())
                .fetch_one(&self.pool)
                .await;
        match res {
            Ok(res) => Ok(Some(UserId::from(res))),
            Err(Error::RowNotFound) => Ok(None),
            Err(err) => {
                tracing::warn!("Cannot fetch users from database");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }
}

#[cfg(test)]
mod test_sqlite_user_repository {
    use super::*;

    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_init_table() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUserRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        sqlx::query("select count(*) from t_users;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_store_user_with_email() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUserRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteUserRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteUserRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteUserRepository::new(&db_file.path().to_string_lossy())
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
}
