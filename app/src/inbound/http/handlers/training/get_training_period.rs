use std::collections::HashMap;

use axum::{
    Extension,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize, de};
use serde_json::json;

use crate::{
    domain::models::activity::DEFAULT_METRICS,
    inbound::http::handlers::activities::activity_schema::PublicActivity,
};
use crate::{
    domain::{
        models::{
            activity::{ActivityMetricSource, TimeseriesMetric, ToUnit, Unit},
            training::{
                SportFilter, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricValues, TrainingPeriod, TrainingPeriodId, TrainingPeriodSports,
            },
        },
        ports::{
            activity::IActivityService, preferences::IPreferencesService,
            training::ITrainingService,
        },
    },
    inbound::{auth::AuthenticatedUser, http::AppState, parser::ParseFile},
};

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody {
    id: String,
    start: NaiveDate,
    end: Option<NaiveDate>,
    name: String,
    sports: ResponseSports,
    note: Option<String>,
    activities: Vec<PublicActivity>,
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
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(period_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(period_with_activities) = state
        .training_metrics_service
        .get_training_period_with_activities_with_metrics(
            user.user(),
            &TrainingPeriodId::from(&period_id),
            &DEFAULT_METRICS,
        )
        .await
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let period = period_with_activities.period();
    let activities: Vec<PublicActivity> = period_with_activities
        .activities()
        .iter()
        .map(|(activity, metrics)| PublicActivity::from(activity, metrics))
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
