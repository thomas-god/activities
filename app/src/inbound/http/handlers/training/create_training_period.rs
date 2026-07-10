use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::{UserId, training::TrainingMetricFilters},
        ports::{
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{CreateTrainingPeriodError, CreateTrainingPeriodRequest, ITrainingService},
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{
            AppState,
            handlers::training::types::{
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

#[derive(Debug, Serialize)]
pub struct CreateTrainingPeriodResponse {
    id: String,
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
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Json(payload): Json<CreateTrainingPeriodBody>,
) -> Result<(StatusCode, Json<CreateTrainingPeriodResponse>), StatusCode> {
    let req = build_request(payload, user.user());

    match state
        .training_metrics_service
        .create_training_period(req)
        .await
    {
        Ok(id) => Ok((
            StatusCode::CREATED,
            Json(CreateTrainingPeriodResponse { id: id.to_string() }),
        )),
        Err(err) => {
            if matches!(&err, CreateTrainingPeriodError::Unknown(_)) {
                tracing::error!("Error creating training period: {}", err.to_string());
            }
            Err(StatusCode::from(err))
        }
    }
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
