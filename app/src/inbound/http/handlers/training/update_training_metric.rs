use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::training::{
    TrainingMetricId, TrainingMetricName, TrainingMetricScope, TrainingPeriodId,
};
use crate::domain::ports::{
    activity::IActivityService,
    preferences::IPreferencesService,
    training::{ITrainingService, UpdateTrainingMetricNameError, UpdateTrainingMetricNameRequest},
};
use crate::inbound::http::AppState;
use crate::inbound::http::auth::{AuthenticatedUser, IUserService};
use crate::inbound::http::handlers::training::types::ScopePayload;
use crate::inbound::parser::ParseFile;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTrainingMetricBody {
    name: String,
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
) -> Response {
    let metric_id = TrainingMetricId::from(&metric_id.to_string());

    let name = TrainingMetricName::from(body.name);
    let request =
        UpdateTrainingMetricNameRequest::new(user.user().clone(), metric_id.clone(), name);

    if let Err(e) = state
        .training_metrics_service
        .update_training_metric_name(request)
        .await
    {
        return handle_update_name_error(e);
    }

    StatusCode::OK.into_response()
}

fn handle_update_name_error(error: UpdateTrainingMetricNameError) -> Response {
    match error {
        UpdateTrainingMetricNameError::MetricDoesNotExist(_) => (
            StatusCode::NOT_FOUND,
            axum::Json(ErrorResponse {
                error: "Training metric does not exist".to_string(),
            }),
        )
            .into_response(),
        UpdateTrainingMetricNameError::GetDefinitionError(e) => {
            eprintln!("Error getting training metric definition: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
                .into_response()
        }
        UpdateTrainingMetricNameError::Unknown(e) => {
            eprintln!("Error updating training metric name: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_update_name_only() {
        let json = r#"{"name": "New Metric Name"}"#;
        let body: UpdateTrainingMetricBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.name, "New Metric Name".to_string());
    }

    #[test]
    fn test_scope_payload_conversion_to_global() {
        let payload = ScopePayload::Global;
        let scope: TrainingMetricScope = payload.into();
        assert!(matches!(scope, TrainingMetricScope::Global));
    }
}
