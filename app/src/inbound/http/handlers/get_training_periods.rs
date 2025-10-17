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
            activity::{ActivityStatistic, TimeseriesMetric, ToUnit, Unit},
            training::{
                ActivityMetricSource, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricValues, TrainingPeriod,
            },
        },
        ports::{IActivityService, ITrainingService},
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
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
    sports: Vec<String>,
    note: Option<String>,
}

impl From<TrainingPeriod> for ResponseBodyItem {
    fn from(value: TrainingPeriod) -> Self {
        Self {
            id: value.id().to_string(),
            start: *value.start(),
            end: *value.end(),
            name: value.name().to_string(),
            sports: value
                .sports()
                .items()
                .map(|sports| sports.iter().map(|sport| sport.to_string()).collect())
                .unwrap_or_default(),
            note: value.note().clone(),
        }
    }
}

pub async fn get_training_periods<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = state
        .training_metrics_service
        .get_training_periods(user.user())
        .await;

    let body = ResponseBody(res.into_iter().map(ResponseBodyItem::from).collect());

    Ok(json!(body).to_string())
}
