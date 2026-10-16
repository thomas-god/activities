//! SQLite-specific backup primitives.
//!
//! The only reliable way to back up a live SQLite database (especially in WAL mode) is to let
//! SQLite itself produce the snapshot. [`vacuum_into`] wraps [`VACUUM INTO`][vacuum-into], which
//! reads a transactionally consistent view of the database without modifying it or blocking
//! writers, and writes it as a new, self-contained database file.
//!
//! [vacuum-into]: https://www.sqlite.org/lang_vacuum.html#vacuuminto

use std::path::Path;

use anyhow::{Context, anyhow};
use sqlx::{ConnectOptions, Connection, sqlite::SqliteConnectOptions};

/// Produces a consistent, single-file snapshot of the SQLite database at `source`, written to
/// `destination`.
///
/// `destination` must not already exist (SQLite refuses to overwrite it). The produced file is
/// checked with `PRAGMA integrity_check` before this function returns.
#[tracing::instrument(skip_all, err)]
pub async fn vacuum_into(source: &Path, destination: &Path) -> anyhow::Result<()> {
    let mut source_connection = open(source)
        .await
        .with_context(|| format!("Unable to open database `{}`", source.display()))?;

    let result = sqlx::query("VACUUM INTO ?1")
        .bind(destination.to_string_lossy().to_string())
        .execute(&mut source_connection)
        .await
        .with_context(|| {
            format!(
                "Unable to snapshot `{}` into `{}`",
                source.display(),
                destination.display()
            )
        });

    close(source_connection).await;

    result?;

    integrity_check(destination).await
}

/// Returns the version of the SQLite library bundled with the application.
pub async fn sqlite_version(source: &Path) -> anyhow::Result<String> {
    let mut source_connection = open(source)
        .await
        .with_context(|| format!("Unable to open database `{}`", source.display()))?;

    let version = sqlx::query_scalar::<_, String>("SELECT sqlite_version();")
        .fetch_one(&mut source_connection)
        .await
        .with_context(|| {
            format!(
                "Unable to read the SQLite version from `{}`",
                source.display()
            )
        });

    close(source_connection).await;

    version
}

async fn open(path: &Path) -> Result<sqlx::SqliteConnection, sqlx::Error> {
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .connect()
        .await
}

/// Closes the connection, logging (but not propagating) failures: by this point the backup either
/// succeeded or already failed, so a close error must not mask the real outcome.
async fn close(connection: sqlx::SqliteConnection) {
    if let Err(err) = connection.close().await {
        tracing::warn!("Unable to close SQLite backup connection: {err}");
    }
}

async fn integrity_check(database: &Path) -> anyhow::Result<()> {
    let mut snapshot_connection = open(database)
        .await
        .with_context(|| format!("Unable to open snapshot `{}`", database.display()))?;

    let result = sqlx::query_scalar::<_, String>("PRAGMA integrity_check;")
        .fetch_all(&mut snapshot_connection)
        .await
        .with_context(|| format!("Unable to check the integrity of `{}`", database.display()));

    close(snapshot_connection).await;

    let rows = result?;

    if rows.len() != 1 || rows[0] != "ok" {
        return Err(anyhow!(
            "Integrity check failed for `{}`: {}",
            database.display(),
            rows.join("; ")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::TempDir;

    use super::*;

    async fn create_database(path: &Path, value: i64) {
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))
            .unwrap()
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE t_values (value INTEGER);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO t_values (value) VALUES (?1);")
            .bind(value)
            .execute(&pool)
            .await
            .unwrap();

        pool.close().await;
    }

    async fn read_value(path: &Path) -> i64 {
        let options =
            SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display())).unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        let value = sqlx::query_scalar::<_, i64>("SELECT value FROM t_values LIMIT 1;")
            .fetch_one(&pool)
            .await
            .unwrap();

        pool.close().await;

        value
    }

    #[tokio::test]
    async fn test_vacuum_into_produces_a_valid_consistent_copy() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source.db");
        let destination = tmp.path().join("snapshot.db");
        create_database(&source, 42).await;

        vacuum_into(&source, &destination).await.unwrap();

        assert!(destination.exists());
        assert_eq!(read_value(&destination).await, 42);
    }

    #[tokio::test]
    async fn test_vacuum_into_leaves_the_source_untouched() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source.db");
        let destination = tmp.path().join("snapshot.db");
        create_database(&source, 7).await;

        vacuum_into(&source, &destination).await.unwrap();

        assert_eq!(read_value(&source).await, 7);
    }

    #[tokio::test]
    async fn test_vacuum_into_fails_when_destination_exists() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source.db");
        let destination = tmp.path().join("snapshot.db");
        create_database(&source, 1).await;
        std::fs::write(&destination, b"already there").unwrap();

        assert!(vacuum_into(&source, &destination).await.is_err());
    }

    #[tokio::test]
    async fn test_vacuum_into_fails_when_source_missing() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("missing.db");
        let destination = tmp.path().join("snapshot.db");

        assert!(vacuum_into(&source, &destination).await.is_err());
    }

    #[tokio::test]
    async fn test_sqlite_version_is_reported() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source.db");
        create_database(&source, 1).await;

        let version = sqlite_version(&source).await.unwrap();

        assert!(!version.is_empty());
    }
}
