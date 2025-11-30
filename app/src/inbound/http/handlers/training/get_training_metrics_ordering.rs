use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::training::TrainingMetricId,
        ports::{IActivityService, IPreferencesService, ITrainingService},
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
            handlers::training::types::ScopePayload,
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct GetTrainingMetricsOrderingQuery {
    #[serde(flatten)]
    scope: ScopePayload,
}

#[derive(Debug, Serialize)]
pub struct GetTrainingMetricsOrderingResponse {
    metric_ids: Vec<String>,
}

/// # Example
/// GET /api/training/metrics/ordering?type=global
/// GET /api/training/metrics/ordering?type=trainingPeriod&trainingPeriodId=5e410a51-9274-4a1d-bdaa-db69a1c4874b
pub async fn get_training_metrics_ordering<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Query(query): Query<GetTrainingMetricsOrderingQuery>,
) -> Result<Json<GetTrainingMetricsOrderingResponse>, (StatusCode, Json<serde_json::Value>)> {
    let scope = query.scope.into();

    let ordering = state
        .training_metrics_service
        .get_training_metrics_ordering(user.user(), &scope)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to get metrics ordering: {}", e) })),
            )
        })?;

    Ok(Json(GetTrainingMetricsOrderingResponse {
        metric_ids: ordering.ids().iter().map(|id| id.to_string()).collect(),
    }))
}
