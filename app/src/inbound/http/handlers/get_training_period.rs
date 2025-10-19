use std::collections::HashMap;

use axum::{
    Extension,
    extract::{Path, Query, State},
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
                TrainingMetricValues, TrainingPeriod, TrainingPeriodId,
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
pub struct ResponseBody {
    id: String,
    start: NaiveDate,
    end: Option<NaiveDate>,
    name: String,
    sports: Vec<String>,
    note: Option<String>,
}

impl From<TrainingPeriod> for ResponseBody {
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

pub async fn get_training_period<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Path(period_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    match state
        .training_metrics_service
        .get_training_period(user.user(), &TrainingPeriodId::from(&period_id))
        .await
    {
        Some(period) => {
            let body = ResponseBody::from(period);
            Ok(json!(body).to_string())
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
