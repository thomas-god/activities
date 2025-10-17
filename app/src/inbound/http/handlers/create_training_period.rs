use axum::{Extension, Json, extract::State, http::StatusCode};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::{
    domain::{
        models::{UserId, training::TrainingMetricFilters},
        ports::{
            CreateTrainingMetricError, CreateTrainingMetricRequest, CreateTrainingPeriodError,
            CreateTrainingPeriodRequest, DateRange, IActivityService, ITrainingService,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
            handlers::types::{
                APITrainingMetricAggregate, APITrainingMetricFilters, APITrainingMetricGranularity,
                APITrainingMetricSource, APITrainingPeriodSports,
            },
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct CreateTrainingPeriodBody {
    start: NaiveDate,
    end: Option<NaiveDate>,
    name: String,
    sports: APITrainingPeriodSports,
    note: Option<String>,
}

fn build_request(body: CreateTrainingPeriodBody, user: &UserId) -> CreateTrainingPeriodRequest {
    CreateTrainingPeriodRequest::new(
        user.clone(),
        body.start,
        body.end,
        body.name,
        body.sports.into(),
        body.note,
    )
}

impl From<CreateTrainingPeriodError> for StatusCode {
    fn from(_value: CreateTrainingPeriodError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

pub async fn create_training_period<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Json(payload): Json<CreateTrainingPeriodBody>,
) -> Result<StatusCode, StatusCode> {
    let req = build_request(payload, user.user());

    state
        .training_metrics_service
        .create_training_period(req)
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
            serde_json::from_str::<CreateTrainingPeriodBody>(
                r#"{
            "start": "2025-10-12",
            "end": "2025-12-12",
            "name": "test training period",
            "sports": []
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingPeriodBody>(
                r#"{
            "start": "2025-10-12",
            "end": "2025-12-12",
            "name": "test training period",
            "sports": [{"Sport": "Running"}, {"SportCategory": "Cycling"}],
            "note": "a potentially long string"
        }"#,
            )
            .is_ok()
        );
    }
}
