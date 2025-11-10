use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    domain::{
        models::activity::{
            ActivityFeedback, ActivityId, ActivityName, ActivityNutrition, ActivityRpe, BonkStatus,
            WorkoutType,
        },
        ports::{
            IActivityService, IPreferencesService, ITrainingService, ModifyActivityError,
            ModifyActivityRequest, UpdateActivityFeedbackError, UpdateActivityFeedbackRequest,
            UpdateActivityNutritionError, UpdateActivityNutritionRequest, UpdateActivityRpeError,
            UpdateActivityRpeRequest, UpdateActivityWorkoutTypeError,
            UpdateActivityWorkoutTypeRequest, UpdateMetricsForActivityRequest,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::ParseFile,
    },
};

impl From<ModifyActivityError> for StatusCode {
    fn from(value: ModifyActivityError) -> Self {
        match value {
            ModifyActivityError::ActivityDoesNotExist(_) => Self::NOT_FOUND,
            _ => Self::UNPROCESSABLE_ENTITY,
        }
    }
}

impl From<UpdateActivityRpeError> for StatusCode {
    fn from(value: UpdateActivityRpeError) -> Self {
        match value {
            UpdateActivityRpeError::ActivityDoesNotExist(_) => Self::NOT_FOUND,
            _ => Self::UNPROCESSABLE_ENTITY,
        }
    }
}

impl From<UpdateActivityWorkoutTypeError> for StatusCode {
    fn from(value: UpdateActivityWorkoutTypeError) -> Self {
        match value {
            UpdateActivityWorkoutTypeError::ActivityDoesNotExist(_) => Self::NOT_FOUND,
            _ => Self::UNPROCESSABLE_ENTITY,
        }
    }
}

impl From<UpdateActivityNutritionError> for StatusCode {
    fn from(value: UpdateActivityNutritionError) -> Self {
        match value {
            UpdateActivityNutritionError::ActivityDoesNotExist(_) => Self::NOT_FOUND,
            _ => Self::UNPROCESSABLE_ENTITY,
        }
    }
}

impl From<UpdateActivityFeedbackError> for StatusCode {
    fn from(value: UpdateActivityFeedbackError) -> Self {
        match value {
            UpdateActivityFeedbackError::ActivityDoesNotExist(_) => Self::NOT_FOUND,
            _ => Self::UNPROCESSABLE_ENTITY,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PatchActivityQuery {
    /// Optional new name for the activity
    name: Option<String>,
    /// Optional RPE (Rate of Perceived Exertion) value from 1-10
    /// Use 0 to clear/remove the RPE value
    rpe: Option<u8>,
    /// Optional workout type: easy, tempo, intervals, long_run, or race
    /// Use empty string to clear/remove the workout type
    workout_type: Option<String>,
    /// Optional bonk status: none or bonked
    /// Use empty string to clear/remove the nutrition info
    bonk_status: Option<String>,
    /// Optional nutrition details/notes
    nutrition_details: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PatchActivityBody {
    /// Optional feedback/notes about the activity
    /// Use empty string or null to clear/remove the feedback
    feedback: Option<String>,
}

/// Handler for PATCH /api/activity/{activity_id}
///
/// Updates an activity's metadata. Currently supports:
/// - `name`: Change the activity name (query parameter)
/// - `rpe`: Set RPE from 1-10, or use 0 to clear it (query parameter)
/// - `workout_type`: Set workout type (easy, tempo, intervals, long_run, race), or empty string to clear (query parameter)
/// - `bonk_status`: Set bonk status (none, bonked), or empty string to clear nutrition info (query parameter)
/// - `nutrition_details`: Optional details about nutrition/hydration (query parameter)
/// - `feedback`: Optional feedback/notes about the activity (request body, JSON)
///
/// # Example
/// PATCH /api/activity/123?rpe=7
/// PATCH /api/activity/123?name=Morning%20Run&rpe=8
/// PATCH /api/activity/123?rpe=0  // Clear RPE
/// PATCH /api/activity/123?workout_type=intervals
/// PATCH /api/activity/123?workout_type=  // Clear workout type
/// PATCH /api/activity/123?bonk_status=bonked&nutrition_details=Forgot%20to%20eat
/// PATCH /api/activity/123?bonk_status=none
/// PATCH /api/activity/123?bonk_status=  // Clear nutrition info
///
/// With request body for feedback:
/// PATCH /api/activity/123
/// Body: {"feedback": "Great run, felt strong throughout!"}
///
/// To clear feedback:
/// Body: {"feedback": ""} or {"feedback": null}
pub async fn patch_activity<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Path(activity_id): Path<String>,
    Query(query): Query<PatchActivityQuery>,
    body: Option<Json<PatchActivityBody>>,
) -> Result<StatusCode, StatusCode> {
    // Update activity name if provided
    if let Some(name) = query.name {
        let req = ModifyActivityRequest::new(
            user.user().clone(),
            ActivityId::from(&activity_id),
            Some(ActivityName::new(name)),
        );

        state
            .activity_service
            .modify_activity(req)
            .await
            .map_err(StatusCode::from)?;
    }

    // Update activity RPE if provided
    if let Some(rpe_value) = query.rpe {
        let rpe = if rpe_value == 0 {
            None
        } else {
            Some(ActivityRpe::try_from(rpe_value).map_err(|_| StatusCode::BAD_REQUEST)?)
        };

        let req =
            UpdateActivityRpeRequest::new(user.user().clone(), ActivityId::from(&activity_id), rpe);

        state
            .activity_service
            .update_activity_rpe(req)
            .await
            .map_err(StatusCode::from)?;
    }

    // Update activity workout type if provided
    if let Some(workout_type_str) = query.workout_type {
        let workout_type = if workout_type_str.is_empty() {
            None
        } else {
            Some(
                workout_type_str
                    .parse::<WorkoutType>()
                    .map_err(|_| StatusCode::BAD_REQUEST)?,
            )
        };

        let req = UpdateActivityWorkoutTypeRequest::new(
            user.user().clone(),
            ActivityId::from(&activity_id),
            workout_type,
        );

        state
            .activity_service
            .update_activity_workout_type(req)
            .await
            .map_err(StatusCode::from)?;
    }

    // Update activity nutrition if bonk_status is provided
    if let Some(bonk_status_str) = query.bonk_status {
        let nutrition = if bonk_status_str.is_empty() {
            None
        } else {
            let bonk_status = bonk_status_str
                .parse::<BonkStatus>()
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            // If nutrition_details is provided and not empty, use it; otherwise None
            let details = query.nutrition_details.filter(|d| !d.is_empty());

            Some(ActivityNutrition::new(bonk_status, details))
        };

        let req = UpdateActivityNutritionRequest::new(
            user.user().clone(),
            ActivityId::from(&activity_id),
            nutrition,
        );

        state
            .activity_service
            .update_activity_nutrition(req)
            .await
            .map_err(StatusCode::from)?;
    }

    // Update activity feedback if provided in request body
    if let Some(Json(body)) = body
        && let Some(feedback_str) = body.feedback
    {
        let feedback = if feedback_str.is_empty() {
            None
        } else {
            Some(ActivityFeedback::from(feedback_str))
        };

        let req = UpdateActivityFeedbackRequest::new(
            user.user().clone(),
            ActivityId::from(&activity_id),
            feedback,
        );

        state
            .activity_service
            .update_activity_feedback(req)
            .await
            .map_err(StatusCode::from)?;
    }

    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpe_value_0_converts_to_none() {
        // Test that RPE value 0 is treated as "clear RPE" (None)
        let query = PatchActivityQuery {
            name: None,
            rpe: Some(0),
            workout_type: None,
            bonk_status: None,
            nutrition_details: None,
        };
        assert_eq!(query.rpe, Some(0));
    }

    #[test]
    fn test_rpe_valid_values() {
        // Test that valid RPE values (1-10) can be parsed
        for i in 1..=10 {
            let result = ActivityRpe::try_from(i);
            assert!(result.is_ok(), "RPE value {} should be valid", i);
        }
    }

    #[test]
    fn test_rpe_invalid_values() {
        // Test that invalid RPE values are rejected
        for i in [11, 12, 100, 255] {
            let result = ActivityRpe::try_from(i);
            assert!(result.is_err(), "RPE value {} should be invalid", i);
        }
    }

    #[test]
    fn test_workout_type_valid_values() {
        // Test that valid workout type values can be parsed
        let valid_types = ["easy", "tempo", "intervals", "long_run", "race"];
        for workout_type in valid_types {
            let result = workout_type.parse::<WorkoutType>();
            assert!(
                result.is_ok(),
                "Workout type '{}' should be valid",
                workout_type
            );
        }
    }

    #[test]
    fn test_workout_type_invalid_values() {
        // Test that invalid workout type values are rejected
        let invalid_types = ["invalid", "sprint", "recovery", "123"];
        for workout_type in invalid_types {
            let result = workout_type.parse::<WorkoutType>();
            assert!(
                result.is_err(),
                "Workout type '{}' should be invalid",
                workout_type
            );
        }
    }

    #[test]
    fn test_workout_type_empty_string() {
        // Test that empty string is handled (should clear workout type)
        let query = PatchActivityQuery {
            name: None,
            rpe: None,
            workout_type: Some(String::new()),
            bonk_status: None,
            nutrition_details: None,
        };
        assert_eq!(query.workout_type, Some(String::new()));
    }

    #[test]
    fn test_bonk_status_valid_values() {
        // Test that valid bonk status values can be parsed
        let valid_statuses = ["none", "bonked"];
        for status in valid_statuses {
            let result = status.parse::<BonkStatus>();
            assert!(result.is_ok(), "Bonk status '{}' should be valid", status);
        }
    }

    #[test]
    fn test_bonk_status_invalid_values() {
        // Test that invalid bonk status values are rejected
        let invalid_statuses = ["invalid", "mild", "severe", "123"];
        for status in invalid_statuses {
            let result = status.parse::<BonkStatus>();
            assert!(
                result.is_err(),
                "Bonk status '{}' should be invalid",
                status
            );
        }
    }

    #[test]
    fn test_bonk_status_empty_string() {
        // Test that empty string is handled (should clear nutrition)
        let query = PatchActivityQuery {
            name: None,
            rpe: None,
            workout_type: None,
            bonk_status: Some(String::new()),
            nutrition_details: None,
        };
        assert_eq!(query.bonk_status, Some(String::new()));
    }

    #[test]
    fn test_feedback_body_parsing() {
        // Test that feedback body can be parsed
        let body = PatchActivityBody {
            feedback: Some("Great run!".to_string()),
        };
        assert_eq!(body.feedback, Some("Great run!".to_string()));

        // Test empty feedback (to clear)
        let body = PatchActivityBody {
            feedback: Some(String::new()),
        };
        assert_eq!(body.feedback, Some(String::new()));

        // Test null feedback
        let body = PatchActivityBody { feedback: None };
        assert_eq!(body.feedback, None);
    }

    // Integration tests for the handler
    use std::sync::Arc;

    use crate::{
        domain::{
            models::{
                UserId,
                activity::{Activity, ActivityStartTime, ActivityStatistics, Sport},
            },
            services::{
                activity::test_utils::MockActivityService,
                preferences::tests_utils::MockPreferencesService,
                training::test_utils::MockTrainingService,
            },
        },
        inbound::{
            http::{CookieConfig, auth::test_utils::MockUserService},
            parser::test_utils::MockFileParser,
        },
    };
    use mockall::predicate::*;

    fn create_test_state(
        activity_service: MockActivityService,
    ) -> AppState<
        MockActivityService,
        MockFileParser,
        MockTrainingService,
        MockUserService,
        MockPreferencesService,
    > {
        AppState {
            activity_service: Arc::new(activity_service),
            file_parser: Arc::new(MockFileParser::test_default()),
            training_metrics_service: Arc::new(MockTrainingService::test_default()),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        }
    }

    #[tokio::test]
    async fn test_patch_activity_update_feedback_ok() {
        let activity_id = "test_activity_id".to_string();
        let activity_id_clone = activity_id.clone();
        let user_id = UserId::from("test_user");

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_update_activity_feedback()
            .with(function(move |req: &UpdateActivityFeedbackRequest| {
                req.user() == &user_id
                    && req.activity() == &ActivityId::from(&activity_id_clone)
                    && req.feedback() == &Some(ActivityFeedback::from("Great session today!"))
            }))
            .times(1)
            .returning(|_| Ok(()));

        let state = create_test_state(activity_service);

        let user = AuthenticatedUser::new(UserId::from("test_user"));
        let path = Path(activity_id);
        let query = Query(PatchActivityQuery {
            name: None,
            rpe: None,
            workout_type: None,
            bonk_status: None,
            nutrition_details: None,
        });
        let body = Some(Json(PatchActivityBody {
            feedback: Some("Great session today!".to_string()),
        }));

        let result = patch_activity(Extension(user), State(state), path, query, body).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_clear_feedback() {
        let activity_id = "test_activity_id".to_string();
        let activity_id_clone = activity_id.clone();
        let user_id = UserId::from("test_user");

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_update_activity_feedback()
            .with(function(move |req: &UpdateActivityFeedbackRequest| {
                req.user() == &user_id
                    && req.activity() == &ActivityId::from(&activity_id_clone)
                    && req.feedback().is_none()
            }))
            .times(1)
            .returning(|_| Ok(()));

        let state = create_test_state(activity_service);

        let user = AuthenticatedUser::new(UserId::from("test_user"));
        let path = Path(activity_id);
        let query = Query(PatchActivityQuery {
            name: None,
            rpe: None,
            workout_type: None,
            bonk_status: None,
            nutrition_details: None,
        });
        let body = Some(Json(PatchActivityBody {
            feedback: Some(String::new()), // Empty string clears feedback
        }));

        let result = patch_activity(Extension(user), State(state), path, query, body).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_no_feedback_in_body() {
        let activity_id = "test_activity_id".to_string();

        let mut activity_service = MockActivityService::new();
        // Should not call update_activity_feedback when body has feedback: None
        activity_service.expect_update_activity_feedback().times(0);

        let state = create_test_state(activity_service);

        let user = AuthenticatedUser::new(UserId::from("test_user"));
        let path = Path(activity_id);
        let query = Query(PatchActivityQuery {
            name: None,
            rpe: None,
            workout_type: None,
            bonk_status: None,
            nutrition_details: None,
        });
        let body = Some(Json(PatchActivityBody { feedback: None }));

        let result = patch_activity(Extension(user), State(state), path, query, body).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_no_body() {
        let activity_id = "test_activity_id".to_string();

        let mut activity_service = MockActivityService::new();
        // Should not call update_activity_feedback when no body is provided
        activity_service.expect_update_activity_feedback().times(0);

        let state = create_test_state(activity_service);

        let user = AuthenticatedUser::new(UserId::from("test_user"));
        let path = Path(activity_id);
        let query = Query(PatchActivityQuery {
            name: None,
            rpe: None,
            workout_type: None,
            bonk_status: None,
            nutrition_details: None,
        });

        let body = None;

        let result = patch_activity(Extension(user), State(state), path, query, body).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_feedback_activity_not_found() {
        let activity_id = "nonexistent_activity".to_string();

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_update_activity_feedback()
            .times(1)
            .returning(|req| {
                Err(UpdateActivityFeedbackError::ActivityDoesNotExist(
                    req.activity().clone(),
                ))
            });

        let state = create_test_state(activity_service);

        let user = AuthenticatedUser::new(UserId::from("test_user"));
        let path = Path(activity_id);
        let query = Query(PatchActivityQuery {
            name: None,
            rpe: None,
            workout_type: None,
            bonk_status: None,
            nutrition_details: None,
        });
        let body = Some(Json(PatchActivityBody {
            feedback: Some("This won't work".to_string()),
        }));

        let result = patch_activity(Extension(user), State(state), path, query, body).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_patch_activity_feedback_wrong_user() {
        let activity_id = "test_activity_id".to_string();
        let user_id = UserId::from("test_user");

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_update_activity_feedback()
            .times(1)
            .returning(move |req| {
                Err(UpdateActivityFeedbackError::UserDoesNotOwnActivity(
                    user_id.clone(),
                    req.activity().clone(),
                ))
            });

        let state = create_test_state(activity_service);

        let user = AuthenticatedUser::new(UserId::from("test_user"));
        let path = Path(activity_id);
        let query = Query(PatchActivityQuery {
            name: None,
            rpe: None,
            workout_type: None,
            bonk_status: None,
            nutrition_details: None,
        });
        let body = Some(Json(PatchActivityBody {
            feedback: Some("Wrong user feedback".to_string()),
        }));

        let result = patch_activity(Extension(user), State(state), path, query, body).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_patch_activity_long_feedback() {
        let activity_id = "test_activity_id".to_string();
        let activity_id_clone = activity_id.clone();
        let user_id = UserId::from("test_user");
        let long_feedback = "This is a very long feedback message. ".repeat(100); // ~3800 chars

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_update_activity_feedback()
            .with(function(move |req: &UpdateActivityFeedbackRequest| {
                req.user() == &user_id
                    && req.activity() == &ActivityId::from(&activity_id_clone)
                    && req.feedback().is_some()
            }))
            .times(1)
            .returning(|_| Ok(()));

        let state = create_test_state(activity_service);

        let user = AuthenticatedUser::new(UserId::from("test_user"));
        let path = Path(activity_id);
        let query = Query(PatchActivityQuery {
            name: None,
            rpe: None,
            workout_type: None,
            bonk_status: None,
            nutrition_details: None,
        });
        let body = Some(Json(PatchActivityBody {
            feedback: Some(long_feedback),
        }));

        let result = patch_activity(Extension(user), State(state), path, query, body).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_combine_rpe_and_feedback() {
        let activity_id = "test_activity_id".to_string();
        let activity_id_clone = activity_id.clone();
        let user_id = UserId::from("test_user");

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_update_activity_rpe()
            .times(1)
            .returning(|_| Ok(()));

        activity_service
            .expect_update_activity_feedback()
            .with(function(move |req: &UpdateActivityFeedbackRequest| {
                req.user() == &user_id
                    && req.activity() == &ActivityId::from(&activity_id_clone)
                    && req.feedback() == &Some(ActivityFeedback::from("Hard session"))
            }))
            .times(1)
            .returning(|_| Ok(()));

        let state = create_test_state(activity_service);

        let user = AuthenticatedUser::new(UserId::from("test_user"));
        let path = Path(activity_id);
        let query = Query(PatchActivityQuery {
            name: None,
            rpe: Some(9),
            workout_type: None,
            bonk_status: None,
            nutrition_details: None,
        });
        let body = Some(Json(PatchActivityBody {
            feedback: Some("Hard session".to_string()),
        }));

        let result = patch_activity(Extension(user), State(state), path, query, body).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::OK);
    }
}
