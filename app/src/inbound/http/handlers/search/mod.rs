use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::domain::ports::search::{ISearchService, SearchResult};
use crate::inbound::auth::AuthenticatedUser;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pattern: String,
}

/// JSON response shape:
///
/// ```json
/// [
///   { "kind": "activity", "id": "activity-1" },
///   { "kind": "training_note", "id": "note-1" }
/// ]
/// ```
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum SearchResultResponse {
    Activity(String),
    TrainingNote(String),
}

impl From<SearchResult> for SearchResultResponse {
    fn from(result: SearchResult) -> Self {
        match result {
            SearchResult::Activity(id) => Self::Activity(id.to_string()),
            SearchResult::TrainingNote(id) => Self::TrainingNote(id.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, err)]
pub async fn search<SS: ISearchService>(
    Extension(user): Extension<AuthenticatedUser>,
    State(search_service): State<Arc<SS>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResultResponse>>, StatusCode> {
    match search_service.search(user.user(), query.pattern).await {
        Ok(results) => Ok(Json(
            results
                .into_iter()
                .map(SearchResultResponse::from)
                .collect(),
        )),
        Err(err) => {
            tracing::error!("Error while searching: {}", err.to_string());
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use axum::extract::Query;

    use super::*;
    use crate::domain::{
        models::{UserId, activity::ActivityId, training::TrainingNoteId},
        ports::search::search_test_utils::MockSearchService,
    };

    #[tokio::test]
    async fn test_search_success() {
        let user = UserId::test_default();
        let expected_user = user.clone();

        let mut search_service = MockSearchService::new();
        search_service
            .expect_search()
            .withf(move |user, pattern| user == &expected_user && pattern == "long ride")
            .times(1)
            .returning(|_, _| {
                Ok(vec![
                    SearchResult::Activity(ActivityId::from("activity-1")),
                    SearchResult::TrainingNote(TrainingNoteId::from("note-1")),
                ])
            });

        let result = search(
            Extension(AuthenticatedUser::new(user)),
            State(Arc::new(search_service)),
            Query(SearchQuery {
                pattern: "long ride".to_string(),
            }),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().0,
            vec![
                SearchResultResponse::Activity("activity-1".to_string()),
                SearchResultResponse::TrainingNote("note-1".to_string()),
            ]
        );
    }

    #[test]
    fn search_result_response_serializes_to_expected_json() {
        let responses = vec![
            SearchResultResponse::Activity("activity-1".to_string()),
            SearchResultResponse::TrainingNote("note-1".to_string()),
        ];

        assert_eq!(
            serde_json::to_value(responses).unwrap(),
            serde_json::json!([
                { "kind": "activity", "id": "activity-1" },
                { "kind": "training_note", "id": "note-1" }
            ])
        );
    }

    #[tokio::test]
    async fn test_search_propagates_service_error() {
        let mut search_service = MockSearchService::new();
        search_service
            .expect_search()
            .times(1)
            .returning(|_, _| Err(anyhow!("failed to search")));

        let result = search(
            Extension(AuthenticatedUser::new(UserId::test_default())),
            State(Arc::new(search_service)),
            Query(SearchQuery {
                pattern: "ride".to_string(),
            }),
        )
        .await;

        assert_eq!(result.unwrap_err(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
