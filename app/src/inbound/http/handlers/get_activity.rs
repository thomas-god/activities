use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset};
use serde::Serialize;

use crate::{
    domain::{
        models::{Activity, ActivityId, Sport, Timeseries, TimeseriesValue},
        ports::IActivityService,
    },
    inbound::{http::AppState, parser::ParseFile},
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResponseBody {
    id: String,
    sport: String,
    duration: usize,
    start_time: DateTime<FixedOffset>,
    timeseries: TimeseriesBody,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TimeseriesValueBody {
    Int(usize),
    Float(f64),
}

impl From<&TimeseriesValue> for TimeseriesValueBody {
    fn from(value: &TimeseriesValue) -> Self {
        match value {
            TimeseriesValue::Int(val) => Self::Int(*val),
            TimeseriesValue::Float(val) => Self::Float(*val),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TimeseriesMetricsBody {
    unit: String,
    values: Vec<Option<TimeseriesValueBody>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TimeseriesBody {
    time: Vec<usize>,
    metrics: HashMap<String, TimeseriesMetricsBody>,
}

impl From<&Activity> for ResponseBody {
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
            timeseries: activity.timeseries().into(),
        }
    }
}

impl From<&Timeseries> for TimeseriesBody {
    fn from(value: &Timeseries) -> Self {
        Self {
            time: (**value.time()).clone(),
            metrics: HashMap::from_iter(value.metrics().iter().map(|metric| {
                (
                    metric.metric().to_string(),
                    TimeseriesMetricsBody {
                        unit: metric.metric().unit(),
                        values: metric
                            .values()
                            .iter()
                            .map(|val| val.as_ref().map(TimeseriesValueBody::from))
                            .collect(),
                    },
                )
            })),
        }
    }
}

pub async fn get_activity<AS: IActivityService, FP: ParseFile>(
    State(state): State<AppState<AS, FP>>,
    Path(activity_id): Path<String>,
) -> Result<Json<ResponseBody>, StatusCode> {
    let Ok(res) = state
        .activity_service
        .get_activity(&ActivityId::from(&activity_id))
        .await
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let body = ResponseBody::from(&res);

    Ok(Json(body))
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        vec,
    };

    use axum::extract::Path;

    use crate::{
        domain::{
            models::{
                ActivityDuration, ActivityStartTime, Metric, Timeseries, TimeseriesMetric,
                TimeseriesTime, TimeseriesValue,
            },
            ports::GetActivityError,
            services::test_utils::MockActivityService,
        },
        inbound::parser::test_utils::MockFileParser,
    };

    use super::*;

    #[tokio::test]
    async fn test_get_activity_exists() {
        let target_id = "target_id".to_string();
        let service = MockActivityService {
            get_activity_result: Arc::new(Mutex::new(Ok(Activity::new(
                ActivityId::from(&target_id),
                ActivityStartTime::new(
                    "2025-09-03T00:00:00Z"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                ),
                ActivityDuration::new(1200),
                Sport::Cycling,
                Timeseries::new(
                    TimeseriesTime::new(vec![0, 1, 2]),
                    vec![TimeseriesMetric::new(
                        Metric::Power,
                        vec![
                            Some(TimeseriesValue::Int(120)),
                            None,
                            Some(TimeseriesValue::Int(130)),
                        ],
                    )],
                ),
            )))),
            ..Default::default()
        };

        let file_parser = MockFileParser::default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            file_parser: Arc::new(file_parser),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(state, path).await;
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(
            response.0,
            ResponseBody {
                duration: 1200,
                id: target_id,
                sport: "Cycling".to_string(),
                start_time: "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                timeseries: TimeseriesBody {
                    time: vec![0, 1, 2],
                    metrics: HashMap::from([(
                        "Power".to_string(),
                        TimeseriesMetricsBody {
                            unit: "W".to_string(),
                            values: vec![
                                Some(TimeseriesValueBody::Int(120)),
                                None,
                                Some(TimeseriesValueBody::Int(130))
                            ]
                        }
                    )])
                }
            }
        );
    }

    #[tokio::test]
    async fn test_get_activity_does_not_exist() {
        let target_id = "target_id".to_string();
        let service = MockActivityService {
            get_activity_result: Arc::new(Mutex::new(Err(GetActivityError::ActivityDoesNotExist(
                ActivityId::from(&target_id),
            )))),
            ..Default::default()
        };

        let file_parser = MockFileParser::default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            file_parser: Arc::new(file_parser),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(state, path).await;
        assert!(response.is_err());
        let response = response.unwrap_err();
        assert_eq!(response, StatusCode::NOT_FOUND);
    }
}
