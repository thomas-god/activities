use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    domain::{
        models::activity::{Activity, ActivityStatistic, Sport},
        ports::{IActivityService, ITrainingMetricService, ListActivitiesFilters},
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct Filters {
    limit: Option<usize>,
}

impl From<Filters> for ListActivitiesFilters {
    fn from(value: Filters) -> Self {
        Self::empty().set_limit(value.limit)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<ResponseBodyItem>);

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBodyItem {
    id: String,
    sport: String,
    name: Option<String>,
    duration: Option<f64>,
    start_time: DateTime<FixedOffset>,
}

impl From<&Activity> for ResponseBodyItem {
    fn from(activity: &Activity) -> Self {
        Self {
            id: activity.id().to_string(),
            sport: match *activity.sport() {
                Sport::Running => "Running".to_string(),
                Sport::Cycling => "Cycling".to_string(),
                Sport::AlpineSKi => "Ski".to_string(),
                Sport::StrengthTraining => "Strenght training".to_string(),
                Sport::Swimming => "Swimming".to_string(),
                Sport::Other => "Other".to_string(),
            },
            name: activity.name().map(|name| name.to_string()),
            start_time: *activity.start_time().date(),
            duration: activity
                .statistics()
                .get(&ActivityStatistic::Duration)
                .cloned(),
        }
    }
}

pub async fn list_activities<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Query(filters): Query<Filters>,
) -> Result<impl IntoResponse, StatusCode> {
    let Ok(res) = state
        .activity_service
        .list_activities(user.user(), &filters.into())
        .await
    else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let body = ResponseBody(res.iter().map(ResponseBodyItem::from).collect());

    Ok(json!(body).to_string())
}
