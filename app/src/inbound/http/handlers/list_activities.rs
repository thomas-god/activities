use axum::{extract::State, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, FixedOffset};
use serde::Serialize;
use serde_json::json;

use crate::{
    domain::{
        models::{
            UserId,
            activity::{Activity, Sport},
        },
        ports::{IActivityService, ITrainingMetricService},
    },
    inbound::{http::AppState, parser::ParseFile},
};

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<ResponseBodyItem>);

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBodyItem {
    id: String,
    sport: String,
    duration: usize,
    start_time: DateTime<FixedOffset>,
}

impl From<&Activity> for ResponseBodyItem {
    fn from(activity: &Activity) -> Self {
        Self {
            id: activity.id().to_string(),
            sport: match *activity.sport() {
                Sport::Running => "Running".to_string(),
                Sport::Cycling => "Cycling".to_string(),
                Sport::Other => "Other".to_string(),
            },
            start_time: *activity.start_time().date(),
            duration: (*activity.duration()).into(),
        }
    }
}

pub async fn list_activities<AS: IActivityService, PF: ParseFile, TMS: ITrainingMetricService>(
    State(state): State<AppState<AS, PF, TMS>>,
) -> Result<impl IntoResponse, StatusCode> {
    let Ok(res) = state
        .activity_service
        .list_activities(&UserId::default())
        .await
    else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let body = ResponseBody(res.iter().map(ResponseBodyItem::from).collect());

    Ok(json!(body).to_string())
}
