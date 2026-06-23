use axum::Extension;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::domain::models::training::{TrainingMetricId, TrainingMetricName, TrainingPeriodId};
use crate::domain::ports::{
    activity::IActivityService,
    preferences::IPreferencesService,
    training::{CopyTrainingMetricError, CopyTrainingMetricRequest, ITrainingService},
};
use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::http::AppState;
use crate::inbound::parser::ParseFile;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyTrainingMetricBody {
    target_period: String,
    new_name: Option<String>,
}

pub async fn copy_training_metric<
    AS: IActivityService,
    PF: ParseFile,
    TS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TS, PS>>,
    Path(metric_id): Path<String>,
    axum::Json(body): axum::Json<CopyTrainingMetricBody>,
) -> Response {
    let source_metric = TrainingMetricId::from(&metric_id);
    let target_period = TrainingPeriodId::from(&body.target_period);
    let new_name = body.new_name.map(TrainingMetricName::from);
    let req =
        CopyTrainingMetricRequest::new(user.user().clone(), source_metric, target_period, new_name);

    match state
        .training_metrics_service
        .copy_training_metric(req)
        .await
    {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(e) => handle_error(e),
    }
}

fn handle_error(error: CopyTrainingMetricError) -> Response {
    match error {
        CopyTrainingMetricError::MetricDoesNotExist(_) => (
            StatusCode::NOT_FOUND,
            axum::Json(ErrorResponse {
                error: "Training metric does not exist".to_string(),
            }),
        )
            .into_response(),
        CopyTrainingMetricError::PeriodDoesNotExist(_) => (
            StatusCode::NOT_FOUND,
            axum::Json(ErrorResponse {
                error: "Training period does not exist".to_string(),
            }),
        )
            .into_response(),
        CopyTrainingMetricError::SaveMetricError(e) => {
            eprintln!("Error saving copied training metric: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
                .into_response()
        }
        CopyTrainingMetricError::Unknown(e) => {
            eprintln!("Error copying training metric: {:?}", e);
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
    fn test_deserialize_copy_training_metric_body() {
        let json = r#"{"targetPeriod": "some-period-id"}"#;
        let body: CopyTrainingMetricBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.target_period, "some-period-id");
        assert!(body.new_name.is_none());
    }

    #[test]
    fn test_deserialize_copy_training_metric_body_with_new_name() {
        let json = r#"{"targetPeriod": "some-period-id", "newName": "new-name"}"#;
        let body: CopyTrainingMetricBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.target_period, "some-period-id");
        assert_eq!(body.new_name, Some("new-name".into()));
    }
}
