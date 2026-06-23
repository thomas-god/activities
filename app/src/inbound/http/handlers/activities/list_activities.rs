use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::{
    domain::{
        models::activity::DEFAULT_METRICS,
        ports::{
            DateRange,
            activity::{IActivityService, ListActivitiesFilters},
            preferences::IPreferencesService,
            training::ITrainingService,
        },
    },
    inbound::{auth::AuthenticatedUser, http::AppState, parser::ParseFile},
};

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
    let Ok(activities) = state
        .activity_service
        .list_activities_with_metrics(user.user(), &filters.into(), &DEFAULT_METRICS)
        .await
    else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(Json(
        activities
            .iter()
            .map(|(activity, metrics)| PublicActivity::from(activity, metrics))
            .collect(),
    ))
}
