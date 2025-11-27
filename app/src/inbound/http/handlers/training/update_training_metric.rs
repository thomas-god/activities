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
    IActivityService, IPreferencesService, ITrainingService, UpdateTrainingMetricNameError,
    UpdateTrainingMetricNameRequest, UpdateTrainingMetricScopeError,
    UpdateTrainingMetricScopeRequest,
};
use crate::inbound::http::AppState;
use crate::inbound::http::auth::{AuthenticatedUser, IUserService};
use crate::inbound::parser::ParseFile;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTrainingMetricBody {
    name: Option<String>,
    scope: Option<ScopePayload>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ScopePayload {
    Global,
    #[serde(rename_all = "camelCase")]
    TrainingPeriod {
        training_period_id: String,
    },
}

impl From<ScopePayload> for TrainingMetricScope {
    fn from(payload: ScopePayload) -> Self {
        match payload {
            ScopePayload::Global => TrainingMetricScope::Global,
            ScopePayload::TrainingPeriod { training_period_id } => {
                TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from(&training_period_id))
            }
        }
    }
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

    // Validate that at least one field is provided
    if body.name.is_none() && body.scope.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(ErrorResponse {
                error: "At least one field (name or scope) must be provided".to_string(),
            }),
        )
            .into_response();
    }

    // Update name if provided
    if let Some(name) = body.name {
        let name = TrainingMetricName::from(name);
        let request =
            UpdateTrainingMetricNameRequest::new(user.user().clone(), metric_id.clone(), name);

        if let Err(e) = state
            .training_metrics_service
            .update_training_metric_name(request)
            .await
        {
            return handle_update_name_error(e);
        }
    }

    // Update scope if provided
    if let Some(scope_payload) = body.scope {
        let scope = TrainingMetricScope::from(scope_payload);
        let request =
            UpdateTrainingMetricScopeRequest::new(user.user().clone(), metric_id.clone(), scope);

        if let Err(e) = state
            .training_metrics_service
            .update_metric_scope(request)
            .await
        {
            return handle_update_scope_error(e);
        }
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
        UpdateTrainingMetricNameError::UserDoesNotOwnTrainingMetric(_, _) => (
            StatusCode::FORBIDDEN,
            axum::Json(ErrorResponse {
                error: "You do not have permission to update this training metric".to_string(),
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

fn handle_update_scope_error(error: UpdateTrainingMetricScopeError) -> Response {
    match error {
        UpdateTrainingMetricScopeError::MetricDoesNotExist(_) => (
            StatusCode::NOT_FOUND,
            axum::Json(ErrorResponse {
                error: "Training metric does not exist".to_string(),
            }),
        )
            .into_response(),
        UpdateTrainingMetricScopeError::UserDoesNotOwnTrainingMetric(_, _) => (
            StatusCode::FORBIDDEN,
            axum::Json(ErrorResponse {
                error: "You do not have permission to update this training metric".to_string(),
            }),
        )
            .into_response(),
        UpdateTrainingMetricScopeError::TrainingPeriodDoesNotExist(_) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ErrorResponse {
                error: "Training period does not exist".to_string(),
            }),
        )
            .into_response(),
        UpdateTrainingMetricScopeError::GetDefinitionError(e) => {
            eprintln!("Error getting training metric definition: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
                .into_response()
        }
        UpdateTrainingMetricScopeError::Unknown(e) => {
            eprintln!("Error updating training metric scope: {:?}", e);
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
        assert_eq!(body.name, Some("New Metric Name".to_string()));
        assert!(body.scope.is_none());
    }

    #[test]
    fn test_deserialize_update_scope_to_global() {
        let json = r#"{"scope": {"type": "global"}}"#;
        let body: UpdateTrainingMetricBody = serde_json::from_str(json).unwrap();
        assert!(body.name.is_none());
        assert!(body.scope.is_some());

        if let Some(ScopePayload::Global) = body.scope {
            // Success
        } else {
            panic!("Expected Global scope");
        }
    }

    #[test]
    fn test_deserialize_update_scope_to_training_period() {
        let json = r#"{"scope": {"type": "trainingPeriod", "trainingPeriodId": "123e4567-e89b-12d3-a456-426614174000"}}"#;
        let body: UpdateTrainingMetricBody = serde_json::from_str(json).unwrap();
        assert!(body.name.is_none());
        assert!(body.scope.is_some());

        if let Some(ScopePayload::TrainingPeriod { training_period_id }) = body.scope {
            assert_eq!(training_period_id, "123e4567-e89b-12d3-a456-426614174000");
        } else {
            panic!("Expected TrainingPeriod scope");
        }
    }

    #[test]
    fn test_deserialize_update_both_name_and_scope() {
        let json = r#"{"name": "Updated Name", "scope": {"type": "global"}}"#;
        let body: UpdateTrainingMetricBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.name, Some("Updated Name".to_string()));
        assert!(body.scope.is_some());

        if let Some(ScopePayload::Global) = body.scope {
            // Success
        } else {
            panic!("Expected Global scope");
        }
    }

    #[test]
    fn test_deserialize_empty_body() {
        let json = r#"{}"#;
        let body: UpdateTrainingMetricBody = serde_json::from_str(json).unwrap();
        assert!(body.name.is_none());
        assert!(body.scope.is_none());
    }

    #[test]
    fn test_scope_payload_conversion_to_global() {
        let payload = ScopePayload::Global;
        let scope: TrainingMetricScope = payload.into();
        assert!(matches!(scope, TrainingMetricScope::Global));
    }

    #[test]
    fn test_scope_payload_conversion_to_training_period() {
        let period_id = "123e4567-e89b-12d3-a456-426614174000".to_string();
        let payload = ScopePayload::TrainingPeriod {
            training_period_id: period_id.clone(),
        };
        let scope: TrainingMetricScope = payload.into();

        match scope {
            TrainingMetricScope::TrainingPeriod(id) => {
                assert_eq!(id.to_string(), period_id);
            }
            _ => panic!("Expected TrainingPeriod scope"),
        }
    }
}
