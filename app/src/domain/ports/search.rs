use crate::domain::models::{
    UserId,
    activity::ActivityId,
    search::{SearchDocument, SearchDocumentType},
    training::TrainingNoteId,
};

pub trait IDocumentsForSearch: Clone + Send + Sync + 'static {
    fn get_documents_to_process(
        &self,
    ) -> impl Future<Output = Result<Vec<SearchDocument>, anyhow::Error>> + Send;

    fn mark_document_as_processed(
        &self,
        document: &SearchDocument,
        processed_at: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn service_kind(&self) -> SearchDocumentType;
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SearchResult {
    Activity(ActivityId),
    TrainingNote(TrainingNoteId),
}

pub trait ISearchService: Clone + Send + Sync + 'static {
    fn search(
        &self,
        user: &UserId,
        pattern: String,
    ) -> impl Future<Output = Result<Vec<SearchResult>, anyhow::Error>> + Send;
}

pub trait ISearchRepository: Clone + Send + Sync + 'static {
    fn search(
        &self,
        user: &UserId,
        pattern: String,
    ) -> impl Future<Output = Result<Vec<SearchResult>, anyhow::Error>> + Send;

    fn save_document(
        &self,
        document: &SearchDocument,
    ) -> impl Future<Output = Result<chrono::DateTime<chrono::Utc>, anyhow::Error>> + Send;
}

#[cfg(test)]
pub mod search_test_utils {
    use mockall::mock;

    use super::*;

    mock! {
        pub SearchService {}

        impl Clone for SearchService {
            fn clone(&self) -> Self;
        }

        impl ISearchService for SearchService {
            async fn search(
                  &self,
                  user: &UserId,
                  pattern: String,
              ) -> Result<Vec<SearchResult>, anyhow::Error>;
        }
    }

    mock! {
        pub SearchRepository {}

        impl Clone for SearchRepository {
            fn clone(&self) -> Self;
        }

        impl ISearchRepository for SearchRepository {
            async fn search(
                &self,
                user: &UserId,
                pattern: String,
            ) -> Result<Vec<SearchResult>, anyhow::Error>;

            async fn save_document(
                &self,
                document: &SearchDocument,
            ) -> Result<chrono::DateTime<chrono::Utc>, anyhow::Error>;
        }
    }

    mock! {
        pub DocumentsForSearch {}

        impl Clone for DocumentsForSearch {
            fn clone(&self) -> Self;
        }

        impl IDocumentsForSearch for DocumentsForSearch {
            async fn get_documents_to_process(
                &self,
            ) -> Result<Vec<SearchDocument>, anyhow::Error>;

            async fn mark_document_as_processed(
                &self,
                document: &SearchDocument,
                processed_at: chrono::DateTime<chrono::Utc>,
            ) -> Result<(), anyhow::Error>;

            fn service_kind(&self) -> SearchDocumentType;
        }
    }
}
