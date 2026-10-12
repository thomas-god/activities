use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{
    ConnectOptions, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::{
    domain::models::UserId,
    inbound::auth::multi_user::{
        EmailAddress, HashedAuthLink, HashedAuthToken, HashedPassword, HashedSession,
        HashedSessionToken,
        auth_link::{AuthLinkRepository, AuthLinkRepositoryError},
        service::UserRepository,
        session::SessionRepository,
    },
};

/// SQLite repository backed by the user database (`user.db`).
///
/// It implements the [`UserRepository`], [`AuthLinkRepository`] and
/// [`SessionRepository`] domain traits.
#[derive(Debug, Clone)]
pub struct SqliteUsersRepository {
    writer: SqlitePool,
    readers: SqlitePool,
}

impl SqliteUsersRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let writer_options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .log_slow_statements(
                log::LevelFilter::Warn,
                std::time::Duration::from_millis(100),
            )
            .journal_mode(SqliteJournalMode::Wal);

        let writer = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(writer_options)
            .await?;

        // Run migrations
        sqlx::migrate!("migrations/users").run(&writer).await?;

        let readers_options = SqliteConnectOptions::from_str(url)?
            .journal_mode(SqliteJournalMode::Wal)
            .read_only(true);
        let readers = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(readers_options)
            .await?;

        Ok(Self { writer, readers })
    }
}

impl UserRepository for SqliteUsersRepository {
    #[tracing::instrument(skip_all, err(Debug))]
    async fn store_user(
        &self,
        user: &UserId,
        email: &EmailAddress,
        password_hash: Option<HashedPassword>,
    ) -> Result<(), ()> {
        let mut tx = match self.writer.begin().await {
            Ok(tx) => tx,
            Err(err) => {
                tracing::warn!("Unable to start transaction to store user");
                tracing::warn!("{}", err);
                return Err(());
            }
        };

        if let Err(err) =
            sqlx::query("INSERT INTO t_users (user_id, password_hash) VALUES (?1, ?2);")
                .bind(user.to_string())
                .bind(password_hash.map(|hash| hash.to_string()))
                .execute(&mut *tx)
                .await
        {
            tracing::warn!("Unable to insert user into database");
            tracing::warn!("{}", err);
            return Err(());
        }

        if let Err(err) = sqlx::query(
            r#"
        INSERT INTO t_emails (email, user) VALUES (
            ?1, (SELECT rowid FROM t_users WHERE user_id = ?2)
        );"#,
        )
        .bind(email.to_string())
        .bind(user.to_string())
        .execute(&mut *tx)
        .await
        {
            tracing::warn!("Unable to insert user email into database");
            tracing::warn!("{}", err);
            return Err(());
        }

        match tx.commit().await {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::warn!("Unable to commit user storing transaction");
                tracing::warn!("{}", err);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn get_user_by_email(
        &self,
        email: &EmailAddress,
    ) -> Result<Option<(UserId, Option<HashedPassword>)>, ()> {
        let res: Result<(String, Option<String>), sqlx::Error> = sqlx::query_as(
            r#"
        SELECT u.user_id, u.password_hash
        FROM t_users u
        JOIN t_emails e ON e.user = u.rowid
        WHERE e.email = ?1
        ;"#,
        )
        .bind(email.to_string())
        .fetch_one(&self.readers)
        .await;
        match res {
            Ok((user_id, password_hash)) => Ok(Some((
                UserId::from(user_id),
                password_hash.map(HashedPassword::new),
            ))),
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
        .execute(&self.writer)
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
        .fetch_all(&self.readers)
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
            .execute(&self.writer)
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
            .execute(&self.writer)
            .await
            .map(|_| ())
            .map_err(|_| AuthLinkRepositoryError::Error)
    }
}

impl SessionRepository for SqliteUsersRepository {
    #[tracing::instrument(skip_all, err(Debug))]
    async fn store_session(&self, session: &HashedSession) -> Result<(), ()> {
        let res = sqlx::query(
            r#"
        INSERT INTO t_sessions (user, token_hash, expire_at)
        SELECT rowid, ?2, ?3 FROM t_users WHERE user_id = ?1
        ;"#,
        )
        .bind(session.user().to_string())
        .bind(session.hash().to_string())
        .bind(session.expire_at())
        .execute(&self.writer)
        .await
        .map_err(|err| {
            tracing::warn!("Unable to insert session into database");
            tracing::warn!("{}", err);
        })?;

        if res.rows_affected() == 0 {
            tracing::warn!("Unable to insert session into database: unknown user");
            return Err(());
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn get_session_by_hash(&self, hash: &HashedSessionToken) -> Option<HashedSession> {
        let res: Option<(String, String, DateTime<Utc>)> = match sqlx::query_as(
            r#"
        SELECT u.user_id, s.token_hash, s.expire_at
        FROM t_sessions s
        JOIN t_users u ON u.rowid = s.user
        WHERE s.token_hash = ?1
        ;"#,
        )
        .bind(hash.to_string())
        .fetch_optional(&self.readers)
        .await
        {
            Ok(res) => res,
            Err(err) => {
                tracing::warn!("Cannot fetch session from database");
                tracing::warn!("{}", err);
                return None;
            }
        };
        res.map(|(user, token, expire_at)| {
            HashedSession::new(
                UserId::from(user),
                HashedSessionToken::new(token),
                expire_at,
            )
        })
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn delete_session_by_hash(&self, hash: &HashedSessionToken) -> Result<(), ()> {
        sqlx::query("DELETE FROM t_sessions WHERE token_hash = ?1;")
            .bind(hash.to_string())
            .execute(&self.writer)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }

    #[tracing::instrument(skip_all, err(Debug))]
    async fn delete_expired_sessions(&self, reference: DateTime<Utc>) -> Result<(), ()> {
        sqlx::query("DELETE FROM t_sessions WHERE expire_at <= ?1;")
            .bind(reference)
            .execute(&self.writer)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }
}

#[cfg(test)]
mod test_sqlite_users_repository {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    use chrono::{TimeDelta, Utc};
    use tempfile::NamedTempFile;

    use crate::{
        domain::models::UserId,
        inbound::auth::multi_user::{
            AuthLink, AuthToken, EmailAddress, HashedSessionToken, Password, Session, SessionToken,
            auth_link::AuthLinkRepository, service::UserRepository, session::SessionRepository,
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
            .store_user(&UserId::test_default(), &test_email(), None)
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
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        sqlx::query("select count(*) from t_auth_links;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        sqlx::query("select count(*) from t_sessions;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        sqlx::query("select count(*) from t_emails;")
            .fetch_one(&repository.readers)
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
            .store_user(
                &UserId::test_default(),
                &EmailAddress::try_from("test@mail.test").unwrap(),
                None,
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
            .store_user(
                &UserId::test_default(),
                &EmailAddress::try_from("test@mail.test").unwrap(),
                None,
            )
            .await
            .unwrap();

        repository
            .store_user(
                &UserId::from("another_user".to_string()),
                &EmailAddress::try_from("test@mail.test").unwrap(),
                None,
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
            .store_user(
                &UserId::test_default(),
                &EmailAddress::try_from("test@mail.test").unwrap(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            repository
                .get_user_by_email(&EmailAddress::try_from("test@mail.test").unwrap())
                .await
                .expect("Should not have err"),
            Some((UserId::test_default(), None))
        );
    }

    #[tokio::test]
    async fn test_get_user_returns_stored_password_hash() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let password = Password::try_from("secret-password".to_string()).unwrap();
        let password_hash = password.as_hash().expect("hashing should succeed");
        repository
            .store_user(
                &UserId::test_default(),
                &test_email(),
                Some(password_hash.clone()),
            )
            .await
            .unwrap();

        let (user, stored_hash) = repository
            .get_user_by_email(&test_email())
            .await
            .expect("should not have err")
            .expect("user should exist");

        assert_eq!(user, UserId::test_default());
        let stored_hash = stored_hash.expect("password hash should be returned");
        assert_eq!(stored_hash, password_hash);
        assert!(stored_hash.verify_password(&password));
    }

    #[tokio::test]
    async fn test_stored_email_is_unverified() {
        let (_db_file, repository) = repository_with_test_user().await;

        let verified: i64 = sqlx::query_scalar("select verified from t_emails;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert_eq!(verified, 0);
    }

    #[tokio::test]
    async fn test_user_has_no_password_hash_by_default() {
        let (_db_file, repository) = repository_with_test_user().await;

        let password_hash: Option<String> =
            sqlx::query_scalar("select password_hash from t_users;")
                .fetch_one(&repository.readers)
                .await
                .unwrap();
        assert_eq!(password_hash, None);
    }

    #[tokio::test]
    async fn test_stored_password_hash_can_be_verified_against_password() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let password = Password::try_from("secret-password".to_string()).unwrap();
        let password_hash = password.as_hash().expect("hashing should succeed");
        repository
            .store_user(&UserId::test_default(), &test_email(), Some(password_hash))
            .await
            .expect("should have stored user with password hash");

        let stored_hash: Option<String> = sqlx::query_scalar("select password_hash from t_users;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        let stored_hash = stored_hash.expect("password hash should be stored");
        let parsed =
            PasswordHash::new(&stored_hash).expect("stored hash should be a valid PHC string");
        Argon2::default()
            .verify_password("secret-password".as_bytes(), &parsed)
            .expect("stored hash should verify against the original password");
    }

    #[tokio::test]
    async fn test_user_can_have_multiple_emails() {
        let (_db_file, repository) = repository_with_test_user().await;

        sqlx::query(
            r#"
            INSERT INTO t_emails (email, user) VALUES (
                'other@mail.test', (SELECT rowid FROM t_users WHERE user_id = ?1)
            );"#,
        )
        .bind(UserId::test_default().to_string())
        .execute(&repository.writer)
        .await
        .unwrap();

        for mail in ["test@mail.test", "other@mail.test"] {
            assert_eq!(
                repository
                    .get_user_by_email(&EmailAddress::try_from(mail).unwrap())
                    .await
                    .expect("should not have err"),
                Some((UserId::test_default(), None))
            );
        }
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
            .execute(&repository.writer)
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

    #[tokio::test]
    async fn test_store_session() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);
        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            expire_at,
        )
        .as_hash();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        let session = repository
            .get_session_by_hash(hashed_session.hash())
            .await
            .expect("session should be found");

        assert_eq!(session.user(), &UserId::test_default());
        assert_eq!(session.hash(), hashed_session.hash());
        assert_eq!(session.expire_at(), &expire_at);
    }

    #[tokio::test]
    async fn test_store_session_token_for_that_user_already_exists() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expire_at = Utc::now() + TimeDelta::minutes(5);

        // Store a first session
        repository
            .store_session(
                &Session::new(
                    UserId::test_default(),
                    SessionToken::from("test_token".to_string()),
                    expire_at,
                )
                .as_hash(),
            )
            .await
            .expect("store_session should have succeeded");

        // Store a second session, for the same user, different token
        repository
            .store_session(
                &Session::new(
                    UserId::test_default(),
                    SessionToken::from("another_test_token".to_string()),
                    expire_at,
                )
                .as_hash(),
            )
            .await
            .expect("store_session should have succeeded");

        let session = repository
            .get_session_by_hash(&SessionToken::from("another_test_token".to_string()).as_hash())
            .await
            .expect("session should be found");
        assert_eq!(session.user(), &UserId::test_default());
    }

    #[tokio::test]
    async fn test_store_session_reject_if_token_already_exists() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        // Store operation should fail for same hash
        repository
            .store_session(&hashed_session)
            .await
            .expect_err("Should have returned an err");

        // Existing token should not be changed
        let session = repository
            .get_session_by_hash(hashed_session.hash())
            .await
            .expect("session should be found");
        assert_eq!(session.hash(), hashed_session.hash());
    }

    #[tokio::test]
    async fn test_store_session_reject_if_user_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash();

        repository
            .store_session(&hashed_session)
            .await
            .expect_err("Should have rejected session for unknown user");

        assert!(
            repository
                .get_session_by_hash(hashed_session.hash())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_deleting_user_cascades_to_sessions() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash();
        repository.store_session(&hashed_session).await.unwrap();

        sqlx::query("DELETE FROM t_users WHERE user_id = ?1;")
            .bind(UserId::test_default().to_string())
            .execute(&repository.writer)
            .await
            .unwrap();

        assert!(
            repository
                .get_session_by_hash(hashed_session.hash())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_get_session_by_hash_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert!(
            repository
                .get_session_by_hash(&HashedSessionToken::new("test_token".to_string()))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_delete_session_ok() {
        let (_db_file, repository) = repository_with_test_user().await;

        let hashed_session = Session::new(
            UserId::test_default(),
            SessionToken::from("test_token".to_string()),
            Utc::now(),
        )
        .as_hash();

        repository
            .store_session(&hashed_session)
            .await
            .expect("store_session should have succeeded");

        // Delete operation
        repository
            .delete_session_by_hash(hashed_session.hash())
            .await
            .unwrap();

        assert!(
            repository
                .get_session_by_hash(hashed_session.hash())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_delete_expired_sessions_only_deletes_expired_ones() {
        let (_db_file, repository) = repository_with_test_user().await;

        let expired = Session::new(
            UserId::test_default(),
            SessionToken::from("expired_token".to_string()),
            Utc::now() - TimeDelta::minutes(5),
        )
        .as_hash();
        let still_valid = Session::new(
            UserId::test_default(),
            SessionToken::from("valid_token".to_string()),
            Utc::now() + TimeDelta::minutes(5),
        )
        .as_hash();
        repository.store_session(&expired).await.unwrap();
        repository.store_session(&still_valid).await.unwrap();

        repository
            .delete_expired_sessions(Utc::now())
            .await
            .unwrap();

        assert!(
            repository
                .get_session_by_hash(expired.hash())
                .await
                .is_none()
        );
        assert!(
            repository
                .get_session_by_hash(still_valid.hash())
                .await
                .is_some()
        );
    }

    #[tokio::test]
    async fn test_delete_session_ok_when_token_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Delete operation
        repository
            .delete_session_by_hash(&HashedSessionToken::new("test_token".to_string()))
            .await
            .unwrap();

        assert!(
            repository
                .get_session_by_hash(&HashedSessionToken::new("test_token".to_string()))
                .await
                .is_none()
        );
    }

    /// Builds a pre-004 schema (migrations 001 to 003 applied by hand), seeds
    /// it with users, an auth link and a session, marks those migrations as
    /// applied so that initializing the repository only runs 004, and checks
    /// that the data survives the table rebuild with its foreign keys intact.
    #[tokio::test]
    async fn test_email_split_migration_preserves_data() {
        let db_file = NamedTempFile::new().unwrap();
        let migrator = sqlx::migrate!("migrations/users");

        // Apply migrations 001-003 manually to build the pre-004 schema.
        let options = SqliteConnectOptions::from_str(&db_file.path().to_string_lossy())
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePool::connect_with(options).await.unwrap();

        {
            let mut conn = pool.acquire().await.unwrap();
            for migration in migrator.migrations.iter().take(3) {
                sqlx::raw_sql(migration.sql.as_ref())
                    .execute(&mut *conn)
                    .await
                    .unwrap();
            }
        }

        // Seed pre-004 data: two users, an auth link for the first one and a
        // session for the second one (rowids are auto-assigned in order).
        sqlx::query("INSERT INTO t_users (email, user_id) VALUES ('a@mail.test', 'user_a');")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO t_users (email, user_id) VALUES ('b@mail.test', 'user_b');")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO t_auth_links (user, token_hash, expire_at)
             VALUES (1, 'link_hash', '2030-01-01T00:00:00+00:00');",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO t_sessions (user, token_hash, expire_at)
             VALUES (2, 'session_hash', '2030-01-01T00:00:00+00:00');",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Mark migrations 001-003 as applied so that the migrator run below
        // only applies 004.
        sqlx::raw_sql(
            "CREATE TABLE _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        for migration in migrator.migrations.iter().take(3) {
            sqlx::query(
                "INSERT INTO _sqlx_migrations (version, description, success, checksum,
                 execution_time) VALUES (?1, ?2, TRUE, ?3, 0);",
            )
            .bind(migration.version)
            .bind(migration.description.as_ref())
            .bind(migration.checksum.as_ref())
            .execute(&pool)
            .await
            .unwrap();
        }
        pool.close().await;

        // Initializing the repository applies 004 on the seeded data.
        let repository = SqliteUsersRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Users are preserved and resolvable through their migrated emails.
        assert_eq!(
            repository
                .get_user_by_email(&EmailAddress::try_from("a@mail.test").unwrap())
                .await
                .expect("should not have err"),
            Some((UserId::from("user_a".to_string()), None))
        );
        assert_eq!(
            repository
                .get_user_by_email(&EmailAddress::try_from("b@mail.test").unwrap())
                .await
                .expect("should not have err"),
            Some((UserId::from("user_b".to_string()), None))
        );

        // Emails of existing users are migrated as verified.
        let emails: Vec<(String, i64)> =
            sqlx::query_as("SELECT email, verified FROM t_emails ORDER BY email;")
                .fetch_all(&repository.readers)
                .await
                .unwrap();
        assert_eq!(
            emails,
            [
                ("a@mail.test".to_string(), 1),
                ("b@mail.test".to_string(), 1)
            ]
        );

        // Auth links and sessions are ephemeral: the migration drops them.
        assert!(repository.get_all_auth_links().await.is_empty());
        assert!(
            repository
                .get_session_by_hash(&HashedSessionToken::new("session_hash".to_string()))
                .await
                .is_none()
        );

        // Existing users are migrated without a password hash.
        let password_hashes: Vec<Option<String>> =
            sqlx::query_scalar("SELECT password_hash FROM t_users ORDER BY rowid;")
                .fetch_all(&repository.readers)
                .await
                .unwrap();
        assert_eq!(password_hashes, [None, None]);
    }
}
