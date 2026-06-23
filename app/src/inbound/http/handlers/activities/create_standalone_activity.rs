use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::domain::ports::{preferences::IPreferencesService, training::ITrainingService};
use crate::{
    domain::ports::activity::IActivityService,
    inbound::{
        http::{AppState, auth::AuthenticatedUser},
        parser::{ParseBytesError, ParseFile, ParsedFileContent, json::StandaloneActivity},
    },
};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStandaloneActivityResponse {
    id: String,
}

pub async fn create_standalone_activity<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Json(activity): Json<StandaloneActivity>,
) -> Result<impl axum::response::IntoResponse, StatusCode> {
    let parsed_content: ParsedFileContent = activity
        .try_into()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let create_request = parsed_content.into_request(user.user());

    let activity = state
        .activity_service
        .create_activity(create_request)
        .await
        .map_err(StatusCode::from)?;

    Ok((
        StatusCode::CREATED,
        Json(CreateStandaloneActivityResponse {
            id: activity.id().to_string(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{Router, middleware::from_extractor, routing::post};
    use axum_test::TestServer;

    use crate::domain::models::activity::Activity;
    use crate::{
        domain::{
            models::{
                UserId,
                activity::{ActivityDuration, ActivityId, ActivityStartTime, Sport},
            },
            services::{
                activity::test_utils::MockActivityService,
                preferences::tests_utils::MockPreferencesService,
                training::test_utils::MockTrainingService,
            },
        },
        inbound::{http::handlers::auth::DefaultUserExtractor, parser::test_utils::MockFileParser},
    };

    use super::*;

    /// Documents the expected JSON payload for POST /activity/standalone.
    ///
    /// Required fields: start_time, duration, sport
    /// Optional fields: distance, elevation, calories (all in SI units)
    #[test]
    fn test_payload_format() {
        // minimal — only required fields
        assert!(
            serde_json::from_str::<StandaloneActivity>(
                r#"{
                    "start_time": "2024-03-15T08:30:00+01:00",
                    "duration": 3600.0,
                    "sport": "Running"
                }"#
            )
            .is_ok()
        );

        // with all optional fields
        assert!(
            serde_json::from_str::<StandaloneActivity>(
                r#"{
                    "start_time": "2024-06-01T06:00:00+00:00",
                    "duration": 7200.0,
                    "sport": "Cycling",
                    "distance": 80000.0,
                    "elevation": 500.0,
                    "calories": 1200.0
                }"#
            )
            .is_ok()
        );

        // missing required field
        assert!(
            serde_json::from_str::<StandaloneActivity>(
                r#"{"duration": 3600.0, "sport": "Running"}"#
            )
            .is_err()
        );

        // unknown sport variant
        assert!(
            serde_json::from_str::<StandaloneActivity>(
                r#"{"start_time": "2024-03-15T08:30:00+01:00", "duration": 3600.0, "sport": "Paragliding"}"#
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn test_create_standalone_activity_returns_created_with_id() {
        let expected_id = ActivityId::new();
        let expected_id_clone = expected_id.clone();

        let mut service = MockActivityService::new();
        service
            .expect_create_activity()
            .times(1)
            .returning(move |_| {
                Ok(Activity::new_empty(
                    expected_id_clone.clone(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityDuration::default(),
                    Sport::Running,
                ))
            });

        let state = AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(MockTrainingService::test_default()),
            file_parser: Arc::new(MockFileParser::test_default()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        let app = Router::new()
            .route(
                "/activity/standalone",
                post(
                    create_standalone_activity::<
                        MockActivityService,
                        MockFileParser,
                        MockTrainingService,
                        MockPreferencesService,
                    >,
                ),
            )
            .route_layer(from_extractor::<DefaultUserExtractor>())
            .with_state(state);

        let server = TestServer::new(app).expect("unable to create test server");

        let response = server
            .post("/activity/standalone")
            .json(&serde_json::json!({
                "start_time": "2024-03-15T08:30:00+01:00",
                "duration": 3600.0,
                "sport": "Running"
            }))
            .await;

        response.assert_status(StatusCode::CREATED);
        let body: CreateStandaloneActivityResponse = response.json();
        assert_eq!(body.id, expected_id.to_string());
    }

    #[tokio::test]
    async fn test_create_standalone_activity_invalid_json_returns_422() {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::test_default()),
            training_metrics_service: Arc::new(MockTrainingService::test_default()),
            file_parser: Arc::new(MockFileParser::test_default()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        let app = Router::new()
            .route(
                "/activity/standalone",
                post(
                    create_standalone_activity::<
                        MockActivityService,
                        MockFileParser,
                        MockTrainingService,
                        MockPreferencesService,
                    >,
                ),
            )
            .route_layer(from_extractor::<DefaultUserExtractor>())
            .with_state(state);

        let server = TestServer::new(app).expect("unable to create test server");

        // missing required field
        let response = server
            .post("/activity/standalone")
            .json(&serde_json::json!({
                "duration": 3600.0,
                "sport": "Running"
            }))
            .await;

        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }
}
