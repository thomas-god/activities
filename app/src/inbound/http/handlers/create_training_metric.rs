use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::{
    domain::ports::{
        CreateTrainingMetricError, CreateTrainingMetricRequest, IActivityService,
        ITrainingMetricService,
    },
    inbound::{
        http::{
            AppState,
            handlers::types::{
                APITrainingMetricAggregate, APITrainingMetricGranularity, APITrainingMetricSource,
            },
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct CreateTrainingMetricBody {
    source: APITrainingMetricSource,
    granularity: APITrainingMetricGranularity,
    aggregate: APITrainingMetricAggregate,
}

impl From<CreateTrainingMetricBody> for CreateTrainingMetricRequest {
    fn from(body: CreateTrainingMetricBody) -> Self {
        Self::new(
            body.source.into(),
            body.granularity.into(),
            body.aggregate.into(),
        )
    }
}

impl From<CreateTrainingMetricError> for StatusCode {
    fn from(_value: CreateTrainingMetricError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

pub async fn create_training_metric<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
>(
    State(state): State<AppState<AS, PF, TMS>>,
    Json(payload): Json<CreateTrainingMetricBody>,
) -> Result<StatusCode, StatusCode> {
    let req = payload.into();

    state
        .training_metrics_service
        .create_metric(req)
        .await
        .map(|_| StatusCode::CREATED)
        .map_err(StatusCode::from)
}
