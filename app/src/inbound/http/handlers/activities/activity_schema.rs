use std::{collections::HashMap, ops::Mul};

use chrono::{DateTime, FixedOffset};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::domain::models::activity::{
    Activity, ActivityMetric, ActivityMetrics, ActivityNutrition, ActivityTimeseries,
    ActivityWithExtraContext, ActivityWithParsedData, Lap, Timeseries, TimeseriesMetric,
    TimeseriesValue, ToUnit, TrainingContext, Unit,
};

// =============================================================================
// Nutrition
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicNutrition {
    pub bonk_status: String,
    pub details: Option<String>,
}

impl From<&ActivityNutrition> for PublicNutrition {
    fn from(nutrition: &ActivityNutrition) -> Self {
        Self {
            bonk_status: nutrition.bonk_status().to_string(),
            details: nutrition.details().map(|d| d.to_string()),
        }
    }
}

// =============================================================================
// Timeseries
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicActivityTimeseries {
    pub time: Vec<usize>,
    pub active_time: Vec<Option<usize>>,
    pub metrics: HashMap<String, PublicTimeseries>,
    pub laps: Vec<PublicLap>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicTimeseries {
    pub unit: String,
    pub values: Vec<Option<PublicTimeseriesValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicLap {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PublicTimeseriesValue {
    Int(usize),
    Float(f64),
}

impl Mul<f64> for PublicTimeseriesValue {
    type Output = PublicTimeseriesValue;

    fn mul(self, rhs: f64) -> Self::Output {
        match self {
            Self::Int(val) => Self::Float(val as f64 * rhs),
            Self::Float(val) => Self::Float(val * rhs),
        }
    }
}

impl From<&TimeseriesValue> for PublicTimeseriesValue {
    fn from(value: &TimeseriesValue) -> Self {
        match value {
            TimeseriesValue::Int(val) => Self::Int(*val),
            TimeseriesValue::Float(val) => Self::Float(*val),
        }
    }
}

impl From<&ActivityTimeseries> for PublicActivityTimeseries {
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
            laps: value.laps().iter().map(PublicLap::from).collect(),
        }
    }
}

impl From<&Lap> for PublicLap {
    fn from(lap: &Lap) -> Self {
        Self {
            start: lap.start(),
            end: lap.end(),
        }
    }
}

fn extract_and_convert_metrics(metrics: &[Timeseries]) -> HashMap<String, PublicTimeseries> {
    HashMap::from_iter(metrics.iter().map(|metric| {
        let (unit, values) = match metric.metric().unit() {
            Unit::MeterPerSecond => (
                Unit::KilometerPerHour,
                metric
                    .values()
                    .iter()
                    .map(|val| {
                        val.as_ref()
                            .map(PublicTimeseriesValue::from)
                            .map(|val| val * 3.6)
                    })
                    .collect(),
            ),
            Unit::Meter => (
                Unit::Kilometer,
                metric
                    .values()
                    .iter()
                    .map(|val| {
                        val.as_ref()
                            .map(PublicTimeseriesValue::from)
                            .map(|val| val * 0.001)
                    })
                    .collect(),
            ),
            _ => (
                metric.metric().unit(),
                metric
                    .values()
                    .iter()
                    .map(|val| val.as_ref().map(PublicTimeseriesValue::from))
                    .collect(),
            ),
        };
        (
            metric.metric().to_string(),
            PublicTimeseries {
                unit: unit.to_string(),
                values,
            },
        )
    }))
}

// =============================================================================
// Public representation of an Activity (without timeseries)
// =============================================================================

/// Canonical representation of an activity returned by the API.
/// All activity statistics (duration, distance, elevation, etc.) are exposed
/// through the `statistics` map so every endpoint returns the same shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicActivity {
    pub id: String,
    pub sport: String,
    pub sport_category: Option<String>,
    pub name: Option<String>,
    pub start_time: DateTime<FixedOffset>,
    pub rpe: Option<u8>,
    pub workout_type: Option<String>,
    pub feedback: Option<String>,
    pub nutrition: Option<PublicNutrition>,
    pub metrics: HashMap<String, PublicMetricValue>,
}

impl PublicActivity {
    pub fn from(activity: &Activity, metrics: &ActivityMetrics) -> Self {
        Self {
            id: activity.id().to_string(),
            sport: activity.sport().to_string(),
            sport_category: activity.sport().category().map(|cat| cat.to_string()),
            name: activity.name().map(|name| name.to_string()),
            start_time: *activity.start_time().datetime(),
            rpe: activity.rpe().as_ref().map(|r| r.value()),
            workout_type: activity.workout_type().as_ref().map(|wt| wt.to_string()),
            feedback: activity.feedback().as_ref().map(|f| f.to_string()),
            nutrition: activity.nutrition().as_ref().map(PublicNutrition::from),
            metrics: HashMap::from_iter(metrics.iter().filter_map(|(metric, value)| {
                value.as_ref().map(|value| {
                    (
                        metric.to_string(),
                        PublicMetricValue {
                            value: *value,
                            unit: metric.unit().to_string(),
                        },
                    )
                })
            })),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Constructor)]
pub struct PublicMetricValue {
    value: f64,
    unit: String,
}

// =============================================================================
// Public representation of an Activity with Timeseries
// =============================================================================

/// Extension of `PublicActivity` that also includes raw timeseries data.
/// Serialises as a flat JSON object (all `PublicActivity` fields at the top level
/// plus `training_context` and `timeseries` keys), so it can be used anywhere
/// `PublicActivity` is accepted on the client side.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicActivityWithTimeseries {
    #[serde(flatten)]
    pub activity: PublicActivity,
    pub training_context: PublicTrainingContext,
    pub timeseries: PublicActivityTimeseries,
}

impl From<&ActivityWithExtraContext> for PublicActivityWithTimeseries {
    fn from(activity: &ActivityWithExtraContext) -> Self {
        Self {
            activity: PublicActivity::from(activity.activity().activity(), activity.metrics()),
            training_context: PublicTrainingContext::from(activity.training_context()),
            timeseries: activity.activity().timeseries().into(),
        }
    }
}

/// Additional context used to interpret or derive statistics of an activity
/// (e.g. the athlete weight at the time of the activity, used for W/kg).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicTrainingContext {
    pub weight: Option<f32>,
}

impl From<&TrainingContext> for PublicTrainingContext {
    fn from(context: &TrainingContext) -> Self {
        Self {
            weight: *context.weight(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    use chrono::DateTime;

    use crate::domain::models::UserId;
    use crate::domain::models::activity::{
        ActiveTime, ActivityDuration, ActivityId, ActivityStartTime, ActivityStatistic,
        ActivityStatistics, ActivityTimeseries, ActivityWithExtraContext, ActivityWithParsedData,
        Sport, Timeseries, TimeseriesActiveTime, TimeseriesMetric, TimeseriesTime, TimeseriesValue,
    };

    fn activity_id() -> ActivityId {
        ActivityId::from("activity_id")
    }

    fn start_time() -> DateTime<FixedOffset> {
        "2025-09-03T00:00:00Z"
            .parse::<DateTime<FixedOffset>>()
            .unwrap()
    }

    fn activity_with_parsed_data() -> ActivityWithParsedData {
        ActivityWithParsedData::new(
            Activity::new_empty(
                activity_id(),
                UserId::test_default(),
                ActivityStartTime::new(start_time()),
                ActivityDuration::from(1200.),
                Sport::IndoorCycling,
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
            ActivityStatistics::new(HashMap::from([(ActivityStatistic::Duration, 1200.0)])),
        )
    }

    fn metrics() -> ActivityMetrics {
        ActivityMetrics::new(HashMap::from([(ActivityMetric::Duration, Some(1200.0))]))
    }

    #[test]
    fn test_public_activity_with_timeseries_json_roundtrip() {
        let activity = ActivityWithExtraContext::new(
            activity_with_parsed_data(),
            TrainingContext::new(Some(70.0)),
            metrics(),
        );

        let public = PublicActivityWithTimeseries::from(&activity);

        let json = serde_json::to_string(&public).unwrap();
        let parsed: PublicActivityWithTimeseries = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, public);
    }

    #[test]
    fn test_public_activity_with_timeseries_serialises_training_context() {
        let activity = ActivityWithExtraContext::new(
            activity_with_parsed_data(),
            TrainingContext::new(Some(70.0)),
            metrics(),
        );

        let public = PublicActivityWithTimeseries::from(&activity);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&public).unwrap()).unwrap();

        assert_eq!(json["training_context"]["weight"], serde_json::json!(70.0));
    }

    #[test]
    fn test_public_activity_with_timeseries_serialises_null_weight() {
        let activity = ActivityWithExtraContext::new(
            activity_with_parsed_data(),
            TrainingContext::new(None),
            metrics(),
        );

        let public = PublicActivityWithTimeseries::from(&activity);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&public).unwrap()).unwrap();

        assert_eq!(json["training_context"]["weight"], serde_json::json!(null));
    }
}
