use axum::{extract::State, http::StatusCode, response::IntoResponse};
use chrono::NaiveDateTime;
use serde::Serialize;
use serde_json::json;

use crate::{
    domain::{
        models::{Activity, Sport},
        ports::ActivityService,
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
    start_time: NaiveDateTime,
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
            start_time: **activity.start_time(),
            duration: (*activity.duration()).into(),
        }
    }
}

pub async fn list_activities<AS: ActivityService, FP: ParseFile>(
    State(state): State<AppState<AS, FP>>,
) -> Result<impl IntoResponse, StatusCode> {
    let Ok(res) = state.activity_service.list_activities().await else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let body = ResponseBody(res.iter().map(ResponseBodyItem::from).collect());

    Ok(json!(body).to_string())
}
