use axum::{Json, extract::State, http::StatusCode};
use fit_parser::{FitParserError, parse_fit_messages, utils::find_field_value_as_uint};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    domain::ports::{ActivityService, CreateActivityError, CreateActivityRequest},
    inbound::http::AppState,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateActivityHttpRequestBody {
    content: Vec<u8>,
}

impl CreateActivityHttpRequestBody {
    fn try_into_domain(
        self,
    ) -> Result<CreateActivityRequest, ParseCreateActivityHttpRequestBodyError> {
        let Ok(messages) = parse_fit_messages(self.content.clone().into_iter()) else {
            return Err(ParseCreateActivityHttpRequestBodyError::InvalidFitContent);
        };
        let calories = find_field_value_as_uint(
            &messages,
            &fit_parser::FitField::Session(fit_parser::SessionField::TotalCalories),
        );

        Ok(CreateActivityRequest::new(calories, self.content))
    }
}

#[derive(Debug, Clone, Error)]
enum ParseCreateActivityHttpRequestBodyError {
    #[error("Error when parsing FIT content")]
    InvalidFitContent,
}

impl From<FitParserError> for ParseCreateActivityHttpRequestBodyError {
    fn from(_value: FitParserError) -> Self {
        Self::InvalidFitContent
    }
}

impl From<CreateActivityError> for StatusCode {
    fn from(_value: CreateActivityError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

pub async fn create_activity<AS: ActivityService>(
    State(state): State<AppState<AS>>,
    Json(body): Json<CreateActivityHttpRequestBody>,
) -> Result<StatusCode, StatusCode> {
    let domain_request = body
        .try_into_domain()
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

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
        fs, mem,
        sync::{Arc, Mutex},
    };

    use anyhow::anyhow;

    use crate::domain::{
        models::{Activity, ActivityId},
        ports::CreateActivityError,
    };

    use super::*;

    #[derive(Clone)]
    struct MockActivityService {
        create_activity_result: Arc<Mutex<Result<Activity, CreateActivityError>>>,
    }

    impl ActivityService for MockActivityService {
        async fn create_activity(
            &self,
            _req: CreateActivityRequest,
        ) -> Result<Activity, CreateActivityError> {
            let mut guard = self.create_activity_result.lock();
            let mut result = Err(CreateActivityError::Unknown(anyhow!("Substitute errror")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }
    }

    #[tokio::test]
    async fn test_create_activity() {
        let content = fs::read("../test.fit").unwrap().to_vec();
        let service = MockActivityService {
            create_activity_result: Arc::new(Mutex::new(Ok(Activity::new(
                ActivityId::new(),
                Some(12),
            )))),
        };

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
        });
        let body = axum::extract::Json(CreateActivityHttpRequestBody { content });

        let response = create_activity(state, body).await;
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
        };

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
        });
        let body = axum::extract::Json(CreateActivityHttpRequestBody { content });

        let response = create_activity(state, body).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err(), StatusCode::UNPROCESSABLE_ENTITY)
    }
}
