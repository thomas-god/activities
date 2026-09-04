use std::str::FromStr;

use anyhow::anyhow;
use sqlx::{
    ConnectOptions, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::domain::{
    models::{
        UserId,
        activity::ActivityId,
        search::{SearchDocument, SearchDocumentEvent, SearchDocumentType},
        training::TrainingNoteId,
    },
    ports::{
        IClock,
        search::{ISearchRepository, SearchResult},
    },
};

#[derive(Debug, Clone)]
pub struct SearchRepository<C> {
    writer: SqlitePool,
    readers: SqlitePool,
    clock: C,
}

impl<C> SearchRepository<C> {
    pub async fn new(url: &str, clock: C) -> Result<Self, sqlx::Error> {
        let writer_options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .log_slow_statements(
                log::LevelFilter::Warn,
                std::time::Duration::from_millis(100),
            )
            .journal_mode(SqliteJournalMode::Wal);

        let writer = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(writer_options)
            .await?;

        // Run migrations using writer pool
        sqlx::migrate!("migrations/search").run(&writer).await?;

        let readers_options = SqliteConnectOptions::from_str(url)?
            .journal_mode(SqliteJournalMode::Wal)
            .read_only(true);
        let readers = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(readers_options)
            .await?;

        Ok(Self {
            writer,
            readers,
            clock,
        })
    }
}

impl<C> ISearchRepository for SearchRepository<C>
where
    C: IClock,
{
    #[tracing::instrument(skip_all, err)]
    async fn save_document(
        &self,
        document: &SearchDocument,
    ) -> Result<chrono::DateTime<chrono::Utc>, anyhow::Error> {
        let mut tx = self.writer.begin().await?;

        // We intentionally don't factor out the `DELETE FROM ...` from both branches to make each
        // one stand on its own, rather than having `SearchDocumentEvent::Deleted` be implicit.
        match document.event() {
            SearchDocumentEvent::Updated => {
                // `document_id` is an ordinary (UNINDEXED) column, not the FTS5 rowid,
                // so there is no unique constraint to build an upsert on.
                // Delete + insert keeps writes idempotent per document id.
                sqlx::query("DELETE FROM t_search WHERE document_id = ?1;")
                    .bind(document.document_id())
                    .execute(&mut *tx)
                    .await?;

                sqlx::query(
                    "INSERT INTO t_search (content, type, user, document_id) VALUES (?1, ?2, ?3, ?4);",
                )
                .bind(document.content())
                .bind(document.document_type().to_string())
                .bind(document.user())
                .bind(document.document_id())
                .execute(&mut *tx)
                .await?;
            }
            SearchDocumentEvent::Deleted => {
                sqlx::query("DELETE FROM t_search WHERE document_id = ?1;")
                    .bind(document.document_id())
                    .execute(&mut *tx)
                    .await?;
            }
        }

        let processed_ad = self.clock.now();
        tx.commit().await?;
        Ok(processed_ad)
    }

    #[tracing::instrument(skip_all, err)]
    async fn search(
        &self,
        user: &UserId,
        pattern: String,
    ) -> Result<Vec<SearchResult>, anyhow::Error> {
        if pattern.trim().is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT type, document_id
             FROM t_search
             WHERE t_search MATCH ?1 AND user = ?2
             ORDER BY rank, document_id;",
        )
        .bind(to_fts5_query(&pattern))
        .bind(user)
        .fetch_all(&self.readers)
        .await?;

        rows.into_iter()
            .map(|(document_type, document_id)| {
                let document_type =
                    SearchDocumentType::try_from(document_type.as_str()).map_err(|err| {
                        anyhow!("Unknown search document type '{document_type}': {err}")
                    })?;

                match document_type {
                    SearchDocumentType::Activity => {
                        Ok(SearchResult::Activity(ActivityId::from(&document_id)))
                    }
                    SearchDocumentType::TrainingNote => Ok(SearchResult::TrainingNote(
                        TrainingNoteId::from(&document_id),
                    )),
                }
            })
            .collect()
    }
}

/// Builds a safe FTS5 MATCH expression from free-form user input.
///
/// Each whitespace-separated term is wrapped in double quotes so FTS5 treats it
/// as a literal phrase, and terms are combined with `AND`. This avoids FTS5
/// operator injection (e.g. `OR`, `-`, `*`) while still letting multi-word
/// searches match documents that contain every term.
fn to_fts5_query(pattern: &str) -> String {
    pattern
        .split_whitespace()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;
    use crate::{
        clock::clock_test_utils::FakeClock,
        domain::{
            models::{
                UserId,
                activity::ActivityId,
                search::{SearchDocument, SearchDocumentEvent, SearchDocumentType},
                training::TrainingNoteId,
            },
            ports::search::SearchResult,
        },
    };

    const DOCUMENT_ID: &str = "activity-1";

    fn test_clock() -> FakeClock {
        FakeClock::new(
            chrono::DateTime::parse_from_rfc3339("2024-03-15T12:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        )
    }

    fn test_document(event: SearchDocumentEvent) -> SearchDocument {
        test_document_with_content(event, "Test document content")
    }

    fn test_document_with_content(event: SearchDocumentEvent, content: &str) -> SearchDocument {
        SearchDocument::new(
            SearchDocumentType::Activity,
            DOCUMENT_ID.to_string(),
            UserId::test_default(),
            event,
            content.to_string(),
            chrono::DateTime::parse_from_rfc3339("2024-03-15T10:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        )
    }

    async fn fetch_document_row(
        repo: &SearchRepository<FakeClock>,
    ) -> Option<(String, String, String, String)> {
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT content, type, document_id, user FROM t_search WHERE document_id = ?1;",
        )
        .bind(DOCUMENT_ID)
        .fetch_optional(&repo.readers)
        .await
        .expect("querying t_search should succeed")
    }

    async fn fetch_document_rows(
        repo: &SearchRepository<FakeClock>,
    ) -> Vec<(String, String, String, String)> {
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT content, type, document_id, user FROM t_search WHERE document_id = ?1;",
        )
        .bind(DOCUMENT_ID)
        .fetch_all(&repo.readers)
        .await
        .expect("querying t_search should succeed")
    }

    #[test]
    fn to_fts5_query_wraps_single_term_in_quotes() {
        assert_eq!(to_fts5_query("ride"), "\"ride\"");
    }

    #[test]
    fn to_fts5_query_joins_terms_with_and() {
        assert_eq!(to_fts5_query("long ride"), "\"long\" AND \"ride\"");
    }

    #[test]
    fn to_fts5_query_collapses_whitespace() {
        assert_eq!(to_fts5_query("long   ride"), "\"long\" AND \"ride\"");
    }

    #[test]
    fn to_fts5_query_returns_empty_for_blank_input() {
        assert_eq!(to_fts5_query(""), "");
        assert_eq!(to_fts5_query("   "), "");
    }

    #[test]
    fn to_fts5_query_escapes_embedded_quotes() {
        assert_eq!(to_fts5_query("ride\""), "\"ride\"\"\"");
    }

    #[test]
    fn to_fts5_query_neutralizes_fts5_boolean_operators() {
        assert_eq!(
            to_fts5_query("ride OR tempo"),
            "\"ride\" AND \"OR\" AND \"tempo\""
        );
        assert_eq!(
            to_fts5_query("ride AND tempo"),
            "\"ride\" AND \"AND\" AND \"tempo\""
        );
        assert_eq!(
            to_fts5_query("ride NOT tempo"),
            "\"ride\" AND \"NOT\" AND \"tempo\""
        );
    }

    #[test]
    fn to_fts5_query_neutralizes_fts5_syntax() {
        assert_eq!(to_fts5_query("ride -tempo"), "\"ride\" AND \"-tempo\"");
        assert_eq!(to_fts5_query("ride*"), "\"ride*\"");
        assert_eq!(
            to_fts5_query("type:activity ride"),
            "\"type:activity\" AND \"ride\""
        );
        assert_eq!(
            to_fts5_query("NEAR(ride tempo)"),
            "\"NEAR(ride\" AND \"tempo)\""
        );
    }

    #[tokio::test]
    async fn save_document_updated_when_no_existing_row() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");

        let processed_at = repo
            .save_document(&test_document(SearchDocumentEvent::Updated))
            .await
            .expect("save_document should succeed");

        assert_eq!(processed_at, test_clock().now());
        assert_eq!(
            fetch_document_row(&repo).await.expect("row should exist"),
            (
                "Test document content".to_string(),
                "activity".to_string(),
                DOCUMENT_ID.to_string(),
                "test_user".to_string(),
            )
        );
    }

    #[tokio::test]
    async fn save_document_updated_when_existing_row() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");

        repo.save_document(&test_document_with_content(
            SearchDocumentEvent::Updated,
            "Initial content",
        ))
        .await
        .expect("first save should succeed");

        repo.save_document(&test_document_with_content(
            SearchDocumentEvent::Updated,
            "Updated content",
        ))
        .await
        .expect("second save should succeed");

        assert_eq!(
            fetch_document_rows(&repo).await,
            vec![(
                "Updated content".to_string(),
                "activity".to_string(),
                DOCUMENT_ID.to_string(),
                "test_user".to_string(),
            )]
        );
    }

    #[tokio::test]
    async fn save_document_updated_is_idempotent() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");

        let document = test_document(SearchDocumentEvent::Updated);

        repo.save_document(&document)
            .await
            .expect("first save should succeed");
        repo.save_document(&document)
            .await
            .expect("second save should succeed");

        assert_eq!(
            fetch_document_rows(&repo).await,
            vec![(
                "Test document content".to_string(),
                "activity".to_string(),
                DOCUMENT_ID.to_string(),
                "test_user".to_string(),
            )]
        );
    }

    #[tokio::test]
    async fn save_document_deleted_removes_existing_row() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");

        repo.save_document(&test_document(SearchDocumentEvent::Updated))
            .await
            .expect("save_document Updated should succeed");
        assert!(fetch_document_row(&repo).await.is_some());

        let processed_at = repo
            .save_document(&test_document(SearchDocumentEvent::Deleted))
            .await
            .expect("save_document Deleted should succeed");

        assert_eq!(processed_at, test_clock().now());
        assert!(fetch_document_row(&repo).await.is_none());
    }

    #[tokio::test]
    async fn save_document_deleted_does_not_fail_when_row_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");

        let processed_at = repo
            .save_document(&test_document(SearchDocumentEvent::Deleted))
            .await
            .expect("deleting a missing row should be a no-op");

        assert_eq!(processed_at, test_clock().now());
        assert!(fetch_document_row(&repo).await.is_none());
    }

    async fn insert_document_row(
        repo: &SearchRepository<FakeClock>,
        content: &str,
        document_type: SearchDocumentType,
        document_id: &str,
        user: &UserId,
    ) {
        sqlx::query(
            "INSERT INTO t_search (content, type, document_id, user) VALUES (?1, ?2, ?3, ?4);",
        )
        .bind(content)
        .bind(document_type.to_string())
        .bind(document_id)
        .bind(user)
        .execute(&repo.writer)
        .await
        .expect("inserting test document should succeed");
    }

    #[tokio::test]
    async fn search_returns_activity_and_training_note_results_for_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");
        let user = UserId::test_default();

        insert_document_row(
            &repo,
            "long ride in the mountains",
            SearchDocumentType::Activity,
            "activity-1",
            &user,
        )
        .await;
        insert_document_row(
            &repo,
            "tempo ride",
            SearchDocumentType::Activity,
            "activity-2",
            &user,
        )
        .await;
        insert_document_row(
            &repo,
            "training note about ride",
            SearchDocumentType::TrainingNote,
            "note-1",
            &user,
        )
        .await;

        let results = repo
            .search(&user, "ride".to_string())
            .await
            .expect("search should succeed");

        assert_eq!(results.len(), 3);
        assert!(results.contains(&SearchResult::Activity(ActivityId::from("activity-1"))));
        assert!(results.contains(&SearchResult::Activity(ActivityId::from("activity-2"))));
        assert!(results.contains(&SearchResult::TrainingNote(TrainingNoteId::from("note-1"))));
    }

    #[tokio::test]
    async fn search_filters_results_by_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");
        let alice = UserId::from("alice");
        let bob = UserId::from("bob");

        insert_document_row(
            &repo,
            "long ride",
            SearchDocumentType::Activity,
            "activity-alice",
            &alice,
        )
        .await;
        insert_document_row(
            &repo,
            "long ride",
            SearchDocumentType::Activity,
            "activity-bob",
            &bob,
        )
        .await;

        let results = repo
            .search(&alice, "ride".to_string())
            .await
            .expect("search should succeed");

        assert_eq!(
            results,
            vec![SearchResult::Activity(ActivityId::from("activity-alice"))]
        );
    }

    #[tokio::test]
    async fn search_matches_documents_containing_all_terms() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");
        let user = UserId::test_default();

        insert_document_row(
            &repo,
            "long ride in the mountains",
            SearchDocumentType::Activity,
            "activity-1",
            &user,
        )
        .await;
        insert_document_row(
            &repo,
            "long run",
            SearchDocumentType::Activity,
            "activity-2",
            &user,
        )
        .await;

        let results = repo
            .search(&user, "long ride".to_string())
            .await
            .expect("search should succeed");

        assert_eq!(
            results,
            vec![SearchResult::Activity(ActivityId::from("activity-1"))]
        );
    }

    #[tokio::test]
    async fn search_returns_no_results_for_blank_pattern() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");
        let user = UserId::test_default();

        insert_document_row(
            &repo,
            "long ride",
            SearchDocumentType::Activity,
            "activity-1",
            &user,
        )
        .await;

        for pattern in ["", "   "] {
            let results = repo
                .search(&user, pattern.to_string())
                .await
                .expect("search should succeed");
            assert!(results.is_empty(), "expected no results for {pattern:?}");
        }
    }

    #[tokio::test]
    async fn search_treats_fts5_operators_as_literal_terms() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");
        let user = UserId::test_default();

        insert_document_row(
            &repo,
            "long ride",
            SearchDocumentType::Activity,
            "activity-1",
            &user,
        )
        .await;
        insert_document_row(
            &repo,
            "tempo run",
            SearchDocumentType::Activity,
            "activity-2",
            &user,
        )
        .await;
        insert_document_row(
            &repo,
            "ride or tempo",
            SearchDocumentType::Activity,
            "activity-3",
            &user,
        )
        .await;

        let results = repo
            .search(&user, "ride OR tempo".to_string())
            .await
            .expect("search should succeed");

        assert_eq!(
            results,
            vec![SearchResult::Activity(ActivityId::from("activity-3"))]
        );
    }
}
