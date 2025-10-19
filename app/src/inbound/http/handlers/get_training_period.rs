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
                ActivityMetricSource, SportFilter, TrainingMetricDefinition,
                TrainingMetricGranularity, TrainingMetricValues, TrainingPeriod, TrainingPeriodId,
                TrainingPeriodSports,
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
    sports: ResponseSports,
    note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseSports {
    categories: Vec<String>,
    sports: Vec<String>,
}

impl From<&TrainingPeriodSports> for ResponseSports {
    fn from(value: &TrainingPeriodSports) -> Self {
        let Some(items) = value.items() else {
            return Self {
                categories: vec![],
                sports: vec![],
            };
        };

        let mut sports = Vec::new();
        let mut categories = Vec::new();

        for sport in items {
            match sport {
                SportFilter::Sport(sport) => sports.push(sport.to_string()),
                SportFilter::SportCategory(category) => categories.push(category.to_string()),
            }
        }

        Self { categories, sports }
    }
}

impl From<TrainingPeriod> for ResponseBody {
    fn from(value: TrainingPeriod) -> Self {
        Self {
            id: value.id().to_string(),
            start: *value.start(),
            end: *value.end(),
            name: value.name().to_string(),
            sports: ResponseSports::from(value.sports()),
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
