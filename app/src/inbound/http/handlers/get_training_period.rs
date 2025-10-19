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
            activity::{Activity, ActivityStatistic, TimeseriesMetric, ToUnit, Unit},
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
    activities: Vec<ActivityItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityItem {
    id: String,
    sport: String,
    sport_category: Option<String>,
    name: Option<String>,
    duration: Option<f64>,
    distance: Option<f64>,
    elevation: Option<f64>,
    start_time: DateTime<FixedOffset>,
}

impl From<&Activity> for ActivityItem {
    fn from(activity: &Activity) -> Self {
        Self {
            id: activity.id().to_string(),
            sport: activity.sport().to_string(),
            sport_category: activity.sport().category().map(|cat| cat.to_string()),
            name: activity.name().map(|name| name.to_string()),
            start_time: *activity.start_time().date(),
            duration: activity
                .statistics()
                .get(&ActivityStatistic::Duration)
                .cloned(),
            distance: activity
                .statistics()
                .get(&ActivityStatistic::Distance)
                .cloned(),
            elevation: activity
                .statistics()
                .get(&ActivityStatistic::Elevation)
                .cloned(),
        }
    }
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
    let Some(period_with_activities) = state
        .training_metrics_service
        .get_training_period_with_activities(user.user(), &TrainingPeriodId::from(&period_id))
        .await
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let period = period_with_activities.period();
    let activities: Vec<ActivityItem> = period_with_activities
        .activities()
        .iter()
        .map(ActivityItem::from)
        .collect();

    let body = ResponseBody {
        id: period.id().to_string(),
        start: *period.start(),
        end: *period.end(),
        name: period.name().to_string(),
        sports: ResponseSports::from(period.sports()),
        note: period.note().clone(),
        activities,
    };

    Ok(json!(body).to_string())
}
