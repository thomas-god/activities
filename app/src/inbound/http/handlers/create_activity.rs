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
    let domain_request = state
        .file_parser
        .try_bytes_into_domain(bytes.to_vec())
        .map_err(|err| {
            tracing::warn!("Unable to process fit file {:?}", err);
            StatusCode::UNPROCESSABLE_ENTITY
        })?;

    state
        .activity_service
        .create_activity(domain_request)
        .await
        .map(|_| StatusCode::CREATED)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        vec,
    };

    use anyhow::anyhow;

    use crate::{
        domain::{
            models::{Activity, ActivityDuration, ActivityId, ActivityStartTime, Sport},
            ports::{CreateActivityError, CreateActivityRequest},
            services::test_utils::MockActivityService,
        },
        inbound::parser::{ParseCreateActivityHttpRequestBodyError, test_utils::MockFileParser},
    };

    use super::*;

    #[tokio::test]
    async fn test_create_activity() {
        let content = vec![1, 2, 3];
        let sport = Sport::Cycling;
        let start_time = ActivityStartTime::new(0).unwrap();
        let duration = ActivityDuration(3600);

        let service = MockActivityService {
            create_activity_result: Arc::new(Mutex::new(Ok(Activity::new(
                ActivityId::new(),
                start_time,
                duration,
                sport,
            )))),
            list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
        };
        let file_parser = MockFileParser {
            try_into_domain_result: Arc::new(Mutex::new(Ok(CreateActivityRequest::new(
                sport,
                duration,
                start_time,
                content.clone(),
            )))),
        };

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
        let service = MockActivityService {
            create_activity_result: Arc::new(Mutex::new(Err(CreateActivityError::Unknown(
                anyhow!("Should not be reached"),
            )))),
            list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
        };

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
}
