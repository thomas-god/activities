//! Database backup service.
//!
//! Produces a consistent, zip-packaged snapshot of the application's source-of-truth SQLite
//! databases. See the [database backups ADR](../../adr/20261016-database-backups.md) for the
//! rationale behind the design.

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::{
    config::BackupConfig,
    domain::ports::IClock,
    outbound::sqlite::backup::{sqlite_version, vacuum_into},
};

const MANIFEST_FILE_NAME: &str = "manifest.json";
const STAGING_DIR_PREFIX: &str = ".staging-";
const ARCHIVE_PREFIX: &str = "backup-";
const ARCHIVE_SUFFIX: &str = ".zip";
const ARCHIVE_TIMESTAMP_FORMAT: &str = "%Y%m%dT%H%M%S%.3fZ";

#[derive(Debug, Serialize)]
struct BackupManifest {
    created_at: String,
    app_version: String,
    sqlite_version: String,
    databases: Vec<BackupManifestEntry>,
}

#[derive(Debug, Serialize)]
struct BackupManifestEntry {
    name: String,
    size_bytes: u64,
}

/// Produces database backup archives.
///
/// A single "run" snapshots every configured database into a fresh staging directory, writes a
/// manifest, and packages everything into a `backup-<timestamp>.zip` archive. Runs are independent
/// and may safely overlap: each uses its own staging directory and archive name, and SQLite allows
/// concurrent read snapshots of a live database.
#[derive(Debug, Clone)]
pub struct BackupService<C> {
    config: BackupConfig,
    clock: C,
    shutdown: CancellationToken,
}

impl<C: IClock> BackupService<C> {
    pub fn new(config: BackupConfig, clock: C, shutdown: CancellationToken) -> Self {
        Self {
            config,
            clock,
            shutdown,
        }
    }

    pub fn config(&self) -> &BackupConfig {
        &self.config
    }

    #[tracing::instrument(skip_all)]
    pub async fn run(&self) {
        let schedule = match self.config.schedule() {
            Ok(Some(schedule)) => schedule,
            Ok(None) => {
                tracing::debug!("Database backups are disabled: no cron configured");
                return;
            }
            Err(err) => {
                tracing::error!("Invalid database backup cron expression: {err}");
                return;
            }
        };

        loop {
            let now = self.clock.now();
            let Some(next) = schedule.after(&now).next() else {
                tracing::warn!(
                    "Database backup schedule has no upcoming occurrence, stopping automatic backups"
                );
                return;
            };

            let delay = (next - now).to_std().unwrap_or(std::time::Duration::ZERO);
            tracing::info!("Next database backup scheduled at {next}");

            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = self.shutdown.cancelled() => return,
            }

            let _ = self.run_once().await;
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn run_once(&self) -> anyhow::Result<PathBuf> {
        let started = std::time::Instant::now();

        let result = async {
            let destination_dir = PathBuf::from(&self.config.backup_path);
            tokio::fs::create_dir_all(&destination_dir)
                .await
                .with_context(|| {
                    format!(
                        "Unable to create backup directory `{}`",
                        destination_dir.display()
                    )
                })?;

            self.run_once_inner(&destination_dir).await
        }
        .await;

        match &result {
            Ok(path) => tracing::info!(
                "Database backup written to {} in {:.2?}",
                path.display(),
                started.elapsed()
            ),
            Err(err) => tracing::error!(
                "Database backup failed after {:.2?}: {err}",
                started.elapsed()
            ),
        }

        result
    }

    async fn run_once_inner(&self, destination_dir: &Path) -> anyhow::Result<PathBuf> {
        let timestamp = self.clock.now();
        let source_dir = self.config.databases_path();

        let staging_dir =
            destination_dir.join(format!("{STAGING_DIR_PREFIX}{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&staging_dir)
            .await
            .with_context(|| {
                format!(
                    "Unable to create backup staging directory `{}`",
                    staging_dir.display()
                )
            })?;

        let archive_name = format!(
            "{ARCHIVE_PREFIX}{}{ARCHIVE_SUFFIX}",
            timestamp.format(ARCHIVE_TIMESTAMP_FORMAT)
        );
        let archive_path = destination_dir.join(&archive_name);

        let result = self
            .snapshot_and_archive(
                &source_dir,
                &staging_dir,
                destination_dir,
                &archive_name,
                timestamp,
            )
            .await;

        // Always clean up the staging directory, whether the run succeeded or not.
        if let Err(err) = tokio::fs::remove_dir_all(&staging_dir).await {
            tracing::warn!(
                "Unable to clean up backup staging directory `{}`: {err}",
                staging_dir.display()
            );
        }

        result.map(|()| archive_path)
    }

    async fn snapshot_and_archive(
        &self,
        source_dir: &Path,
        staging_dir: &Path,
        destination_dir: &Path,
        archive_name: &str,
        timestamp: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut entries = Vec::new();

        for database in self.config.databases() {
            let source = source_dir.join(database);
            if !tokio::fs::try_exists(&source).await.unwrap_or(false) {
                tracing::warn!(
                    "Skipping backup of `{}`: database not found",
                    source.display()
                );
                continue;
            }

            let snapshot = staging_dir.join(database);
            vacuum_into(&source, &snapshot).await?;

            let size_bytes = tokio::fs::metadata(&snapshot).await?.len();
            entries.push(BackupManifestEntry {
                name: database.to_string(),
                size_bytes,
            });
        }

        if entries.is_empty() {
            return Err(anyhow!(
                "No database found to back up in `{}`",
                source_dir.display()
            ));
        }

        let sqlite_version = sqlite_version(&staging_dir.join(&entries[0].name)).await?;
        let manifest = BackupManifest {
            created_at: timestamp.to_rfc3339(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            sqlite_version,
            databases: entries,
        };
        let manifest_json = serde_json::to_vec_pretty(&manifest)?;
        tokio::fs::write(staging_dir.join(MANIFEST_FILE_NAME), manifest_json).await?;

        // Unique temp name: concurrent runs must never write to the same partial archive.
        let temp_archive =
            destination_dir.join(format!(".backup-{}.zip.tmp", uuid::Uuid::new_v4()));
        let staging = staging_dir.to_path_buf();
        let temp = temp_archive.clone();
        tokio::task::spawn_blocking(move || zip_directory(&staging, &temp))
            .await
            .context("Backup archive task panicked")??;

        tokio::fs::rename(&temp_archive, destination_dir.join(archive_name))
            .await
            .with_context(|| format!("Unable to finalize backup archive `{archive_name}`"))?;

        // Pruning is best-effort: the archive is already safely written, so a retention failure
        // must not report the whole run as failed.
        if let Err(err) = self.prune_old_archives(destination_dir).await {
            tracing::warn!("Unable to prune old backups: {err}");
        }

        Ok(())
    }

    /// Removes the oldest archives, keeping at most `retention` of them. A retention of `0` keeps
    /// every archive.
    async fn prune_old_archives(&self, destination_dir: &Path) -> anyhow::Result<()> {
        if self.config.retention == 0 {
            return Ok(());
        }

        let mut archives = Vec::new();
        let mut entries = tokio::fs::read_dir(destination_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            if is_archive_name(&name) {
                archives.push(name);
            }
        }

        archives.sort();
        let excess = archives.len().saturating_sub(self.config.retention);

        for name in archives.into_iter().take(excess) {
            let path = destination_dir.join(&name);
            if let Err(err) = tokio::fs::remove_file(&path).await {
                tracing::warn!("Unable to remove old backup `{}`: {err}", path.display());
            }
        }

        Ok(())
    }
}

fn is_archive_name(name: &str) -> bool {
    name.starts_with(ARCHIVE_PREFIX) && name.ends_with(ARCHIVE_SUFFIX)
}

/// Zips every file of `source_dir` into `archive_path`.
///
/// Blocking, and meant to be run through [`tokio::task::spawn_blocking`].
fn zip_directory(source_dir: &Path, archive_path: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(archive_path)
        .with_context(|| format!("Unable to create archive `{}`", archive_path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut paths: Vec<PathBuf> = std::fs::read_dir(source_dir)
        .with_context(|| {
            format!(
                "Unable to read staging directory `{}`",
                source_dir.display()
            )
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    paths.sort();

    for path in paths {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        zip.start_file(name, options)?;
        let content = std::fs::read(&path)?;
        zip.write_all(&content)?;
    }

    zip.finish()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tempfile::TempDir;
    use zip::ZipArchive;

    use super::*;
    use crate::clock::clock_test_utils::FakeClock;

    async fn create_database(path: &Path) {
        tokio::fs::create_dir_all(path.parent().unwrap())
            .await
            .unwrap();

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
        sqlx::query("INSERT INTO t_values (value) VALUES (1);")
            .execute(&pool)
            .await
            .unwrap();

        pool.close().await;
    }

    fn config(data_path: &Path, retention: usize, include_search: bool) -> BackupConfig {
        BackupConfig {
            data_path: data_path.to_string_lossy().to_string(),
            backup_path: data_path.join("backups").to_string_lossy().to_string(),
            retention,
            include_search,
            cron: None,
        }
    }

    fn service(config: BackupConfig, timestamp: DateTime<Utc>) -> BackupService<FakeClock> {
        BackupService::new(config, FakeClock::new(timestamp), CancellationToken::new())
    }

    async fn archive_names(destination_dir: &Path) -> Vec<String> {
        let mut names = Vec::new();
        let mut entries = tokio::fs::read_dir(destination_dir).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            if is_archive_name(&name) {
                names.push(name);
            }
        }
        names.sort();
        names
    }

    fn archive_entries(archive: &Path) -> Vec<String> {
        let file = std::fs::File::open(archive).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        (0..zip.len())
            .map(|index| zip.by_index(index).unwrap().name().unwrap().to_string())
            .collect()
    }

    fn read_manifest(archive: &Path) -> serde_json::Value {
        let file = std::fs::File::open(archive).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let mut manifest = zip.by_name(MANIFEST_FILE_NAME).unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut manifest, &mut content).unwrap();
        serde_json::from_str(&content).unwrap()
    }

    #[tokio::test]
    async fn test_run_once_writes_an_archive_with_databases_and_manifest() {
        let tmp = TempDir::new().unwrap();
        create_database(&tmp.path().join("db/activities.db")).await;
        let config = config(tmp.path(), 7, false);
        let service = service(config, Utc::now());

        let archive = service.run_once().await.unwrap();

        assert!(archive.exists());
        let entries = archive_entries(&archive);
        assert!(entries.contains(&"activities.db".to_string()));
        assert!(entries.contains(&MANIFEST_FILE_NAME.to_string()));

        let manifest = read_manifest(&archive);
        assert_eq!(manifest["databases"][0]["name"], "activities.db");
        assert!(manifest["databases"][0]["size_bytes"].as_u64().unwrap() > 0);
        assert!(!manifest["sqlite_version"].as_str().unwrap().is_empty());
        assert!(!manifest["app_version"].as_str().unwrap().is_empty());
        assert!(!manifest["created_at"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_run_once_skips_missing_databases() {
        let tmp = TempDir::new().unwrap();
        create_database(&tmp.path().join("db/activities.db")).await;
        let config = config(tmp.path(), 7, false);
        let service = service(config, Utc::now());

        let archive = service.run_once().await.unwrap();

        let entries = archive_entries(&archive);
        assert!(entries.contains(&"activities.db".to_string()));
        assert!(!entries.contains(&"user.db".to_string()));
    }

    #[tokio::test]
    async fn test_run_once_excludes_search_by_default() {
        let tmp = TempDir::new().unwrap();
        create_database(&tmp.path().join("db/activities.db")).await;
        create_database(&tmp.path().join("db/search.db")).await;
        let config = config(tmp.path(), 7, false);
        let service = service(config, Utc::now());

        let archive = service.run_once().await.unwrap();

        assert!(!archive_entries(&archive).contains(&"search.db".to_string()));
    }

    #[tokio::test]
    async fn test_run_once_includes_search_when_opt_in() {
        let tmp = TempDir::new().unwrap();
        create_database(&tmp.path().join("db/activities.db")).await;
        create_database(&tmp.path().join("db/search.db")).await;
        let config = config(tmp.path(), 7, true);
        let service = service(config, Utc::now());

        let archive = service.run_once().await.unwrap();

        assert!(archive_entries(&archive).contains(&"search.db".to_string()));
    }

    #[tokio::test]
    async fn test_run_once_fails_when_no_database_exists() {
        let tmp = TempDir::new().unwrap();
        let config = config(tmp.path(), 7, false);
        let service = service(config, Utc::now());

        assert!(service.run_once().await.is_err());
    }

    #[tokio::test]
    async fn test_retention_keeps_only_the_most_recent_archives() {
        let tmp = TempDir::new().unwrap();
        create_database(&tmp.path().join("db/activities.db")).await;
        let config = config(tmp.path(), 2, false);
        let destination = PathBuf::from(&config.backup_path);
        let start = Utc::now();

        let first = service(config.clone(), start).run_once().await.unwrap();
        let second = service(config.clone(), start + chrono::Duration::seconds(1))
            .run_once()
            .await
            .unwrap();
        let third = service(config.clone(), start + chrono::Duration::seconds(2))
            .run_once()
            .await
            .unwrap();

        let remaining = archive_names(&destination).await;
        assert_eq!(remaining.len(), 2);
        assert!(!first.exists());
        assert!(second.exists());
        assert!(third.exists());
    }

    #[tokio::test]
    async fn test_retention_zero_keeps_every_archive() {
        let tmp = TempDir::new().unwrap();
        create_database(&tmp.path().join("db/activities.db")).await;
        let config = config(tmp.path(), 0, false);
        let destination = PathBuf::from(&config.backup_path);
        let start = Utc::now();

        service(config.clone(), start).run_once().await.unwrap();
        service(config.clone(), start + chrono::Duration::seconds(1))
            .run_once()
            .await
            .unwrap();

        assert_eq!(archive_names(&destination).await.len(), 2);
    }

    #[tokio::test]
    async fn test_concurrent_runs_do_not_interfere() {
        let tmp = TempDir::new().unwrap();
        create_database(&tmp.path().join("db/activities.db")).await;
        let config = config(tmp.path(), 7, false);
        let destination = PathBuf::from(&config.backup_path);
        let start = Utc::now();

        let first = service(config.clone(), start);
        let second = service(config.clone(), start + chrono::Duration::seconds(1));

        let (first, second) = tokio::join!(first.run_once(), second.run_once());

        assert!(first.is_ok());
        assert!(second.is_ok());
        assert_ne!(first.unwrap(), second.unwrap());
        assert_eq!(archive_names(&destination).await.len(), 2);
    }

    #[tokio::test]
    async fn test_run_does_not_leave_staging_directories_behind() {
        let tmp = TempDir::new().unwrap();
        create_database(&tmp.path().join("db/activities.db")).await;
        let config = config(tmp.path(), 7, false);
        let destination = PathBuf::from(&config.backup_path);
        let service = service(config, Utc::now());

        service.run_once().await.unwrap();

        let mut entries = tokio::fs::read_dir(&destination).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            assert!(
                !name.starts_with(STAGING_DIR_PREFIX),
                "staging directory `{name}` was not cleaned up"
            );
        }
    }

    #[tokio::test]
    async fn test_run_returns_immediately_without_cron() {
        let tmp = TempDir::new().unwrap();
        let service = service(config(tmp.path(), 7, false), Utc::now());

        tokio::time::timeout(std::time::Duration::from_secs(1), service.run())
            .await
            .expect("run should return immediately when no cron is configured");
    }

    #[tokio::test]
    async fn test_run_stops_on_shutdown() {
        let tmp = TempDir::new().unwrap();
        let mut config = config(tmp.path(), 7, false);
        config.cron = Some("0 0 3 * * *".to_string());
        let shutdown = CancellationToken::new();
        let service = BackupService::new(config, FakeClock::new(Utc::now()), shutdown.clone());

        let handle = tokio::spawn(async move { service.run().await });
        tokio::task::yield_now().await;
        shutdown.cancel();

        tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("run should stop when shutdown is cancelled")
            .unwrap();
    }
}
