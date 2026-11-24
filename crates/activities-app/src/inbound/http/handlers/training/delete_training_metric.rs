use crate::inbound::http::error_status::ApiErrorStatus;
use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    domain::{
        models::training::TrainingMetricId,
        ports::{
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{DeleteTrainingMetricError, DeleteTrainingMetricRequest, ITrainingService},
        },
    },
    inbound::{auth::AuthenticatedUser, http::AppState, parser::ParseFile},
};

impl ApiErrorStatus for DeleteTrainingMetricError {
    fn status_code(self) -> StatusCode {
        match self {
            DeleteTrainingMetricError::MetricDoesNotExist(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

#[tracing::instrument(skip_all, err)]
pub async fn delete_training_metric<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(metric_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let req =
        DeleteTrainingMetricRequest::new(user.user().clone(), TrainingMetricId::from(&metric_id));
    match state.training_metrics_service.delete_metric(req).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(err) => {
            if matches!(&err, DeleteTrainingMetricError::Unknown(_)) {
                tracing::error!("Error deleting training metric: {}", err.to_string());
            }
            Err(err.status_code())
        }
    }
}
