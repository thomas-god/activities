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
    inbound::{
        http::{AppState, auth::AuthenticatedUser},
        parser::ParseFile,
    },
};

impl From<DeleteTrainingMetricError> for StatusCode {
    fn from(value: DeleteTrainingMetricError) -> Self {
        match value {
            DeleteTrainingMetricError::MetricDoesNotExist(_) => Self::NOT_FOUND,
            _ => Self::UNPROCESSABLE_ENTITY,
        }
    }
}

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
    state
        .training_metrics_service
        .delete_metric(req)
        .await
        .map(|_| StatusCode::OK)
        .map_err(StatusCode::from)
}
