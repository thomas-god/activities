use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::{
    domain::{
        models::training::{TrainingMetricId, TrainingMetricsOrdering},
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
pub struct SetTrainingMetricsOrderingBody {
    #[serde(flatten)]
    scope: ScopePayload,
    metric_ids: Vec<String>,
}

pub async fn set_training_metrics_ordering<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Json(payload): Json<SetTrainingMetricsOrderingBody>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let scope = payload.scope.into();

    // Convert string IDs to TrainingMetricId
    let metric_ids: Vec<TrainingMetricId> = payload
        .metric_ids
        .iter()
        .map(|id| TrainingMetricId::from(id.as_str()))
        .collect();

    // Create ordering from IDs
    let ordering = TrainingMetricsOrdering::try_from(metric_ids).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid ordering: duplicate metric IDs found" })),
        )
    })?;

    state
        .training_metrics_service
        .set_training_metrics_ordering(user.user(), &scope, ordering)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to set metrics ordering: {}", e) })),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_global_scope() {
        let json = r#"{
            "type": "global",
            "metric_ids": ["metric-1", "metric-2", "metric-3"]
        }"#;
        let body: SetTrainingMetricsOrderingBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.metric_ids, vec!["metric-1", "metric-2", "metric-3"]);
        if let ScopePayload::Global = body.scope {
            // Success
        } else {
            panic!("Expected Global scope");
        }
    }

    #[test]
    fn test_deserialize_training_period_scope() {
        let json = r#"{
            "type": "trainingPeriod",
            "trainingPeriodId": "123e4567-e89b-12d3-a456-426614174000",
            "metric_ids": ["metric-1", "metric-2"]
        }"#;
        let body: SetTrainingMetricsOrderingBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.metric_ids, vec!["metric-1", "metric-2"]);
        if let ScopePayload::TrainingPeriod { training_period_id } = body.scope {
            assert_eq!(training_period_id, "123e4567-e89b-12d3-a456-426614174000");
        } else {
            panic!("Expected TrainingPeriod scope");
        }
    }

    #[test]
    fn test_deserialize_empty_metric_ids() {
        let json = r#"{
            "type": "global",
            "metric_ids": []
        }"#;
        let body: SetTrainingMetricsOrderingBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.metric_ids, Vec::<String>::new());
        if let ScopePayload::Global = body.scope {
            // Success
        } else {
            panic!("Expected Global scope");
        }
    }
}
