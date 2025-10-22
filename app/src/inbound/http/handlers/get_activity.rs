use std::{collections::HashMap, ops::Mul};

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset};
use serde::Serialize;

use crate::{
    domain::{
        models::activity::{
            Activity, ActivityId, ActivityNutrition, ActivityStatistic, ActivityTimeseries,
            ActivityWithTimeseries, Lap, Sport, Timeseries, TimeseriesMetric, TimeseriesValue,
            ToUnit, Unit,
        },
        ports::{IActivityService, ITrainingService},
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResponseBody {
    id: String,
    sport: String,
    sport_category: Option<String>,
    name: Option<String>,
    duration: Option<f64>,
    start_time: DateTime<FixedOffset>,
    rpe: Option<u8>,
    workout_type: Option<String>,
    nutrition: Option<NutritionBody>,
    statistics: HashMap<String, f64>,
    timeseries: ActivityTimeseriesBody,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NutritionBody {
    bonk_status: String,
    details: Option<String>,
}

impl From<&ActivityNutrition> for NutritionBody {
    fn from(nutrition: &ActivityNutrition) -> Self {
        Self {
            bonk_status: nutrition.bonk_status().to_string(),
            details: nutrition.details().map(|d| d.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ActivityTimeseriesBody {
    time: Vec<usize>,
    active_time: Vec<Option<usize>>,
    metrics: HashMap<String, TimeseriesBody>,
    laps: Vec<LapBody>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TimeseriesBody {
    unit: String,
    values: Vec<Option<TimeseriesValueBody>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LapBody {
    start: usize,
    end: usize,
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

impl From<&ActivityWithTimeseries> for ResponseBody {
    fn from(activity: &ActivityWithTimeseries) -> Self {
        Self {
            id: activity.id().to_string(),
            name: activity.name().map(|name| name.to_string()),
            sport: activity.sport().to_string(),
            sport_category: activity.sport().category().map(|cat| cat.to_string()),
            start_time: *activity.start_time().date(),
            duration: activity
                .statistics()
                .get(&ActivityStatistic::Duration)
                .cloned(),
            rpe: activity.rpe().as_ref().map(|r| u8::from(*r)),
            workout_type: activity.workout_type().map(|wt| wt.to_string()),
            nutrition: activity.nutrition().as_ref().map(NutritionBody::from),
            statistics: activity.statistics().items(),
            timeseries: activity.timeseries().into(),
        }
    }
}

impl From<&ActivityTimeseries> for ActivityTimeseriesBody {
    fn from(value: &ActivityTimeseries) -> Self {
        Self {
            time: value.time().values().into(),
            active_time: value
                .active_time()
                .values()
                .iter()
                .map(|value| value.value())
                .collect(),
            metrics: extract_and_convert_metrics(value.metrics()),
            laps: value.laps().iter().map(LapBody::from).collect(),
        }
    }
}

impl From<&Lap> for LapBody {
    fn from(lap: &Lap) -> Self {
        Self {
            start: lap.start(),
            end: lap.end(),
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
        (
            metric.metric().to_string(),
            TimeseriesBody {
                unit: unit.to_string(),
                values,
            },
        )
    }))
}

pub async fn get_activity<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    Extension(_user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Path(activity_id): Path<String>,
) -> Result<Json<ResponseBody>, StatusCode> {
    let Ok(res) = state
        .activity_service
        .get_activity_with_timeseries(&ActivityId::from(&activity_id))
        .await
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let body = ResponseBody::from(&res);

    Ok(Json(body))
}

#[cfg(test)]
mod tests {
    use std::{hash::Hash, sync::Arc, vec};

    use axum::extract::Path;
    use mockall::predicate::eq;

    use crate::{
        domain::{
            models::{
                UserId,
                activity::{
                    ActiveTime, ActivityStartTime, ActivityStatistics, ActivityTimeseries,
                    Timeseries, TimeseriesActiveTime, TimeseriesMetric, TimeseriesTime,
                    TimeseriesValue,
                },
            },
            ports::GetActivityError,
            services::{
                activity::test_utils::MockActivityService,
                training::test_utils::MockTrainingService,
            },
        },
        inbound::{
            http::{CookieConfig, auth::test_utils::MockUserService},
            parser::test_utils::MockFileParser,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_get_activity_exists() {
        let target_id = "target_id".to_string();
        let mut service = MockActivityService::new();
        service
            .expect_get_activity_with_timeseries()
            .returning(|_| {
                Ok(ActivityWithTimeseries::new(
                    Activity::new(
                        ActivityId::from("target_id"),
                        UserId::test_default(),
                        None,
                        ActivityStartTime::new(
                            "2025-09-03T00:00:00Z"
                                .parse::<DateTime<FixedOffset>>()
                                .unwrap(),
                        ),
                        Sport::IndoorCycling,
                        ActivityStatistics::new(HashMap::from([(
                            ActivityStatistic::Duration,
                            1200.,
                        )])),
                        None,
                        None,
                        None,
                    ),
                    ActivityTimeseries::new(
                        TimeseriesTime::new(vec![0, 1, 2]),
                        TimeseriesActiveTime::new(vec![
                            ActiveTime::Running(0),
                            ActiveTime::Running(1),
                            ActiveTime::Running(2),
                        ]),
                        vec![],
                        vec![Timeseries::new(
                            TimeseriesMetric::Power,
                            vec![
                                Some(TimeseriesValue::Int(120)),
                                None,
                                Some(TimeseriesValue::Int(130)),
                            ],
                        )],
                    )
                    .unwrap(),
                ))
            });
        let file_parser = MockFileParser::test_default();
        let metrics = MockTrainingService::test_default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(
            Extension(AuthenticatedUser::new(UserId::test_default())),
            state,
            path,
        )
        .await;
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(
            response.0,
            ResponseBody {
                duration: Some(1200.),
                id: target_id,
                name: None,
                sport: "IndoorCycling".to_string(),
                sport_category: Some("Cycling".to_string()),
                start_time: "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                rpe: None,
                workout_type: None,
                nutrition: None,
                statistics: HashMap::from([("Duration".to_string(), 1200.)]),
                timeseries: ActivityTimeseriesBody {
                    time: vec![0, 1, 2],
                    active_time: vec![Some(0), Some(1), Some(2)],
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
                    )]),
                    laps: vec![]
                }
            }
        );
    }

    #[tokio::test]
    async fn test_get_activity_does_not_exist() {
        let mut service = MockActivityService::new();
        service
            .expect_get_activity_with_timeseries()
            .with(eq(ActivityId::from("target_id")))
            .returning(|_| {
                Err(GetActivityError::ActivityDoesNotExist(ActivityId::from(
                    "target_id",
                )))
            });

        let file_parser = MockFileParser::test_default();
        let metrics = MockTrainingService::test_default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(
            Extension(AuthenticatedUser::new(UserId::test_default())),
            state,
            path,
        )
        .await;
        assert!(response.is_err());
        let response = response.unwrap_err();
        assert_eq!(response, StatusCode::NOT_FOUND);
    }
}
