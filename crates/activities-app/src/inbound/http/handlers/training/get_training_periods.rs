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

use crate::{
    domain::{
        models::{
            activity::{ActivityMetricSource, ActivityStatistic, TimeseriesMetric, ToUnit, Unit},
            training::{
                SportFilter, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricValues, TrainingPeriod, TrainingPeriodSports,
            },
        },
        ports::{
            activity::IActivityService, preferences::IPreferencesService,
            training::ITrainingService,
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{AppState, handlers::training::types::SportsResponse},
        parser::ParseFile,
    },
};

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
