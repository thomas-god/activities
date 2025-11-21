use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::training::{TrainingMetricId, TrainingMetricName};
use crate::domain::ports::{
    IActivityService, IPreferencesService, ITrainingService, UpdateTrainingMetricNameError,
    UpdateTrainingMetricNameRequest,
};
use crate::inbound::http::AppState;
use crate::inbound::http::auth::{AuthenticatedUser, IUserService};
use crate::inbound::parser::ParseFile;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
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

    match state
        .training_metrics_service
        .update_training_metric_name(request)
        .await
    {
        Ok(_) => StatusCode::OK.into_response(),
        Err(UpdateTrainingMetricNameError::MetricDoesNotExist(_)) => (
            StatusCode::NOT_FOUND,
            axum::Json(ErrorResponse {
                error: "Training metric does not exist".to_string(),
            }),
        )
            .into_response(),
        Err(UpdateTrainingMetricNameError::UserDoesNotOwnTrainingMetric(_, _)) => (
            StatusCode::FORBIDDEN,
            axum::Json(ErrorResponse {
                error: "You do not have permission to update this training metric".to_string(),
            }),
        )
            .into_response(),
        Err(UpdateTrainingMetricNameError::GetDefinitionError(e)) => {
            eprintln!("Error getting training metric definition: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
                .into_response()
        }
        Err(UpdateTrainingMetricNameError::Unknown(e)) => {
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
