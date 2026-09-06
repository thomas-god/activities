use std::str::FromStr;

use anyhow::anyhow;
use sqlx::{
    ConnectOptions, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use unicode_categories::UnicodeCategories;
use unicode_normalization::UnicodeNormalization;

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

const KEY_LAST_IMPORT_DATE: &str = "last_import_date";

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

                let content = normalize_for_search(document.content());
                if non_empty_tokens(&content) {
                    sqlx::query(
                    "INSERT INTO t_search (content, type, user, document_id) VALUES (?1, ?2, ?3, ?4);",
                )
                .bind(&content)
                .bind(document.document_type().to_string())
                .bind(document.user())
                .bind(document.document_id())
                .execute(&mut *tx)
                .await?;
                }
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
        let query = to_fts5_query(&normalize_for_search(&pattern));
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT type, document_id
             FROM t_search(?1)
             WHERE user = ?2
             ORDER BY rank, document_id;",
        )
        .bind(query)
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

    async fn get_last_import_value(
        &self,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, anyhow::Error> {
        sqlx::query_as::<_, (chrono::DateTime<chrono::Utc>,)>(
            "SELECT value FROM t_metadata WHERE key = ?1 LIMIT 1;",
        )
        .bind(KEY_LAST_IMPORT_DATE)
        .fetch_optional(&self.readers)
        .await
        .map(|row| row.map(|(import_date,)| import_date))
        .map_err(|err| anyhow!(err))
    }

    async fn set_last_import_value(
        &self,
        imported_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO t_metadata (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value=excluded.value;",
        )
        .bind(KEY_LAST_IMPORT_DATE)
        .bind(imported_at)
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|err| anyhow!(err))
    }
}

/// Normalizes a string of its diacritics.
fn normalize_for_search(input: &str) -> String {
    input
        .trim()
        .nfd()
        .filter(|c| !c.is_mark_nonspacing())
        .collect()
}

const TOKENIZER_TOKEN_MIN_LEN: usize = 3;

/// Checks if a an input &str will result in an non-empty list of tokens
fn non_empty_tokens(input: &str) -> bool {
    input
        .split_whitespace()
        .filter(|w| w.chars().count() >= TOKENIZER_TOKEN_MIN_LEN)
        .count()
        > 0
}

/// Builds a safe FTS5 MATCH expression from free-form user input.
///
/// Each whitespace-separated term is wrapped in double quotes so FTS5 treats it
/// as a literal phrase, and terms are combined with `AND`. This avoids FTS5
/// operator injection (e.g. `OR`, `-`, `*`) while still letting multi-word
/// searches match documents that contain every term.
///
/// Terms of fewer than `TOKEN_MIN_LEN` characters are dropped: the trigram tokenizer needs at
/// least `TOKEN_MIN_LEN` characters to form a trigram, so shorter terms could never match
/// anything in the index.
fn to_fts5_query(pattern: &str) -> String {
    pattern
        .split_whitespace()
        .filter(|w| w.chars().count() >= TOKENIZER_TOKEN_MIN_LEN)
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

#[cfg(test)]
mod test_search_tokenizing {
    use super::*;

    #[test]
    fn empty_and_whitespace_only_input_normalize_to_empty() {
        assert_eq!(normalize_for_search(""), "");
        assert_eq!(normalize_for_search("   "), "");
        assert_eq!(normalize_for_search("\t\r\n "), "");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize_for_search("  Hello  "), "Hello");
        assert_eq!(normalize_for_search("\n\t World \r\n"), "World");
    }

    #[test]
    fn preserves_inner_whitespace() {
        assert_eq!(normalize_for_search("  a   b  "), "a   b");
    }

    #[test]
    fn strips_diacritics_from_precomposed_input() {
        assert_eq!(normalize_for_search("Café"), "Cafe");
        assert_eq!(normalize_for_search("Crème brûlée"), "Creme brulee");
        assert_eq!(normalize_for_search("Déjà vu"), "Deja vu");
    }

    #[test]
    fn strips_diacritics_from_already_decomposed_input() {
        // "e\u{301}" (e + combining acute) is the NFD form of "é"; both must normalize the same.
        assert_eq!(normalize_for_search("e\u{0301}"), "e");
        assert_eq!(normalize_for_search("e\u{0301}"), normalize_for_search("é"));
        // "ế" decomposes as e + combining circumflex + combining acute.
        assert_eq!(normalize_for_search("ế"), "e");
        // "Å" decomposes as A + combining ring above.
        assert_eq!(normalize_for_search("A\u{030A}"), "A");
        assert_eq!(normalize_for_search("Ångström"), "Angstrom");
    }

    #[test]
    fn preserves_letter_case() {
        assert_eq!(normalize_for_search("Éléphant"), "Elephant");
        assert_eq!(normalize_for_search("Über"), "Uber");
    }

    #[test]
    fn keeps_characters_without_decomposable_diacritics() {
        // ß and ø have no canonical decomposition, so they survive NFD untouched.
        assert_eq!(normalize_for_search("Straße"), "Straße");
        assert_eq!(normalize_for_search("øre"), "øre");
        // Scripts without marks are passed through unchanged.
        assert_eq!(normalize_for_search("中文"), "中文");
        assert_eq!(normalize_for_search("Привет"), "Привет");
    }

    #[test]
    fn preserves_digits_and_punctuation() {
        assert_eq!(
            normalize_for_search("  Hello, world! #123.  "),
            "Hello, world! #123."
        );
    }

    #[test]
    fn handles_mark_only_and_leading_marks() {
        assert_eq!(normalize_for_search("\u{0301}"), "");
        assert_eq!(normalize_for_search("\u{0301}a\u{0301}"), "a");
    }

    #[test]
    fn normalization_is_idempotent() {
        let samples = ["Café déjà vu", "  Ångström  ", "Straße", "中文"];
        for sample in samples {
            let once = normalize_for_search(sample);
            assert_eq!(normalize_for_search(&once), once, "{sample:?}");
        }
    }

    #[test]
    fn non_empty_tokens_is_false_when_no_token_reaches_min_length() {
        assert!(!non_empty_tokens(""));
        assert!(!non_empty_tokens("   "));
        assert!(!non_empty_tokens("a"));
        assert!(!non_empty_tokens("ab"));
        assert!(!non_empty_tokens("lo"));
        assert!(!non_empty_tokens("ab cd"));
        // Characters are counted, not bytes: 中文 is 2 chars but 6 bytes.
        assert!(!non_empty_tokens("中文"));
    }

    #[test]
    fn non_empty_tokens_is_true_when_a_token_reaches_min_length() {
        // Exactly TOKENIZER_TOKEN_MIN_LEN characters is the smallest matchable token.
        assert!(non_empty_tokens("abc"));
        assert!(non_empty_tokens("ride"));
        // A single long token is enough, the shorter ones are simply dropped.
        assert!(non_empty_tokens("ab ride"));
        assert!(non_empty_tokens("long   ride"));
        assert!(non_empty_tokens("Привет"));
    }
}

#[cfg(test)]
mod test_search_prepare_query {
    use super::*;

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
        // AND/NOT are at least 3 characters long, so they survive the trigram
        // filter and must be quoted to be treated as literal terms rather than
        // operators.
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
    fn to_fts5_query_drops_terms_shorter_than_a_trigram() {
        // The trigram tokenizer can only match substrings of at least 3
        // characters, so shorter terms (including the operator "OR") are
        // dropped instead of being sent to FTS5.
        assert_eq!(to_fts5_query("a"), "");
        assert_eq!(to_fts5_query("ri"), "");
        // Characters are counted, not bytes: 中文 is 2 chars but 6 bytes.
        assert_eq!(to_fts5_query("中文"), "");
        // A 3-character term is the smallest that can still match.
        assert_eq!(to_fts5_query("abc"), "\"abc\"");
        // Short terms are dropped individually, longer ones are kept.
        assert_eq!(to_fts5_query("go ride"), "\"ride\"");
        assert_eq!(to_fts5_query("on a long ride"), "\"long\" AND \"ride\"");
        assert_eq!(to_fts5_query("ride OR tempo"), "\"ride\" AND \"tempo\"");
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
}

#[cfg(test)]
mod tests_sqlite_search_repository {
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
    async fn save_document_updated_skips_insert_when_content_has_no_indexable_token() {
        // "ab cd" only has 2-char tokens, "éé à" normalizes to "ee a", and
        // whitespace normalizes to empty: none can ever match a trigram search.
        for content in ["ab cd", "   ", "éé à"] {
            let db_file = NamedTempFile::new().unwrap();
            let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
                .await
                .expect("Failed to create test repository");

            repo.save_document(&test_document_with_content(
                SearchDocumentEvent::Updated,
                content,
            ))
            .await
            .expect("save_document should succeed");

            assert!(
                fetch_document_row(&repo).await.is_none(),
                "expected no row to be indexed for {content:?}"
            );
        }
    }

    #[tokio::test]
    async fn save_document_updated_indexes_content_with_at_least_one_long_token() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");

        // Short tokens are dropped for matching, but the document still holds a
        // searchable "ride" token, so the whole normalized content is stored.
        repo.save_document(&test_document_with_content(
            SearchDocumentEvent::Updated,
            "ab ride",
        ))
        .await
        .expect("save_document should succeed");

        assert_eq!(
            fetch_document_row(&repo).await.expect("row should exist"),
            (
                "ab ride".to_string(),
                "activity".to_string(),
                DOCUMENT_ID.to_string(),
                "test_user".to_string(),
            )
        );
    }

    #[tokio::test]
    async fn save_document_updated_removes_row_when_content_becomes_unindexable() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");
        let user = UserId::test_default();

        repo.save_document(&test_document_with_content(
            SearchDocumentEvent::Updated,
            "long ride",
        ))
        .await
        .expect("first save should succeed");

        assert_eq!(
            repo.search(&user, "ride".to_string())
                .await
                .expect("search should succeed"),
            vec![SearchResult::Activity(ActivityId::from(DOCUMENT_ID))]
        );

        // The delete always runs first, so re-saving with unindexable content
        // removes the previously indexed row instead of leaving it stale.
        repo.save_document(&test_document_with_content(
            SearchDocumentEvent::Updated,
            "ab cd",
        ))
        .await
        .expect("second save should succeed");

        assert!(fetch_document_row(&repo).await.is_none());
        assert!(
            repo.search(&user, "ride".to_string())
                .await
                .expect("search should succeed")
                .is_empty()
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
    async fn search_returns_no_results_for_blank_or_too_short_pattern() {
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

        // "l" and "lo" are too short for the trigram tokenizer to match,
        // even though the indexed content starts with them.
        for pattern in ["", "   ", "l", "lo"] {
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

    async fn fetch_metadata_keys(repo: &SearchRepository<FakeClock>) -> Vec<String> {
        sqlx::query_as::<_, (String,)>("SELECT key FROM t_metadata ORDER BY key;")
            .fetch_all(&repo.readers)
            .await
            .expect("querying t_metadata should succeed")
            .into_iter()
            .map(|(key,)| key)
            .collect()
    }

    fn utc_datetime(rfc3339: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[tokio::test]
    async fn get_last_import_value_returns_none_when_never_set() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");

        let value = repo
            .get_last_import_value()
            .await
            .expect("get_last_import_value should succeed");

        assert!(value.is_none());
        assert!(fetch_metadata_keys(&repo).await.is_empty());
    }

    #[tokio::test]
    async fn set_last_import_value_persists_value_readable_by_get() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");
        let imported_at = utc_datetime("2024-03-15T10:30:45.123456789Z");

        repo.set_last_import_value(imported_at)
            .await
            .expect("set_last_import_value should succeed");

        assert_eq!(
            repo.get_last_import_value()
                .await
                .expect("get_last_import_value should succeed"),
            Some(imported_at)
        );
        assert_eq!(fetch_metadata_keys(&repo).await, vec![KEY_LAST_IMPORT_DATE]);
    }

    #[tokio::test]
    async fn set_last_import_value_overwrites_previous_value() {
        let db_file = NamedTempFile::new().unwrap();
        let repo = SearchRepository::new(&db_file.path().to_string_lossy(), test_clock())
            .await
            .expect("Failed to create test repository");
        let first = utc_datetime("2024-03-15T10:00:00Z");
        let second = utc_datetime("2024-03-16T08:00:00Z");

        repo.set_last_import_value(first)
            .await
            .expect("first set should succeed");
        repo.set_last_import_value(second)
            .await
            .expect("second set should succeed");

        assert_eq!(
            repo.get_last_import_value()
                .await
                .expect("get_last_import_value should succeed"),
            Some(second)
        );
        // The upsert must not leave duplicate rows behind.
        assert_eq!(fetch_metadata_keys(&repo).await, vec![KEY_LAST_IMPORT_DATE]);
    }
}
