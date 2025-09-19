use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    domain::{
        models::training_metrics::TrainingMetricId,
        ports::{
            DeleteTrainingMetricError, DeleteTrainingMetricRequest, IActivityService,
            ITrainingMetricService,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, ISessionService},
        },
        parser::ParseFile,
    },
};

impl From<DeleteTrainingMetricError> for StatusCode {
    fn from(value: DeleteTrainingMetricError) -> Self {
        match value {
            DeleteTrainingMetricError::MetricDoesNotExist(_) => Self::NOT_FOUND,
            DeleteTrainingMetricError::UserDoesNotOwnTrainingMetric(_, _) => Self::FORBIDDEN,
            _ => Self::UNPROCESSABLE_ENTITY,
        }
    }
}

pub async fn delete_training_metric<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    SR: ISessionService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, SR>>,
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
