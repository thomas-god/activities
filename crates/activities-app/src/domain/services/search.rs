use std::sync::Arc;

use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::domain::{
    models::UserId,
    ports::{
        IClock,
        search::{
            IDocumentsForSearch, ISearchRepository, ISearchService, RemainingDocuments,
            SearchResult,
        },
    },
};

const BATCH_SIZE: i64 = 100;

#[derive(Debug, Clone)]
pub struct SearchService<SR, ADS, TDS, C> {
    repository: SR,
    activity_notify: Arc<Notify>,
    activity_service: ADS,
    training_notify: Arc<Notify>,
    training_service: TDS,
    shutdown: CancellationToken,
    clock: C,
}

impl<SR, ADS, TDS, C> SearchService<SR, ADS, TDS, C>
where
    SR: ISearchRepository,
    ADS: IDocumentsForSearch,
    TDS: IDocumentsForSearch,
    C: IClock,
{
    pub fn new(
        repository: SR,
        activity_notify: Arc<Notify>,
        activity_service: ADS,
        training_notify: Arc<Notify>,
        training_service: TDS,
        shutdown: CancellationToken,
        clock: C,
    ) -> Self {
        Self {
            repository,
            activity_notify,
            activity_service,
            training_notify,
            training_service,
            shutdown,
            clock,
        }
    }

    pub async fn run(&self) {
        let _ = self.import_existing_documents().await;

        let activity_notified = self.activity_notify.notified();
        let training_notified = self.training_notify.notified();
        let shutdown = self.shutdown.cancelled();
        tokio::pin!(activity_notified, training_notified, shutdown);

        loop {
            tokio::select! {
                _ = &mut activity_notified =>  {
                    self.process_pending_outbox(self.activity_service.clone()).await;
                    activity_notified.set(self.activity_notify.notified());
                }
                _ = &mut training_notified =>  {
                    self.process_pending_outbox(self.training_service.clone()).await;
                    training_notified.set(self.training_notify.notified());
                }
                _ = &mut shutdown => return,
            }
        }
    }

    #[tracing::instrument(skip_all)]
    async fn process_pending_outbox<DS: IDocumentsForSearch>(&self, service: DS) {
        let kind = service.service_kind();
        let documents = match service.get_pending_documents_to_process().await {
            Ok(documents) => documents,
            Err(err) => {
                tracing::warn!("Error getting pending documents form {kind} outbox: {err}");
                return;
            }
        };

        for document in documents {
            let processed_at = match self.repository.save_document(&document).await {
                Ok(processed_at) => processed_at,
                Err(err) => {
                    tracing::warn!(
                        "Error saving document to search index for {kind} {}: {}",
                        document.document_id(),
                        err
                    );
                    continue;
                }
            };
            if let Err(err) = service
                .mark_document_as_processed(&document, processed_at)
                .await
            {
                tracing::warn!(
                    "Error marking document as processed for {kind} {}: {}",
                    document.document_id(),
                    err
                );
            }
        }
    }

    #[tracing::instrument(skip_all)]
    async fn import_existing_documents(&self) -> anyhow::Result<()> {
        if let Some(last_import) = self.repository.get_last_import_value().await? {
            tracing::info!("Skipping import of search documents, last done {last_import}");
            return anyhow::Ok(());
        }

        let cloned_self = self.clone();
        tokio::spawn(async move {
            // Activity documents
            if let Err(err) = cloned_self
                .import_existing_documents_for_service(cloned_self.activity_service.clone())
                .await
            {
                tracing::error!("Error while importing activity documents: {err}");
            }

            // Training documents
            if let Err(err) = cloned_self
                .import_existing_documents_for_service(cloned_self.training_service.clone())
                .await
            {
                tracing::error!("Error while importing training documents: {err}");
            }
        });
        anyhow::Ok(())
    }

    #[tracing::instrument(skip_all, err)]
    async fn import_existing_documents_for_service<DS: IDocumentsForSearch>(
        &self,
        service: DS,
    ) -> Result<(), anyhow::Error> {
        tracing::info!(
            "Starting importing existing documents for {}",
            service.service_kind()
        );
        let start = std::time::Instant::now();
        let mut remaining_documents = RemainingDocuments::from(true);
        let mut page = 0;
        let mut imported_count = 0;

        while remaining_documents.remaining() {
            let (documents, flag) = service.snapshot_documents(BATCH_SIZE, page).await?;
            page += 1;
            remaining_documents = flag;

            for document in documents {
                if self.repository.save_document(&document).await.is_ok() {
                    imported_count += 1;
                };
            }
        }

        tracing::info!(
            "Finished importing existing documents for {}: {imported_count} documents in {}ms",
            service.service_kind(),
            start.elapsed().as_millis()
        );
        self.repository
            .set_last_import_value(self.clock.now())
            .await?;
        Ok(())
    }
}

impl<SR, ADS, TDS, C> ISearchService for SearchService<SR, ADS, TDS, C>
where
    SR: ISearchRepository,
    ADS: IDocumentsForSearch,
    TDS: IDocumentsForSearch,
    C: IClock,
{
    #[tracing::instrument(skip_all, err)]
    async fn search(
        &self,
        user: &UserId,
        pattern: String,
    ) -> Result<Vec<SearchResult>, anyhow::Error> {
        self.repository.search(user, pattern).await
    }
}

#[cfg(test)]
mod tests_search_service {
    //! Tests for `SearchService`.
    //!
    //! A couple of test-only tricks worth knowing before reading:
    //!
    //! - `run()` is an infinite `tokio::select!` loop and never returns on its
    //!   own. Tests spawn it and stop it by cancelling `service.shutdown`, then
    //!   await the spawned handle.
    //! - `run()` clones the activity/training service for each notification, so
    //!   the mocks configure `expect_clone()` to return a fully-configured mock.
    //! - A `done` `Notify` is used purely as a synchronization probe: mock
    //!   methods notify it, and the test awaits it to know processing finished
    //!   before cancelling shutdown. This avoids sleeps and races.

    use anyhow::anyhow;

    use super::{BATCH_SIZE, SearchService};
    use crate::{
        clock::clock_test_utils::FakeClock,
        domain::{
            models::{
                UserId,
                activity::ActivityId,
                search::{SearchDocument, SearchDocumentEvent, SearchDocumentType},
            },
            ports::search::{
                ISearchService, RemainingDocuments, SearchResult,
                search_test_utils::{MockDocumentsForSearch, MockSearchRepository},
            },
        },
    };

    fn document(id: &str) -> SearchDocument {
        SearchDocument::new(
            SearchDocumentType::Activity,
            id.to_string(),
            UserId::test_default(),
            SearchDocumentEvent::Updated,
            "some content".to_string(),
            chrono::Utc::now(),
        )
    }

    fn training_document(id: &str) -> SearchDocument {
        SearchDocument::new(
            SearchDocumentType::TrainingNote,
            id.to_string(),
            UserId::test_default(),
            SearchDocumentEvent::Updated,
            "some training content".to_string(),
            chrono::Utc::now(),
        )
    }

    fn build_service_skip_import(
        mut repository: MockSearchRepository,
        activity_service: MockDocumentsForSearch,
        training_service: MockDocumentsForSearch,
    ) -> SearchService<
        MockSearchRepository,
        MockDocumentsForSearch,
        MockDocumentsForSearch,
        FakeClock,
    > {
        repository
            .expect_get_last_import_value()
            .returning(|| Ok(Some(chrono::Utc::now())));
        SearchService::new(
            repository,
            std::sync::Arc::new(tokio::sync::Notify::new()),
            activity_service,
            std::sync::Arc::new(tokio::sync::Notify::new()),
            training_service,
            tokio_util::sync::CancellationToken::new(),
            FakeClock::default(),
        )
    }

    fn processed_at() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::<chrono::Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[tokio::test]
    async fn test_process_pending_outbox_does_nothing_without_documents() {
        let mut repository = MockSearchRepository::new();
        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_service_kind()
            .returning(|| SearchDocumentType::Activity);
        documents
            .expect_get_pending_documents_to_process()
            .return_once(|| Ok(vec![]));

        // Expect no side effects
        repository.expect_save_document().times(0);
        documents.expect_mark_document_as_processed().times(0);

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        service.process_pending_outbox(documents).await;
    }

    #[tokio::test]
    async fn test_process_pending_outbox_does_nothing_when_fetching_documents_fails() {
        let mut repository = MockSearchRepository::new();
        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_service_kind()
            .returning(|| SearchDocumentType::Activity);
        documents
            .expect_get_pending_documents_to_process()
            .return_once(|| Err(anyhow!("failed to fetch documents")));

        // Expect no side effects
        repository.expect_save_document().times(0);
        documents.expect_mark_document_as_processed().times(0);

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        service.process_pending_outbox(documents).await;
    }

    #[tokio::test]
    async fn test_process_pending_outbox_saves_and_marks_documents() {
        let processed_at = processed_at();
        let expected_processed_at = processed_at.clone();
        let doc_a = document("doc-a");
        let doc_b = document("doc-b");

        let mut repository = MockSearchRepository::new();
        repository
            .expect_save_document()
            .times(2)
            .returning(move |_| Ok(processed_at));

        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_service_kind()
            .returning(|| SearchDocumentType::Activity);
        documents
            .expect_get_pending_documents_to_process()
            .return_once(move || Ok(vec![doc_a, doc_b]));
        documents
            .expect_mark_document_as_processed()
            .times(2)
            .withf(move |doc, at| {
                (doc.document_id() == "doc-a" || doc.document_id() == "doc-b")
                    && *at == expected_processed_at
            })
            .returning(|_, _| Ok(()));

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        service.process_pending_outbox(documents).await;
    }

    #[tokio::test]
    async fn test_process_pending_outbox_skips_marking_when_save_fails() {
        let processed_at = processed_at();
        let expected_processed_at = processed_at.clone();
        let doc_a = document("doc-a");
        let doc_b = document("doc-b");

        let mut repository = MockSearchRepository::new();
        repository
            .expect_save_document()
            .times(2)
            .returning(move |doc| {
                if doc.document_id() == "doc-a" {
                    Err(anyhow!("failed to save document"))
                } else {
                    Ok(processed_at)
                }
            });

        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_service_kind()
            .returning(|| SearchDocumentType::Activity);
        documents
            .expect_get_pending_documents_to_process()
            .return_once(move || Ok(vec![doc_a, doc_b]));
        documents
            .expect_mark_document_as_processed()
            // Only doc-b is saved
            .times(1)
            .withf(move |doc, at| doc.document_id() == "doc-b" && *at == expected_processed_at)
            .returning(|_, _| Ok(()));

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        service.process_pending_outbox(documents).await;
    }

    #[tokio::test]
    async fn test_process_pending_outbox_continues_when_marking_fails() {
        let processed_at = processed_at();
        let doc_a = document("doc-a");
        let doc_b = document("doc-b");

        let mut repository = MockSearchRepository::new();
        repository
            .expect_save_document()
            .times(2)
            .returning(move |_| Ok(processed_at));

        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_service_kind()
            .returning(|| SearchDocumentType::Activity);
        documents
            .expect_get_pending_documents_to_process()
            .return_once(move || Ok(vec![doc_a, doc_b]));
        documents
            .expect_mark_document_as_processed()
            .times(2)
            .returning(|doc, _| {
                if doc.document_id() == "doc-a" {
                    Err(anyhow!("failed to mark document"))
                } else {
                    Ok(())
                }
            });

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        service.process_pending_outbox(documents).await;
    }

    #[tokio::test]
    async fn test_run_returns_when_shutdown_is_cancelled() {
        let service = build_service_skip_import(
            MockSearchRepository::new(),
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        let shutdown = service.shutdown.clone();
        let handle = tokio::spawn(async move {
            service.run().await;
        });

        shutdown.cancel();
        handle
            .await
            .expect("run should return once shutdown is cancelled");
    }

    #[tokio::test]
    async fn test_run_processes_activity_when_notified_then_stops_on_shutdown() {
        let done = std::sync::Arc::new(tokio::sync::Notify::new());

        let mut repository = MockSearchRepository::new();
        repository.expect_clone().returning(|| {
            let mut repo = MockSearchRepository::new();
            repo.expect_get_last_import_value()
                .returning(|| Ok(Some(chrono::Utc::now())));
            return repo;
        });
        repository
            .expect_save_document()
            .times(1)
            .returning(|_| Ok(processed_at()));

        let mut activity = MockDocumentsForSearch::new();
        let done_clone = done.clone();
        activity.expect_clone().returning(move || {
            let mut cloned = MockDocumentsForSearch::new();
            cloned
                .expect_service_kind()
                .returning(|| SearchDocumentType::Activity);
            cloned
                .expect_get_pending_documents_to_process()
                .return_once(|| Ok(vec![document("doc-a")]));
            let done_clone = done_clone.clone();
            cloned
                .expect_mark_document_as_processed()
                .times(1)
                .returning(move |_, _| {
                    done_clone.notify_one();
                    Ok(())
                });
            cloned
        });

        let service =
            build_service_skip_import(repository, activity, MockDocumentsForSearch::new());
        let activity_notify = service.activity_notify.clone();
        let shutdown = service.shutdown.clone();

        let handle = tokio::spawn(async move {
            service.run().await;
        });

        activity_notify.notify_one();
        done.notified().await;

        shutdown.cancel();
        handle
            .await
            .expect("run should return after shutdown is cancelled");
    }

    #[tokio::test]
    async fn test_search_delegates_to_repository() {
        let user = UserId::test_default();
        let expected_user = user.clone();
        let expected = vec![SearchResult::Activity(ActivityId::from("activity-1"))];

        let mut repository = MockSearchRepository::new();
        repository.expect_clone().returning(|| {
            let mut repo = MockSearchRepository::new();
            repo.expect_get_last_import_value()
                .returning(|| Ok(Some(chrono::Utc::now())));
            return repo;
        });
        repository
            .expect_search()
            .times(1)
            .withf(move |u, pattern| u == &expected_user && pattern == "long ride")
            .returning(|_, _| Ok(vec![SearchResult::Activity(ActivityId::from("activity-1"))]));

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        let res = service
            .search(&user, "long ride".to_string())
            .await
            .unwrap();
        assert_eq!(res, expected);
    }

    #[tokio::test]
    async fn test_search_propagates_repository_error() {
        let mut repository = MockSearchRepository::new();
        repository.expect_clone().returning(|| {
            let mut repo = MockSearchRepository::new();
            repo.expect_get_last_import_value()
                .returning(|| Ok(Some(chrono::Utc::now())));
            return repo;
        });
        repository
            .expect_search()
            .returning(|_, _| Err(anyhow!("failed to search")));

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        let res = service
            .search(&UserId::test_default(), "pattern".to_string())
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_run_processes_training_when_notified_then_stops_on_shutdown() {
        let done = std::sync::Arc::new(tokio::sync::Notify::new());

        let mut repository = MockSearchRepository::new();
        repository.expect_clone().returning(|| {
            let mut repo = MockSearchRepository::new();
            repo.expect_get_last_import_value()
                .returning(|| Ok(Some(chrono::Utc::now())));
            return repo;
        });
        repository
            .expect_save_document()
            .times(1)
            .returning(|_| Ok(processed_at()));

        let mut training = MockDocumentsForSearch::new();
        let done_clone = done.clone();
        training.expect_clone().returning(move || {
            let mut cloned = MockDocumentsForSearch::new();
            cloned
                .expect_service_kind()
                .returning(|| SearchDocumentType::TrainingNote);
            cloned
                .expect_get_pending_documents_to_process()
                .return_once(|| Ok(vec![training_document("doc-training")]));
            let done_clone = done_clone.clone();
            cloned
                .expect_mark_document_as_processed()
                .times(1)
                .returning(move |_, _| {
                    done_clone.notify_one();
                    Ok(())
                });
            cloned
        });

        let service =
            build_service_skip_import(repository, MockDocumentsForSearch::new(), training);
        let training_notify = service.training_notify.clone();
        let shutdown = service.shutdown.clone();

        let handle = tokio::spawn(async move {
            service.run().await;
        });

        training_notify.notify_one();
        done.notified().await;

        shutdown.cancel();
        handle
            .await
            .expect("run should return after shutdown is cancelled");
    }

    #[tokio::test]
    async fn test_run_processes_activity_multiple_times_when_notified_multiple_times() {
        let done = std::sync::Arc::new(tokio::sync::Notify::new());

        let mut repository = MockSearchRepository::new();
        repository.expect_clone().returning(|| {
            let mut repo = MockSearchRepository::new();
            repo.expect_get_last_import_value()
                .returning(|| Ok(Some(chrono::Utc::now())));
            return repo;
        });
        repository
            .expect_save_document()
            .times(2)
            .returning(|_| Ok(processed_at()));

        let mut activity = MockDocumentsForSearch::new();
        let done_clone = done.clone();
        activity.expect_clone().times(2).returning(move || {
            let mut cloned = MockDocumentsForSearch::new();
            cloned
                .expect_service_kind()
                .returning(|| SearchDocumentType::Activity);
            cloned
                .expect_get_pending_documents_to_process()
                .return_once(|| Ok(vec![document("doc-a")]));
            let done_clone = done_clone.clone();
            cloned
                .expect_mark_document_as_processed()
                .times(1)
                .returning(move |_, _| {
                    done_clone.notify_one();
                    Ok(())
                });
            cloned
        });

        let service =
            build_service_skip_import(repository, activity, MockDocumentsForSearch::new());
        let activity_notify = service.activity_notify.clone();
        let shutdown = service.shutdown.clone();

        let handle = tokio::spawn(async move {
            service.run().await;
        });

        activity_notify.notify_one();
        done.notified().await;

        activity_notify.notify_one();
        done.notified().await;

        shutdown.cancel();
        handle
            .await
            .expect("run should return after shutdown is cancelled");
    }

    #[tokio::test]
    async fn test_run_returns_immediately_when_shutdown_is_already_cancelled() {
        let service = build_service_skip_import(
            MockSearchRepository::new(),
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        let shutdown = service.shutdown.clone();
        shutdown.cancel();

        let handle = tokio::spawn(async move {
            service.run().await;
        });

        handle
            .await
            .expect("run should return immediately when shutdown is already cancelled");
    }

    #[tokio::test]
    async fn test_snapshot_existing_documents_imports_documents_from_all_pages() {
        let doc_a = document("doc-a");
        let doc_b = document("doc-b");
        let doc_c = document("doc-c");

        let mut repository = MockSearchRepository::new();
        repository
            .expect_save_document()
            .times(3)
            .returning(|_| Ok(processed_at()));
        repository
            .expect_set_last_import_value()
            .times(1)
            .returning(|_| Ok(()));

        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_snapshot_documents()
            .times(1)
            .withf(|batch_size, page| *batch_size == BATCH_SIZE && *page == 0)
            .return_once(move |_, _| Ok((vec![doc_a, doc_b], RemainingDocuments::from(true))));
        documents
            .expect_snapshot_documents()
            .times(1)
            .withf(|batch_size, page| *batch_size == BATCH_SIZE && *page == 1)
            .return_once(move |_, _| Ok((vec![doc_c], RemainingDocuments::from(false))));

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        service
            .import_existing_documents_for_service(documents)
            .await
            .expect("snapshot should succeed");
    }

    #[tokio::test]
    async fn test_snapshot_existing_documents_imports_nothing_when_first_page_has_no_more() {
        let mut repository = MockSearchRepository::new();
        repository.expect_save_document().times(0);
        repository
            .expect_set_last_import_value()
            .times(1)
            .returning(|_| Ok(()));

        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_snapshot_documents()
            .times(1)
            .withf(|batch_size, page| *batch_size == BATCH_SIZE && *page == 0)
            .return_once(|_, _| Ok((vec![], RemainingDocuments::from(false))));

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        service
            .import_existing_documents_for_service(documents)
            .await
            .expect("snapshot should succeed");
    }

    #[tokio::test]
    async fn test_snapshot_existing_documents_propagates_error_from_first_page() {
        let mut repository = MockSearchRepository::new();
        repository.expect_save_document().times(0);

        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_snapshot_documents()
            .times(1)
            .withf(|batch_size, page| *batch_size == BATCH_SIZE && *page == 0)
            .return_once(|_, _| Err(anyhow!("failed to fetch snapshot")));

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        let res = service
            .import_existing_documents_for_service(documents)
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_snapshot_existing_documents_stops_paginating_when_fetching_a_page_fails() {
        let doc_a = document("doc-a");

        let mut repository = MockSearchRepository::new();
        repository
            .expect_save_document()
            .times(1)
            .returning(|_| Ok(processed_at()));

        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_snapshot_documents()
            .times(1)
            .withf(|batch_size, page| *batch_size == BATCH_SIZE && *page == 0)
            .return_once(move |_, _| Ok((vec![doc_a], RemainingDocuments::from(true))));
        documents
            .expect_snapshot_documents()
            .times(1)
            .withf(|batch_size, page| *batch_size == BATCH_SIZE && *page == 1)
            .return_once(|_, _| Err(anyhow!("failed to fetch snapshot")));
        // The error on page 1 must stop pagination: no page 2 is requested and
        // the whole snapshot is reported as failed.
        documents
            .expect_snapshot_documents()
            .times(0)
            .withf(|_, page| *page == 2);

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        let res = service
            .import_existing_documents_for_service(documents)
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_snapshot_existing_documents_continues_when_saving_a_document_fails() {
        let doc_a = document("doc-a");
        let doc_b = document("doc-b");
        let doc_c = document("doc-c");

        let mut repository = MockSearchRepository::new();
        repository.expect_save_document().times(3).returning(|doc| {
            if doc.document_id() == "doc-a" {
                Err(anyhow!("failed to save document"))
            } else {
                Ok(processed_at())
            }
        });
        repository
            .expect_set_last_import_value()
            .times(1)
            .returning(|_| Ok(()));

        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_snapshot_documents()
            .times(1)
            .withf(|batch_size, page| *batch_size == BATCH_SIZE && *page == 0)
            .return_once(move |_, _| Ok((vec![doc_a, doc_b], RemainingDocuments::from(true))));
        documents
            .expect_snapshot_documents()
            .times(1)
            .withf(|batch_size, page| *batch_size == BATCH_SIZE && *page == 1)
            .return_once(move |_, _| Ok((vec![doc_c], RemainingDocuments::from(false))));

        let service = build_service_skip_import(
            repository,
            MockDocumentsForSearch::new(),
            MockDocumentsForSearch::new(),
        );

        // Save failures for individual documents are tolerated: the snapshot
        // keeps importing the remaining documents (including later pages).
        service
            .import_existing_documents_for_service(documents)
            .await
            .expect("snapshot should succeed");
    }

    #[tokio::test]
    async fn test_import_existing_documents_skips_import_when_last_import_value_is_set() {
        let mut repository = MockSearchRepository::new();
        repository.expect_clone().times(0);
        repository.expect_save_document().times(0);

        // The repository reports that an import already happened, so the
        // background import must never be spawned: no document source is
        // cloned and no snapshot is taken.
        let mut activity = MockDocumentsForSearch::new();
        activity.expect_clone().times(0);
        activity.expect_snapshot_documents().times(0);

        let mut training = MockDocumentsForSearch::new();
        training.expect_clone().times(0);
        training.expect_snapshot_documents().times(0);

        let service = build_service_skip_import(repository, activity, training);

        service
            .import_existing_documents()
            .await
            .expect("skipping an already-done import should succeed");
    }

    #[tokio::test]
    async fn test_import_existing_documents_propagates_error_from_get_last_import_value() {
        let mut repository = MockSearchRepository::new();
        repository
            .expect_get_last_import_value()
            .returning(|| Err(anyhow!("failed to read last import value")));

        let service = SearchService::new(
            repository,
            std::sync::Arc::new(tokio::sync::Notify::new()),
            MockDocumentsForSearch::new(),
            std::sync::Arc::new(tokio::sync::Notify::new()),
            MockDocumentsForSearch::new(),
            tokio_util::sync::CancellationToken::new(),
            FakeClock::default(),
        );

        let res = service.import_existing_documents().await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_snapshot_existing_documents_sets_last_import_value_after_successful_import() {
        let doc_a = document("doc-a");
        let doc_b = document("doc-b");

        let mut repository = MockSearchRepository::new();
        repository
            .expect_save_document()
            .times(2)
            .returning(|_| Ok(processed_at()));
        // Once every page has been imported, the repository records the import
        // date from the service clock so future startups skip the import.
        repository
            .expect_set_last_import_value()
            .times(1)
            .withf(|at| *at == processed_at())
            .returning(|_| Ok(()));

        let mut documents = MockDocumentsForSearch::new();
        documents
            .expect_snapshot_documents()
            .times(1)
            .withf(|batch_size, page| *batch_size == BATCH_SIZE && *page == 0)
            .return_once(move |_, _| Ok((vec![doc_a, doc_b], RemainingDocuments::from(false))));

        let service = SearchService::new(
            repository,
            std::sync::Arc::new(tokio::sync::Notify::new()),
            MockDocumentsForSearch::new(),
            std::sync::Arc::new(tokio::sync::Notify::new()),
            MockDocumentsForSearch::new(),
            tokio_util::sync::CancellationToken::new(),
            FakeClock::new(processed_at()),
        );

        service
            .import_existing_documents_for_service(documents)
            .await
            .expect("snapshot should succeed");
    }
}
