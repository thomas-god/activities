use axum::{extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use serde_json::json;

use crate::{
    domain::{
        models::{Activity, Sport},
        ports::ActivityService,
    },
    inbound::http::AppState,
};

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<ResponseBodyItem>);

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBodyItem {
    id: String,
    sport: Option<String>,
    duration: Option<usize>,
    calories: Option<usize>,
}

impl From<&Activity> for ResponseBodyItem {
    fn from(activity: &Activity) -> Self {
        Self {
            id: activity.id().to_string(),
            sport: activity.sport().map(|s| match s {
                Sport::Running => "Running".to_string(),
                Sport::Cycling => "Cycling".to_string(),
                Sport::Other => "Other".to_string(),
            }),
            duration: *activity.duration(),
            calories: *activity.calories(),
        }
    }
}

pub async fn list_activities<AS: ActivityService>(
    State(state): State<AppState<AS>>,
) -> Result<impl IntoResponse, StatusCode> {
    let Ok(res) = state.activity_service.list_activities().await else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let body = ResponseBody(res.iter().map(ResponseBodyItem::from).collect());

    Ok(json!(body).to_string())
}
