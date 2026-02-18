use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    domain::{
        models::training::{SportFilter, TrainingPeriod, TrainingPeriodSports},
        ports::{IActivityService, IPreferencesService, ITrainingService},
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Clone, Deserialize)]
pub struct QueryParams {
    ref_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<ResponseBodyItem>);

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBodyItem {
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

impl From<TrainingPeriod> for ResponseBodyItem {
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

pub async fn get_active_training_periods<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Query(params): Query<QueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = state
        .training_metrics_service
        .get_active_training_periods(user.user(), &params.ref_date)
        .await;

    let body = ResponseBody(res.into_iter().map(ResponseBodyItem::from).collect());

    Ok(json!(body).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::from_str;

    #[test]
    fn test_query_params_deserialize() {
        let json = r#"{"ref_date": "2025-11-15"}"#;
        let params: QueryParams = from_str(json).unwrap();
        assert_eq!(
            params.ref_date,
            NaiveDate::from_ymd_opt(2025, 11, 15).unwrap()
        );
    }

    #[test]
    fn test_response_body_item_serialize() {
        use crate::domain::models::{
            UserId,
            training::{TrainingPeriodId, TrainingPeriodSports},
        };

        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::from("test_user".to_string()),
            NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
            Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
            "Q4 2025".to_string(),
            TrainingPeriodSports::new(None),
            Some("Test note".to_string()),
        )
        .unwrap();

        let item = ResponseBodyItem::from(period.clone());

        assert_eq!(item.id, period.id().to_string());
        assert_eq!(item.start, *period.start());
        assert_eq!(item.end, *period.end());
        assert_eq!(item.name, period.name());
        assert_eq!(item.note, period.note().clone());
    }
}
