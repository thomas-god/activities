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

use crate::domain::activity::models::ActivityMetricSource;
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
use crate::domain::training::models::TrainingPeriodId;
use crate::domain::training::models::TrainingPeriodSports;
use crate::domain::training::ports::ITrainingService;
use crate::http::AppState;
use crate::parser::ParseFile;
use crate::{
    domain::activity::models::DEFAULT_METRICS,
    http::handlers::{
        activities::activity_schema::PublicActivity, training::types::SportsResponse,
    },
};
use activities_auth::infra::http::AuthenticatedUser;

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody {
    id: String,
    start: NaiveDate,
    end: Option<NaiveDate>,
    name: String,
    sports: SportsResponse,
    note: Option<String>,
    activities: Vec<PublicActivity>,
}

#[tracing::instrument(skip_all, err)]
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
        sports: SportsResponse::from(period.sports()),
        note: period.note().clone(),
        activities,
    };

    Ok(json!(body).to_string())
}
