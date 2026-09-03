use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use derive_more::Display;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::activity::{
            ActivityFeedback, ActivityId, ActivityName, ActivityNutrition, ActivityPatch,
            ActivityRpe, BonkStatus, WorkoutType,
        },
        ports::{
            activity::{IActivityService, PatchActivityError, PatchActivityRequest},
            preferences::IPreferencesService,
            training::ITrainingService,
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{AppState, shared::PatchField},
        parser::ParseFile,
    },
};

impl From<PatchActivityError> for StatusCode {
    fn from(value: PatchActivityError) -> Self {
        match value {
            PatchActivityError::ActivityDoesNotExist(_) => Self::NOT_FOUND,
            PatchActivityError::UserDoesNotOwnActivity(_, _) => Self::FORBIDDEN,
            _ => Self::UNPROCESSABLE_ENTITY,
        }
    }
}

/// Nutrition part of a patch request. Unlike the top-level fields, `details` is a
/// plain `Option`: it is either provided (or `null`/absent => no details).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PatchNutrition {
    bonk_status: String,
    details: Option<String>,
}

/// Body of the PATCH /api/activity/{activity_id} request.
///
/// It mirrors the domain [`ActivityPatch`] (double `Option` convention):
/// - a field **absent** from the body leaves the current value untouched,
/// - a field set to **`null`** clears/removes the current value,
/// - a field with a **value** sets it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PatchActivityBody {
    #[serde(default)]
    name: PatchField<String>,
    #[serde(default)]
    rpe: PatchField<u8>,
    #[serde(default)]
    workout_type: PatchField<String>,
    #[serde(default)]
    nutrition: PatchField<PatchNutrition>,
    #[serde(default)]
    feedback: PatchField<String>,
}

impl TryFrom<PatchActivityBody> for ActivityPatch {
    type Error = String;

    fn try_from(value: PatchActivityBody) -> Result<ActivityPatch, Self::Error> {
        let name = match value.name {
            PatchField::Absent => None,
            PatchField::Clear => Some(None),
            PatchField::Set(name) => Some(Some(ActivityName::from(name.as_str()))),
        };

        let rpe = match value.rpe {
            PatchField::Absent => None,
            PatchField::Clear => Some(None),
            PatchField::Set(value) => Some(Some(ActivityRpe::try_from(value)?)),
        };

        let workout_type = match value.workout_type {
            PatchField::Absent => None,
            PatchField::Clear => Some(None),
            PatchField::Set(value) => Some(Some(value.parse::<WorkoutType>()?)),
        };

        let nutrition = match value.nutrition {
            PatchField::Absent => None,
            PatchField::Clear => Some(None),
            PatchField::Set(nutrition) => {
                let bonk_status = nutrition.bonk_status.parse::<BonkStatus>()?;
                Some(Some(ActivityNutrition::new(bonk_status, nutrition.details)))
            }
        };

        let feedback = match value.feedback {
            PatchField::Absent => None,
            PatchField::Clear => Some(None),
            PatchField::Set(feedback) => Some(Some(ActivityFeedback::from(feedback))),
        };

        Ok(ActivityPatch::new(
            name,
            rpe,
            workout_type,
            nutrition,
            feedback,
        ))
    }
}

/// Handler for PATCH /api/activity/{activity_id}
///
/// Updates an activity's mutable optional fields using the same double `Option` semantics
/// as the domain `ActivityPatch`:
/// - omitted field   => left untouched
/// - `null`          => cleared/set to None
/// - a value         => set
///
/// # Examples
/// ```json
/// // Set name and RPE
/// PATCH /api/activity/123
/// Content-Type: application/json
/// {"name": "Morning Run", "rpe": 7}
///
/// // Clear RPE and workout type
/// PATCH /api/activity/123
/// Content-Type: application/json
/// {"rpe": null, "workout_type": null}
///
/// // Set nutrition
/// PATCH /api/activity/123
/// Content-Type: application/json
/// {"nutrition": {"bonk_status": "bonked", "details": "Forgot to eat"}}
///
/// // Clear nutrition and feedback
/// PATCH /api/activity/123
/// Content-Type: application/json
/// {"nutrition": null, "feedback": null}
/// ```
#[tracing::instrument(skip_all, err(Debug))]
pub async fn patch_activity<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(activity_id): Path<String>,
    body: Option<Json<PatchActivityBody>>,
) -> Result<StatusCode, Response> {
    let Some(Json(body)) = body else {
        return Ok(StatusCode::OK);
    };

    let patch = match ActivityPatch::try_from(body) {
        Ok(patch) => patch,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid patch request: {}", err),
            )
                .into_response());
        }
    };

    if patch.is_empty() {
        return Ok(StatusCode::OK);
    }

    let req = PatchActivityRequest::new(ActivityId::from(&activity_id), user.user().clone(), patch);

    match state.activity_service.patch_activity(req).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(err) => {
            if let PatchActivityError::Unknown(_) = &err {
                tracing::error!("Error while patching activity {activity_id}: {err}");
            }
            Err(StatusCode::from(err).into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::anyhow;
    use mockall::predicate::*;
    use serde_json::json;

    use super::*;

    use crate::{
        domain::{
            models::{
                UserId,
                activity::{Activity, ActivityDuration, ActivityStartTime, Sport},
            },
            services::{
                activity::test_utils::MockActivityService,
                preferences::tests_utils::MockPreferencesService,
                training::test_utils::MockTrainingService,
            },
        },
        inbound::parser::test_utils::MockFileParser,
    };

    const USER_ID: &str = "test_user";
    const ACTIVITY_ID: &str = "test_activity_id";

    // =========================================================================
    // Helpers
    // =========================================================================

    fn create_test_state(
        activity_service: MockActivityService,
    ) -> AppState<MockActivityService, MockFileParser, MockTrainingService, MockPreferencesService>
    {
        AppState {
            activity_service: Arc::new(activity_service),
            file_parser: Arc::new(MockFileParser::test_default()),
            training_metrics_service: Arc::new(MockTrainingService::test_default()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        }
    }

    async fn call_patch(
        body: Option<PatchActivityBody>,
        activity_service: MockActivityService,
    ) -> StatusCode {
        let state = create_test_state(activity_service);
        let user = AuthenticatedUser::new(UserId::from(USER_ID));
        let path = Path(ACTIVITY_ID.to_string());

        match patch_activity(Extension(user), State(state), path, body.map(Json)).await {
            Ok(status) => status,
            Err(body) => body.status(),
        }
    }

    fn base_activity() -> Activity {
        Activity::new(
            ActivityId::from(ACTIVITY_ID),
            UserId::from(USER_ID),
            Some(ActivityName::from("Long ride")),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration::default(),
            Sport::Cycling,
            Some(ActivityRpe::Five),
            Some(WorkoutType::Tempo),
            Some(ActivityNutrition::new(
                BonkStatus::None,
                Some("one gel".to_string()),
            )),
            Some(ActivityFeedback::from("legs felt good")),
        )
    }

    /// A comparable snapshot of an activity's mutable fields.
    type ActivitySummary = (
        Option<String>,                       // name
        Option<u8>,                           // rpe
        Option<String>,                       // workout_type
        Option<(BonkStatus, Option<String>)>, // nutrition
        Option<String>,                       // feedback
    );

    fn summary(activity: &Activity) -> ActivitySummary {
        (
            activity.name().map(|name| name.to_string()),
            activity.rpe().as_ref().map(|rpe| rpe.value()),
            activity
                .workout_type()
                .as_ref()
                .map(|workout| workout.to_string()),
            activity.nutrition().as_ref().map(|nutrition| {
                (
                    nutrition.bonk_status(),
                    nutrition.details().map(|details| details.to_string()),
                )
            }),
            activity
                .feedback()
                .as_ref()
                .map(|feedback| feedback.to_string()),
        )
    }

    /// Creates a mock that expects a single `patch_activity` call and checks that
    /// the forwarded patch, applied to [base_activity], yields `expected`.
    fn mock_ok(expected: ActivitySummary) -> MockActivityService {
        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_patch_activity()
            .with(function(move |req: &PatchActivityRequest| {
                let patched = base_activity().apply_patch(req.patch().clone());
                req.user() == &UserId::from(USER_ID)
                    && req.activity() == &ActivityId::from(ACTIVITY_ID)
                    && summary(&patched) == expected
            }))
            .times(1)
            .returning(|_| Ok(()));
        activity_service
    }

    // =========================================================================
    // Unit tests: body (de)serialization & conversion to ActivityPatch
    // =========================================================================

    #[test]
    fn test_body_field_semantics_absent_null_value() {
        // Absent field => untouched
        let body: PatchActivityBody = serde_json::from_value(json!({})).unwrap();
        assert_eq!(body.name, PatchField::Absent);
        assert_eq!(body.rpe, PatchField::Absent);
        assert_eq!(body.workout_type, PatchField::Absent);
        assert_eq!(body.nutrition, PatchField::Absent);
        assert_eq!(body.feedback, PatchField::Absent);

        // Null => clear
        let body: PatchActivityBody = serde_json::from_value(json!({
            "name": null, "rpe": null, "workout_type": null,
            "nutrition": null, "feedback": null
        }))
        .unwrap();
        assert_eq!(body.name, PatchField::Clear);
        assert_eq!(body.rpe, PatchField::Clear);
        assert_eq!(body.workout_type, PatchField::Clear);
        assert_eq!(body.nutrition, PatchField::Clear);
        assert_eq!(body.feedback, PatchField::Clear);

        // Value => set
        let body: PatchActivityBody = serde_json::from_value(json!({
            "name": "Morning Run",
            "rpe": 7,
            "workout_type": "intervals",
            "nutrition": {"bonk_status": "bonked", "details": "Forgot to eat"},
            "feedback": "great session"
        }))
        .unwrap();
        assert_eq!(body.name, PatchField::Set("Morning Run".to_string()));
        assert_eq!(body.rpe, PatchField::Set(7));
        assert_eq!(body.workout_type, PatchField::Set("intervals".to_string()));
        assert_eq!(
            body.nutrition,
            PatchField::Set(PatchNutrition {
                bonk_status: "bonked".to_string(),
                details: Some("Forgot to eat".to_string()),
            })
        );
        assert_eq!(body.feedback, PatchField::Set("great session".to_string()));
    }

    #[test]
    fn test_body_nutrition_details_is_optional() {
        // details absent and null both map to no details
        let body: PatchActivityBody = serde_json::from_value(json!({
            "nutrition": {"bonk_status": "none"}
        }))
        .unwrap();
        assert_eq!(
            body.nutrition,
            PatchField::Set(PatchNutrition {
                bonk_status: "none".to_string(),
                details: None,
            })
        );

        let body: PatchActivityBody = serde_json::from_value(json!({
            "nutrition": {"bonk_status": "none", "details": null}
        }))
        .unwrap();
        assert_eq!(
            body.nutrition,
            PatchField::Set(PatchNutrition {
                bonk_status: "none".to_string(),
                details: None,
            })
        );
    }

    #[test]
    fn test_to_activity_patch_sets_all_fields() {
        let body = PatchActivityBody {
            name: PatchField::Set("Morning Run".to_string()),
            rpe: PatchField::Set(7),
            workout_type: PatchField::Set("intervals".to_string()),
            nutrition: PatchField::Set(PatchNutrition {
                bonk_status: "bonked".to_string(),
                details: Some("forgot to eat".to_string()),
            }),
            feedback: PatchField::Set("great session".to_string()),
        };

        let patch = ActivityPatch::try_from(body).unwrap();
        let patched = base_activity().apply_patch(patch);

        assert_eq!(
            summary(&patched),
            (
                Some("Morning Run".to_string()),
                Some(7),
                Some("intervals".to_string()),
                Some((BonkStatus::Bonked, Some("forgot to eat".to_string()))),
                Some("great session".to_string()),
            )
        );
    }

    #[test]
    fn test_to_activity_patch_clears_fields_with_null() {
        let body = PatchActivityBody {
            name: PatchField::Clear,
            rpe: PatchField::Clear,
            workout_type: PatchField::Clear,
            nutrition: PatchField::Clear,
            feedback: PatchField::Clear,
        };

        let patch = ActivityPatch::try_from(body).unwrap();
        let patched = base_activity().apply_patch(patch);

        assert_eq!(summary(&patched), (None, None, None, None, None));
    }

    #[test]
    fn test_to_activity_patch_leaves_absent_fields_untouched() {
        let body = PatchActivityBody {
            rpe: PatchField::Set(8),
            ..PatchActivityBody::default()
        };

        let patch = ActivityPatch::try_from(body).unwrap();
        let patched = base_activity().apply_patch(patch);

        // Only RPE changes, everything else keeps its current value.
        assert_eq!(
            summary(&patched),
            (
                Some("Long ride".to_string()),
                Some(8),
                Some("tempo".to_string()),
                Some((BonkStatus::None, Some("one gel".to_string()))),
                Some("legs felt good".to_string()),
            )
        );
    }

    #[test]
    fn test_to_activity_patch_rejects_invalid_values() {
        // 0 is not a valid RPE: clearing is expressed with `null`, not a magic value.
        let body = PatchActivityBody {
            rpe: PatchField::Set(0),
            ..PatchActivityBody::default()
        };
        assert!(ActivityPatch::try_from(body).is_err());

        let body = PatchActivityBody {
            rpe: PatchField::Set(11),
            ..PatchActivityBody::default()
        };
        assert!(ActivityPatch::try_from(body).is_err());

        let body = PatchActivityBody {
            workout_type: PatchField::Set("sprint".to_string()),
            ..PatchActivityBody::default()
        };
        assert!(ActivityPatch::try_from(body).is_err());

        let body = PatchActivityBody {
            nutrition: PatchField::Set(PatchNutrition {
                bonk_status: "mild".to_string(),
                details: None,
            }),
            ..PatchActivityBody::default()
        };
        assert!(ActivityPatch::try_from(body).is_err());
    }

    // =========================================================================
    // Integration tests
    // =========================================================================

    #[tokio::test]
    async fn test_patch_activity_sets_fields() {
        let body = PatchActivityBody {
            name: PatchField::Set("Morning Run".to_string()),
            rpe: PatchField::Set(7),
            workout_type: PatchField::Set("intervals".to_string()),
            nutrition: PatchField::Set(PatchNutrition {
                bonk_status: "bonked".to_string(),
                details: Some("forgot to eat".to_string()),
            }),
            feedback: PatchField::Set("great session".to_string()),
        };

        let status = call_patch(
            Some(body),
            mock_ok((
                Some("Morning Run".to_string()),
                Some(7),
                Some("intervals".to_string()),
                Some((BonkStatus::Bonked, Some("forgot to eat".to_string()))),
                Some("great session".to_string()),
            )),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_clears_fields_with_null() {
        let body = PatchActivityBody {
            name: PatchField::Clear,
            rpe: PatchField::Clear,
            workout_type: PatchField::Clear,
            nutrition: PatchField::Clear,
            feedback: PatchField::Clear,
        };

        let status = call_patch(Some(body), mock_ok((None, None, None, None, None))).await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_partial_update() {
        // Clear the name and update the RPE, leave everything else untouched.
        let body = PatchActivityBody {
            name: PatchField::Clear,
            rpe: PatchField::Set(8),
            ..PatchActivityBody::default()
        };

        let status = call_patch(
            Some(body),
            mock_ok((
                None,
                Some(8),
                Some("tempo".to_string()),
                Some((BonkStatus::None, Some("one gel".to_string()))),
                Some("legs felt good".to_string()),
            )),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_no_body_is_noop() {
        let mut activity_service = MockActivityService::new();
        activity_service.expect_patch_activity().times(0);

        let status = call_patch(None, activity_service).await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_empty_body_is_noop() {
        let mut activity_service = MockActivityService::new();
        activity_service.expect_patch_activity().times(0);

        let status = call_patch(Some(PatchActivityBody::default()), activity_service).await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_patch_activity_invalid_rpe_is_bad_request() {
        // rpe = 0 is not a sentinel for clearing anymore: it is an invalid value.
        let body = PatchActivityBody {
            rpe: PatchField::Set(0),
            ..PatchActivityBody::default()
        };

        let mut activity_service = MockActivityService::new();
        activity_service.expect_patch_activity().times(0);

        let status = call_patch(Some(body), activity_service).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_patch_activity_invalid_workout_type_is_bad_request() {
        let body = PatchActivityBody {
            workout_type: PatchField::Set("sprint".to_string()),
            ..PatchActivityBody::default()
        };

        let mut activity_service = MockActivityService::new();
        activity_service.expect_patch_activity().times(0);

        let status = call_patch(Some(body), activity_service).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_patch_activity_invalid_bonk_status_is_bad_request() {
        let body = PatchActivityBody {
            nutrition: PatchField::Set(PatchNutrition {
                bonk_status: "mild".to_string(),
                details: None,
            }),
            ..PatchActivityBody::default()
        };

        let mut activity_service = MockActivityService::new();
        activity_service.expect_patch_activity().times(0);

        let status = call_patch(Some(body), activity_service).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_patch_activity_activity_not_found() {
        let body = PatchActivityBody {
            feedback: PatchField::Set("This won't work".to_string()),
            ..PatchActivityBody::default()
        };

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_patch_activity()
            .times(1)
            .returning(|req| {
                Err(PatchActivityError::ActivityDoesNotExist(
                    req.activity().clone(),
                ))
            });

        let status = call_patch(Some(body), activity_service).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_patch_activity_not_owned_by_user() {
        let body = PatchActivityBody {
            feedback: PatchField::Set("Wrong user feedback".to_string()),
            ..PatchActivityBody::default()
        };

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_patch_activity()
            .times(1)
            .returning(|req| {
                Err(PatchActivityError::UserDoesNotOwnActivity(
                    req.user().clone(),
                    req.activity().clone(),
                ))
            });

        let status = call_patch(Some(body), activity_service).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_patch_activity_unknown_error() {
        let body = PatchActivityBody {
            feedback: PatchField::Set("any".to_string()),
            ..PatchActivityBody::default()
        };

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_patch_activity()
            .times(1)
            .returning(|_| Err(PatchActivityError::Unknown(anyhow!("an error occured"))));

        let status = call_patch(Some(body), activity_service).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}
