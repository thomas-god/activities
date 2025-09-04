use axum::{body::Bytes, extract::State, http::StatusCode};

use crate::{
    domain::ports::{ActivityService, CreateActivityError},
    inbound::{http::AppState, parser::ParseFile},
};

impl From<CreateActivityError> for StatusCode {
    fn from(_value: CreateActivityError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

pub async fn create_activity<AS: ActivityService, PF: ParseFile>(
    State(state): State<AppState<AS, PF>>,
    bytes: Bytes,
) -> Result<StatusCode, StatusCode> {
    let create_activity_request = state
        .file_parser
        .try_bytes_into_domain(bytes.to_vec())
        .map_err(|err| {
            tracing::warn!("Unable to process fit file {:?}", err);
            StatusCode::UNPROCESSABLE_ENTITY
        })?;

    state
        .activity_service
        .create_activity(create_activity_request)
        .await
        .map(|_| StatusCode::CREATED)
        .map_err(|err| match err {
            CreateActivityError::SimilarActivityExistsError => StatusCode::CONFLICT,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        })
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        vec,
    };

    use crate::{
        domain::{ports::CreateActivityError, services::test_utils::MockActivityService},
        inbound::parser::{ParseCreateActivityHttpRequestBodyError, test_utils::MockFileParser},
    };

    use super::*;

    #[tokio::test]
    async fn test_create_activity() {
        let content = vec![1, 2, 3];
        let service = MockActivityService::default();
        let file_parser = MockFileParser::default();
        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            file_parser: Arc::new(file_parser),
        });
        let bytes = axum::body::Bytes::from(content);

        let response = create_activity(state, bytes).await;
        assert!(response.is_ok());
        assert_eq!(response.unwrap(), StatusCode::CREATED)
    }

    #[tokio::test]
    async fn test_create_activity_fit_parse_fails() {
        let content = vec![1, 2, 3];
        let service = MockActivityService::default();

        let file_parser = MockFileParser {
            try_into_domain_result: Arc::new(Mutex::new(Err(
                ParseCreateActivityHttpRequestBodyError::InvalidFitContent,
            ))),
        };

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            file_parser: Arc::new(file_parser),
        });
        let bytes = axum::body::Bytes::from(content);

        let response = create_activity(state, bytes).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err(), StatusCode::UNPROCESSABLE_ENTITY)
    }

    #[tokio::test]
    async fn test_create_activity_with_similar_already_exists() {
        let content = vec![1, 2, 3];
        let service = MockActivityService {
            create_activity_result: Arc::new(Mutex::new(Err(
                CreateActivityError::SimilarActivityExistsError,
            ))),
            ..Default::default()
        };

        let file_parser = MockFileParser::default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            file_parser: Arc::new(file_parser),
        });
        let bytes = axum::body::Bytes::from(content);

        let response = create_activity(state, bytes).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err(), StatusCode::CONFLICT)
    }
}
