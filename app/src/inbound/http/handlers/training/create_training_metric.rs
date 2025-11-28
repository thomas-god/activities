use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::{
    domain::{
        models::{
            UserId,
            training::{TrainingMetricFilters, TrainingMetricGroupBy, TrainingMetricName},
        },
        ports::{
            CreateTrainingMetricError, CreateTrainingMetricRequest, DateRange, IActivityService,
            IPreferencesService, ITrainingService,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
            handlers::training::types::{
                APITrainingMetricAggregate, APITrainingMetricFilters, APITrainingMetricGranularity,
                APITrainingMetricGroupBy, APITrainingMetricSource, ScopePayload,
            },
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct CreateTrainingMetricBody {
    name: String,
    source: APITrainingMetricSource,
    granularity: APITrainingMetricGranularity,
    aggregate: APITrainingMetricAggregate,
    filters: APITrainingMetricFilters,
    group_by: Option<APITrainingMetricGroupBy>,
    scope: ScopePayload,
    initial_date_range: Option<DateRange>,
}

fn build_request(
    body: CreateTrainingMetricBody,
    user: &UserId,
) -> Result<CreateTrainingMetricRequest, String> {
    if body.name.trim().is_empty() {
        return Err("Metric name cannot be empty".to_string());
    }

    Ok(CreateTrainingMetricRequest::new(
        user.clone(),
        TrainingMetricName::from(body.name),
        body.source.into(),
        body.granularity.into(),
        body.aggregate.into(),
        body.filters.into(),
        body.group_by.map(TrainingMetricGroupBy::from),
        body.scope.into(),
        body.initial_date_range,
    ))
}

impl From<CreateTrainingMetricError> for StatusCode {
    fn from(_value: CreateTrainingMetricError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

pub async fn create_training_metric<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Json(payload): Json<CreateTrainingMetricBody>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let req = build_request(payload, user.user()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
    })?;

    state
        .training_metrics_service
        .create_metric(req)
        .await
        .map(|_| StatusCode::CREATED)
        .map_err(|e| {
            (
                StatusCode::from(e),
                Json(serde_json::json!({ "error": "Failed to create training metric" })),
            )
        })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_payload_format() {
        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "name": "Test Metric",
            "source": { "Statistic": "Calories"},
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
            "source": { "Timeseries": ["Distance", "Average"]},
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
            "source": { "Timeseries": ["Distance", "Average"]},
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
            "source": { "Statistic": "Calories"},
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
            "source": { "Statistic": "Calories"},
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
            "source": { "Statistic": "Calories"},
            "granularity": "Weekly",
            "aggregate": "Min",
            "filters": {},
            "scope": {"type": "global"}
        }"#,
            )
            .is_ok()
        );
    }
}
