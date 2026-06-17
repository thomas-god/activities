use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::UserId;
use crate::domain::models::activity::ActivityMetricV2;
use crate::domain::models::training::{
    TrainingMetricFilters, TrainingMetricId, TrainingMetricName, TrainingMetricScope,
    TrainingMetricWindow, TrainingPeriodId,
};
use crate::domain::ports::training::{UpdateTrainingMetricError, UpdateTrainingMetricRequest};
use crate::domain::ports::{
    activity::IActivityService,
    preferences::IPreferencesService,
    training::{ITrainingService, UpdateTrainingMetricNameError, UpdateTrainingMetricNameRequest},
};
use crate::inbound::http::AppState;
use crate::inbound::http::auth::{AuthenticatedUser, IUserService};
use crate::inbound::http::handlers::training::types::{
    APITimeseriesWindow, APITrainingMetricFilters, APITrainingMetricSummary, ScopePayload,
};
use crate::inbound::parser::ParseFile;

#[derive(Deserialize)]
pub struct UpdateTrainingMetricBody {
    name: String,
    metric: ActivityMetricV2,
    window: Option<APITimeseriesWindow>,
    #[serde(default)]
    filters: Option<APITrainingMetricFilters>,
    #[serde(default)]
    summary: APITrainingMetricSummary,
}

pub async fn update_training_metric<
    AS: IActivityService,
    PF: ParseFile,
    TS: ITrainingService,
    US: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TS, US, PS>>,
    Path(metric_id): Path<Uuid>,
    axum::Json(body): axum::Json<UpdateTrainingMetricBody>,
) -> StatusCode {
    let metric_id = TrainingMetricId::from(&metric_id.to_string());

    let Ok(request) = build_request(user.user().clone(), metric_id, body) else {
        return StatusCode::BAD_REQUEST;
    };

    match state
        .training_metrics_service
        .update_training_metric(request)
        .await
    {
        Err(UpdateTrainingMetricError::MetricDoesNotExist(_)) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
        Ok(_) => StatusCode::CREATED,
    }
}

fn build_request(
    user: UserId,
    metric: TrainingMetricId,
    body: UpdateTrainingMetricBody,
) -> Result<UpdateTrainingMetricRequest, String> {
    let name = TrainingMetricName::from(body.name);
    let filters = body
        .filters
        .map(TrainingMetricFilters::try_from)
        .transpose()
        .map_err(|_| "Invalid fitlers".to_string())?
        .unwrap_or_else(TrainingMetricFilters::empty);

    Ok(UpdateTrainingMetricRequest::new(
        user,
        metric,
        name,
        body.metric,
        body.window.map(TrainingMetricWindow::from),
        filters,
        body.summary.into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_required_fields_only() {
        let json = r#"{"name": "New Metric Name", "metric": "Calories"}"#;
        let body: UpdateTrainingMetricBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.name, "New Metric Name".to_string());
        assert_eq!(body.metric, ActivityMetricV2::Calories);
        assert!(body.window.is_none());
        assert!(body.filters.is_none());
    }

    #[test]
    fn test_deserialize_missing_required_metric_fails() {
        let json = r#"{"name": "New Metric Name"}"#;
        let result: Result<UpdateTrainingMetricBody, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_scope_payload_conversion_to_global() {
        let payload = ScopePayload::Global;
        let scope: TrainingMetricScope = payload.into();
        assert!(matches!(scope, TrainingMetricScope::Global));
    }
}
