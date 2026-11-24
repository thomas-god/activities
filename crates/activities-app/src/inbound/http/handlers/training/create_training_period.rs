use crate::inbound::http::error_status::ApiErrorStatus;
use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::domain::activity::ports::IActivityService;
use crate::domain::preferences::ports::IPreferencesService;
use crate::domain::shared::UserId;
use crate::domain::training::models::TrainingMetricActivityFilters;
use crate::domain::training::ports::CreateTrainingPeriodError;
use crate::domain::training::ports::CreateTrainingPeriodRequest;
use crate::domain::training::ports::ITrainingService;
use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::http::AppState;
use crate::inbound::http::handlers::training::types::APIActivityMetricSource;
use crate::inbound::http::handlers::training::types::APITrainingMetricAggregate;
use crate::inbound::http::handlers::training::types::APITrainingMetricFilters;
use crate::inbound::http::handlers::training::types::APITrainingMetricGranularity;
use crate::inbound::http::handlers::training::types::APITrainingPeriodSports;
use crate::inbound::parser::ParseFile;

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

impl ApiErrorStatus for CreateTrainingPeriodError {
    fn status_code(self) -> StatusCode {
        StatusCode::UNPROCESSABLE_ENTITY
    }
}

#[tracing::instrument(skip_all, err)]
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
            Err(err.status_code())
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
