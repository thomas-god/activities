use crate::domain::models::search::SearchDocument;

pub trait IDocumentsForSearch {
    fn get_documents_to_process(
        &self,
    ) -> impl Future<Output = Result<Vec<SearchDocument>, anyhow::Error>> + Send;

    fn mark_document_as_processed(
        &self,
        document: &SearchDocument,
        processed_at: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;
}
