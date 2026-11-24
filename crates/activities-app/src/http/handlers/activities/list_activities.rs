use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::domain::activity::models::DEFAULT_METRICS;
use crate::domain::activity::ports::IActivityService;
use crate::domain::activity::ports::ListActivitiesFilters;
use crate::domain::preferences::ports::IPreferencesService;
use crate::domain::shared::DateRange;
use crate::domain::training::ports::ITrainingService;
use crate::http::AppState;
use crate::parser::ParseFile;
use activities_auth::infra::http::AuthenticatedUser;

use super::activity_schema::PublicActivity;

#[derive(Debug, Deserialize)]
pub struct Filters {
    limit: Option<usize>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

impl From<Filters> for ListActivitiesFilters {
    fn from(value: Filters) -> Self {
        let date_range = match (value.start_date, value.end_date) {
            (Some(start), Some(end)) => Some(DateRange::new(start, end)),
            _ => None,
        };
        Self::empty()
            .set_limit(value.limit)
            .set_date_range(date_range)
    }
}

#[tracing::instrument(skip_all, err)]
pub async fn list_activities<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Query(filters): Query<Filters>,
) -> Result<Json<Vec<PublicActivity>>, StatusCode> {
    let activities = match state
        .activity_service
        .list_activities_with_metrics(user.user(), &filters.into(), &DEFAULT_METRICS)
        .await
    {
        Ok(activities) => activities,
        Err(err) => {
            tracing::error!("Error while getting all activities: {}", err.to_string());
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Json(
        activities
            .iter()
            .map(|(activity, metrics)| PublicActivity::from(activity, metrics))
            .collect(),
    ))
}
