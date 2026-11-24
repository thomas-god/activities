use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use uuid::Uuid;

use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::http::AppState;
use activities_domain::models::training::TrainingPeriodId;
use activities_domain::ports::training::{DeleteTrainingPeriodError, DeleteTrainingPeriodRequest};
use activities_domain::ports::{
    activity::IActivityService, preferences::IPreferencesService, training::ITrainingService,
};
use activities_infra::parser::ParseFile;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[tracing::instrument(skip_all)]
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
        Err(DeleteTrainingPeriodError::Unknown(e)) => {
            tracing::error!("Error deleting training period: {:?}", e);
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
