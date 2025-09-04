use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset};
use serde::Serialize;

use crate::{
    domain::{
        models::{Activity, ActivityId, Sport},
        ports::ActivityService,
    },
    inbound::{http::AppState, parser::ParseFile},
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResponseBody {
    id: String,
    sport: String,
    duration: usize,
    start_time: DateTime<FixedOffset>,
}

impl From<&Activity> for ResponseBody {
    fn from(activity: &Activity) -> Self {
        Self {
            id: activity.id().to_string(),
            sport: match *activity.sport() {
                Sport::Running => "Running".to_string(),
                Sport::Cycling => "Cycling".to_string(),
                Sport::Other => "Other".to_string(),
            },
            start_time: **activity.start_time(),
            duration: (*activity.duration()).into(),
        }
    }
}

pub async fn get_activity<AS: ActivityService, FP: ParseFile>(
    State(state): State<AppState<AS, FP>>,
    Path(activity_id): Path<String>,
) -> Result<Json<ResponseBody>, StatusCode> {
    let Ok(res) = state
        .activity_service
        .get_activity(&ActivityId::from(&activity_id))
        .await
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let body = ResponseBody::from(&res);

    Ok(Json(body))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::extract::Path;

    use crate::{
        domain::{
            models::{ActivityDuration, ActivityStartTime},
            ports::GetActivityError,
            services::test_utils::MockActivityService,
        },
        inbound::parser::test_utils::MockFileParser,
    };

    use super::*;

    #[tokio::test]
    async fn test_get_activity_exists() {
        let target_id = "target_id".to_string();
        let service = MockActivityService {
            get_activity_result: Arc::new(Mutex::new(Ok(Activity::new(
                ActivityId::from(&target_id),
                ActivityStartTime::new(
                    "2025-09-03T00:00:00Z"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                ),
                ActivityDuration::new(1200),
                Sport::Cycling,
            )))),
            ..Default::default()
        };

        let file_parser = MockFileParser::default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            file_parser: Arc::new(file_parser),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(state, path).await;
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(
            response.0,
            ResponseBody {
                duration: 1200,
                id: target_id,
                sport: "Cycling".to_string(),
                start_time: "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap()
            }
        );
    }

    #[tokio::test]
    async fn test_get_activity_does_not_exist() {
        let target_id = "target_id".to_string();
        let service = MockActivityService {
            get_activity_result: Arc::new(Mutex::new(Err(GetActivityError::ActivityDoesNotExist(
                ActivityId::from(&target_id),
            )))),
            ..Default::default()
        };

        let file_parser = MockFileParser::default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            file_parser: Arc::new(file_parser),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(state, path).await;
        assert!(response.is_err());
        let response = response.unwrap_err();
        assert_eq!(response, StatusCode::NOT_FOUND);
    }
}
