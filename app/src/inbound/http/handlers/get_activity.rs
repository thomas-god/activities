use std::{collections::HashMap, ops::Mul};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset};
use serde::Serialize;

use crate::{
    domain::{
        models::activity::{
            Activity, ActivityId, ActivityStatistic, ActivityTimeseries, Sport, Timeseries,
            TimeseriesMetric, TimeseriesValue, ToUnit, Unit,
        },
        ports::{IActivityService, ITrainingMetricService},
    },
    inbound::{http::AppState, parser::ParseFile},
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResponseBody {
    id: String,
    sport: String,
    name: Option<String>,
    duration: Option<f64>,
    start_time: DateTime<FixedOffset>,
    timeseries: ActivityTimeseriesBody,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ActivityTimeseriesBody {
    time: Vec<usize>,
    metrics: HashMap<String, TimeseriesBody>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TimeseriesBody {
    unit: String,
    values: Vec<Option<TimeseriesValueBody>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TimeseriesValueBody {
    Int(usize),
    Float(f64),
}

impl Mul<f64> for TimeseriesValueBody {
    type Output = TimeseriesValueBody;

    fn mul(self, rhs: f64) -> Self::Output {
        match self {
            Self::Int(val) => Self::Float(val as f64 * rhs),
            Self::Float(val) => Self::Float(val * rhs),
        }
    }
}

impl From<&TimeseriesValue> for TimeseriesValueBody {
    fn from(value: &TimeseriesValue) -> Self {
        match value {
            TimeseriesValue::Int(val) => Self::Int(*val),
            TimeseriesValue::Float(val) => Self::Float(*val),
        }
    }
}

impl From<&Activity> for ResponseBody {
    fn from(activity: &Activity) -> Self {
        Self {
            id: activity.id().to_string(),
            name: activity.name().map(|name| name.to_string()),
            sport: match *activity.sport() {
                Sport::Running => "Running".to_string(),
                Sport::Cycling => "Cycling".to_string(),
                Sport::Other => "Other".to_string(),
            },
            start_time: *activity.start_time().date(),
            duration: activity
                .statistics()
                .get(&ActivityStatistic::Duration)
                .cloned(),
            timeseries: activity.timeseries().into(),
        }
    }
}

impl From<&ActivityTimeseries> for ActivityTimeseriesBody {
    fn from(value: &ActivityTimeseries) -> Self {
        Self {
            time: value.time().values().into(),
            metrics: extract_and_convert_metrics(value.metrics()),
        }
    }
}

fn extract_and_convert_metrics(metrics: &[Timeseries]) -> HashMap<String, TimeseriesBody> {
    HashMap::from_iter(metrics.iter().map(|metric| {
        let (unit, values) = match metric.metric() {
            TimeseriesMetric::Speed => (
                Unit::KilometerPerHour,
                metric
                    .values()
                    .iter()
                    .map(|val| {
                        val.as_ref()
                            .map(TimeseriesValueBody::from)
                            .map(|val| val * 3.6)
                    })
                    .collect(),
            ),
            TimeseriesMetric::Distance => (
                Unit::Kilometer,
                metric
                    .values()
                    .iter()
                    .map(|val| {
                        val.as_ref()
                            .map(TimeseriesValueBody::from)
                            .map(|val| val * 0.001)
                    })
                    .collect(),
            ),
            _ => (
                metric.metric().unit(),
                metric
                    .values()
                    .iter()
                    .map(|val| val.as_ref().map(TimeseriesValueBody::from))
                    .collect(),
            ),
        };
        return (
            metric.metric().to_string(),
            TimeseriesBody {
                unit: unit.to_string(),
                values,
            },
        );
    }))
}

pub async fn get_activity<AS: IActivityService, FP: ParseFile, TMS: ITrainingMetricService>(
    State(state): State<AppState<AS, FP, TMS>>,
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
                UserId,
                activity::{
                    ActivityStartTime, ActivityStatistics, ActivityTimeseries, Timeseries,
                    TimeseriesMetric, TimeseriesTime, TimeseriesValue,
                },
            },
            ports::GetActivityError,
            services::test_utils::{MockActivityService, MockTrainingMetricsService},
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
                UserId::default(),
                None,
                ActivityStartTime::new(
                    "2025-09-03T00:00:00Z"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                ),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::from([(ActivityStatistic::Duration, 1200.)])),
                ActivityTimeseries::new(
                    TimeseriesTime::new(vec![0, 1, 2]),
                    vec![Timeseries::new(
                        TimeseriesMetric::Power,
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
        let metrics = MockTrainingMetricsService::default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(state, path).await;
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(
            response.0,
            ResponseBody {
                duration: Some(1200.),
                id: target_id,
                name: None,
                sport: "Cycling".to_string(),
                start_time: "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                timeseries: ActivityTimeseriesBody {
                    time: vec![0, 1, 2],
                    metrics: HashMap::from([(
                        "Power".to_string(),
                        TimeseriesBody {
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
        let metrics = MockTrainingMetricsService::default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(state, path).await;
        assert!(response.is_err());
        let response = response.unwrap_err();
        assert_eq!(response, StatusCode::NOT_FOUND);
    }
}
