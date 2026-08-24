use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::{
    domain::{
        models::{
            UserId,
            activity::ActivityMetricV2,
            training::{
                TrainingMetricFilters, TrainingMetricGroupBy, TrainingMetricName,
                TrainingMetricTarget, TrainingMetricWindow,
            },
        },
        ports::{
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{CreateTrainingMetricError, CreateTrainingMetricRequest, ITrainingService},
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{
            AppState,
            handlers::training::types::{
                APITimeseriesWindow, APITrainingMetricAggregate, APITrainingMetricFilters,
                APITrainingMetricGranularity, APITrainingMetricGroupBy, APITrainingMetricScope,
                APITrainingMetricSource, APITrainingMetricSummary, APITrainingMetricTarget,
            },
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct CreateTrainingMetricBody {
    name: String,
    metric: ActivityMetricV2,
    window: Option<APITimeseriesWindow>,
    #[serde(default)]
    filters: Option<APITrainingMetricFilters>,
    #[serde(default)]
    summary: APITrainingMetricSummary,
    #[serde(default)]
    target: Option<APITrainingMetricTarget>,
    scope: APITrainingMetricScope,
}

fn build_request(
    body: CreateTrainingMetricBody,
    user: &UserId,
) -> Result<CreateTrainingMetricRequest, String> {
    if body.name.trim().is_empty() {
        return Err("Metric name cannot be empty".to_string());
    }
    let filters = body
        .filters
        .map(TrainingMetricFilters::try_from)
        .transpose()
        .map_err(|_| "Invalid fitlers".to_string())?
        .unwrap_or_else(TrainingMetricFilters::empty);

    let target = body
        .target
        .map(TrainingMetricTarget::try_from)
        .transpose()
        .map_err(|_| "Invalid target unit".to_string())?;

    Ok(CreateTrainingMetricRequest::new(
        user.clone(),
        TrainingMetricName::from(body.name),
        body.metric,
        body.window.map(TrainingMetricWindow::from),
        filters,
        body.summary.into(),
        body.scope.into(),
        target,
    ))
}

impl From<CreateTrainingMetricError> for StatusCode {
    fn from(_value: CreateTrainingMetricError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

#[tracing::instrument(skip_all, err(Debug))]
pub async fn create_training_metric<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Json(payload): Json<CreateTrainingMetricBody>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let req = build_request(payload, user.user()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
    })?;

    match state.training_metrics_service.create_metric(req).await {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(err) => {
            if matches!(&err, CreateTrainingMetricError::Unknown(_)) {
                tracing::error!("Error creating training metric: {}", err.to_string());
            }
            Err((
                StatusCode::from(err),
                Json(serde_json::json!({ "error": "Failed to create training metric" })),
            ))
        }
    }
}

#[cfg(test)]
mod tests_create_training_metric {

    use super::*;

    use crate::domain::models::activity::Unit;

    #[test]
    fn test_payload_format() {
        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "name": "Test Metric",
            "metric": "Calories",
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": {},
            "scope": {"type": "global"}
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "name": "Test Metric",
            "metric": "MinSpeed",
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": {},
            "scope": {"type": "trainingPeriod", "trainingPeriodId": "123e4567-e89b-12d3-a456-426614174000"}
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "name": "Test Metric",
            "metric": "MinSpeed",
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": { "sports": [{"Sport": "Running"}, {"SportCategory": "Cycling"}] },
            "scope": {"type": "global"}
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "name": "Test Metric",
            "metric": "Calories",
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": {},
            "group_by": "Sport",
            "scope": {"type": "global"}
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "name": "Test Metric",
            "metric": "Calories",
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": {},
            "group_by": "RpeRange",
            "scope": {"type": "global"}
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "name": "My Custom Metric",
            "metric": "Calories",
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": {},
            "scope": {"type": "global"}
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "name": "My Custom Metric",
            "metric": "Calories",
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": {
                "rpes":[1, 5],
                "bonked": "Bonked",
                "workout_types": ["Tempo", "Easy"]
            },
            "scope": {"type": "global"}
        }"#,
            )
            .is_ok()
        );
    }

    #[test]
    fn test_payload_with_target() {
        let body: CreateTrainingMetricBody = serde_json::from_str(
            r#"{
            "name": "Test Metric",
            "metric": "Calories",
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": {},
            "scope": {"type": "global"},
            "target": {"value": 100.0, "unit": "km"}
        }"#,
        )
        .unwrap();

        let req = build_request(body, &UserId::test_default()).unwrap();
        let target = req.target().as_ref().expect("target should be set");
        assert_eq!(target.value(), 100.0);
        assert_eq!(target.unit(), Unit::Kilometer);
    }

    #[test]
    fn test_payload_with_invalid_target_unit_rejected() {
        let body: CreateTrainingMetricBody = serde_json::from_str(
            r#"{
            "name": "Test Metric",
            "metric": "Calories",
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": {},
            "scope": {"type": "global"},
            "target": {"value": 100.0, "unit": "parsec"}
        }"#,
        )
        .unwrap();

        assert!(build_request(body, &UserId::test_default()).is_err());
    }
}
