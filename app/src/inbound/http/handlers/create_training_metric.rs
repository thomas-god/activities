use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::{
    domain::{
        models::UserId,
        ports::{
            CreateTrainingMetricError, CreateTrainingMetricRequest, IActivityService,
            ITrainingMetricService,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, ISessionRepository},
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

fn build_request(body: CreateTrainingMetricBody, user: &UserId) -> CreateTrainingMetricRequest {
    CreateTrainingMetricRequest::new(
        user.clone(),
        body.source.into(),
        body.granularity.into(),
        body.aggregate.into(),
    )
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
    SR: ISessionRepository,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, SR>>,
    Json(payload): Json<CreateTrainingMetricBody>,
) -> Result<StatusCode, StatusCode> {
    let req = build_request(payload, user.user());

    state
        .training_metrics_service
        .create_metric(req)
        .await
        .map(|_| StatusCode::CREATED)
        .map_err(StatusCode::from)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_payload_format() {
        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "source": { "Statistic": "Calories"},
            "granularity": "Weekly",
            "aggregate": "Min"
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingMetricBody>(
                r#"{
            "source": { "Timeseries": ["Distance", "Average"]},
            "granularity": "Weekly",
            "aggregate": "Min"
        }"#,
            )
            .is_ok()
        );
    }
}
