use std::collections::HashMap;

use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, FixedOffset, Local, NaiveDate};
use serde::{Deserialize, Serialize, de};
use serde_json::json;

use crate::domain::activity::models::ActivityMetricSource;
use crate::domain::activity::models::ActivityStatistic;
use crate::domain::activity::models::TimeseriesMetric;
use crate::domain::activity::models::ToUnit;
use crate::domain::activity::models::Unit;
use crate::domain::activity::ports::IActivityService;
use crate::domain::preferences::ports::IPreferencesService;
use crate::domain::training::models::SportFilter;
use crate::domain::training::models::TrainingMetricDefinition;
use crate::domain::training::models::TrainingMetricGranularity;
use crate::domain::training::models::TrainingMetricValues;
use crate::domain::training::models::TrainingPeriod;
use crate::domain::training::models::TrainingPeriodSports;
use crate::domain::training::ports::ITrainingService;
use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::http::AppState;
use crate::inbound::http::handlers::training::types::SportsResponse;
use crate::inbound::parser::ParseFile;

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<ResponseBodyItem>);

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBodyItem {
    id: String,
    start: NaiveDate,
    end: Option<NaiveDate>,
    name: String,
    sports: SportsResponse,
    note: Option<String>,
}

impl From<TrainingPeriod> for ResponseBodyItem {
    fn from(value: TrainingPeriod) -> Self {
        Self {
            id: value.id().to_string(),
            start: *value.start(),
            end: *value.end(),
            name: value.name().to_string(),
            sports: SportsResponse::from(value.sports()),
            note: value.note().clone(),
        }
    }
}

#[tracing::instrument(skip_all, err)]
pub async fn get_training_periods<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = state
        .training_metrics_service
        .get_training_periods(user.user())
        .await;

    let body = ResponseBody(res.into_iter().map(ResponseBodyItem::from).collect());

    Ok(json!(body).to_string())
}
