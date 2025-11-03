use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::training::TrainingPeriodId;
use crate::domain::ports::{
    IActivityService, ITrainingService, UpdateTrainingPeriodNameError,
    UpdateTrainingPeriodNameRequest, UpdateTrainingPeriodNoteError,
    UpdateTrainingPeriodNoteRequest,
};
use crate::inbound::http::AppState;
use crate::inbound::http::auth::{AuthenticatedUser, IUserService};
use crate::inbound::parser::ParseFile;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
pub struct UpdateTrainingPeriodBody {
    name: Option<String>,
    note: Option<String>,
}

pub async fn update_training_period<
    AS: IActivityService,
    PF: ParseFile,
    TS: ITrainingService,
    US: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TS, US>>,
    Path(period_id): Path<Uuid>,
    axum::Json(body): axum::Json<UpdateTrainingPeriodBody>,
) -> Response {
    let period_id = TrainingPeriodId::from(&period_id.to_string());

    // Update name if provided
    if let Some(name) = body.name {
        let request =
            UpdateTrainingPeriodNameRequest::new(user.user().clone(), period_id.clone(), name);

        match state
            .training_metrics_service
            .update_training_period_name(request)
            .await
        {
            Ok(_) => {}
            Err(UpdateTrainingPeriodNameError::PeriodDoesNotExist(_)) => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(ErrorResponse {
                        error: "Training period does not exist".to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodNameError::UserDoesNotOwnPeriod(_, _)) => {
                return (
                    StatusCode::FORBIDDEN,
                    axum::Json(ErrorResponse {
                        error: "You do not have permission to update this training period"
                            .to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodNameError::Unknown(e)) => {
                eprintln!("Error updating training period name: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(ErrorResponse {
                        error: "Internal server error".to_string(),
                    }),
                )
                    .into_response();
            }
        }
    }

    // Update note if provided
    if body.note.is_some() {
        let request =
            UpdateTrainingPeriodNoteRequest::new(user.user().clone(), period_id.clone(), body.note);

        match state
            .training_metrics_service
            .update_training_period_note(request)
            .await
        {
            Ok(_) => {}
            Err(UpdateTrainingPeriodNoteError::PeriodDoesNotExist(_)) => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(ErrorResponse {
                        error: "Training period does not exist".to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodNoteError::UserDoesNotOwnPeriod(_, _)) => {
                return (
                    StatusCode::FORBIDDEN,
                    axum::Json(ErrorResponse {
                        error: "You do not have permission to update this training period"
                            .to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodNoteError::Unknown(e)) => {
                eprintln!("Error updating training period note: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(ErrorResponse {
                        error: "Internal server error".to_string(),
                    }),
                )
                    .into_response();
            }
        }
    }

    StatusCode::OK.into_response()
}
