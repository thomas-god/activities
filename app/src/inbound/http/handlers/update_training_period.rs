use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::training::TrainingPeriodId;
use crate::domain::ports::{
    IActivityService, ITrainingService, UpdateTrainingPeriodNameError,
    UpdateTrainingPeriodNameRequest,
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
    name: String,
}

pub async fn update_training_period_name<
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
    let request = UpdateTrainingPeriodNameRequest::new(
        user.user().clone(),
        TrainingPeriodId::from(&period_id.to_string()),
        body.name,
    );

    match state
        .training_metrics_service
        .update_training_period_name(request)
        .await
    {
        Ok(_) => StatusCode::OK.into_response(),
        Err(UpdateTrainingPeriodNameError::PeriodDoesNotExist(_)) => (
            StatusCode::NOT_FOUND,
            axum::Json(ErrorResponse {
                error: "Training period does not exist".to_string(),
            }),
        )
            .into_response(),
        Err(UpdateTrainingPeriodNameError::UserDoesNotOwnPeriod(_, _)) => (
            StatusCode::FORBIDDEN,
            axum::Json(ErrorResponse {
                error: "You do not have permission to update this training period".to_string(),
            }),
        )
            .into_response(),
        Err(UpdateTrainingPeriodNameError::Unknown(e)) => {
            eprintln!("Error updating training period name: {:?}", e);
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
