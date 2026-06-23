use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::models::training::TrainingPeriodId;
use crate::domain::ports::training::{DeleteTrainingPeriodError, DeleteTrainingPeriodRequest};
use crate::domain::ports::{
    activity::IActivityService, preferences::IPreferencesService, training::ITrainingService,
};
use crate::inbound::http::AppState;
use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::parser::ParseFile;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

pub async fn delete_training_period<
    AS: IActivityService,
    PF: ParseFile,
    TS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TS, PS>>,
    Path(period_id): Path<Uuid>,
) -> Response {
    let request = DeleteTrainingPeriodRequest::new(
        user.user().clone(),
        TrainingPeriodId::from(&period_id.to_string()),
    );

    match state
        .training_metrics_service
        .delete_training_period(request)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(DeleteTrainingPeriodError::PeriodDoesNotExist(_)) => (
            StatusCode::NOT_FOUND,
            axum::Json(ErrorResponse {
                error: "Training period does not exist".to_string(),
            }),
        )
            .into_response(),
        Err(DeleteTrainingPeriodError::Unknown(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
            .into_response(),
    }
}
