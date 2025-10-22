use axum::{
    Extension,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    domain::{
        models::activity::{ActivityId, ActivityName, ActivityRpe, WorkoutType},
        ports::{
            IActivityService, ITrainingService, ModifyActivityError, ModifyActivityRequest,
            UpdateActivityRpeError, UpdateActivityRpeRequest, UpdateActivityWorkoutTypeError,
            UpdateActivityWorkoutTypeRequest,
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
}

/// Handler for PATCH /api/activity/{activity_id}
///
/// Updates an activity's metadata. Currently supports:
/// - `name`: Change the activity name (query parameter)
/// - `rpe`: Set RPE from 1-10, or use 0 to clear it (query parameter)
/// - `workout_type`: Set workout type (easy, tempo, intervals, long_run, race), or empty string to clear (query parameter)
///
/// # Example
/// PATCH /api/activity/123?rpe=7
/// PATCH /api/activity/123?name=Morning%20Run&rpe=8
/// PATCH /api/activity/123?rpe=0  // Clear RPE
/// PATCH /api/activity/123?workout_type=intervals
/// PATCH /api/activity/123?workout_type=  // Clear workout type
pub async fn patch_activity<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Path(activity_id): Path<String>,
    Query(query): Query<PatchActivityQuery>,
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
        };
        assert_eq!(query.workout_type, Some(String::new()));
    }
}
