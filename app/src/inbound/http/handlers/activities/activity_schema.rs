use std::{collections::HashMap, ops::Mul};

use chrono::{DateTime, FixedOffset};
use serde::Serialize;

use crate::domain::models::activity::{
    Activity, ActivityNutrition, ActivityTimeseries, ActivityWithTimeseries, Lap, Timeseries,
    TimeseriesMetric, TimeseriesValue, ToUnit, Unit,
};

// =============================================================================
// Nutrition
// =============================================================================

#[derive(Debug, Clone, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PublicActivityTimeseries {
    pub time: Vec<usize>,
    pub active_time: Vec<Option<usize>>,
    pub metrics: HashMap<String, PublicTimeseries>,
    pub laps: Vec<PublicLap>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PublicTimeseries {
    pub unit: String,
    pub values: Vec<Option<PublicTimeseriesValue>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PublicLap {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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
        let (unit, values) = match metric.metric() {
            TimeseriesMetric::Speed => (
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
            TimeseriesMetric::Distance => (
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
#[derive(Debug, Clone, Serialize, PartialEq)]
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
    pub statistics: HashMap<String, f64>,
}

impl From<&Activity> for PublicActivity {
    fn from(activity: &Activity) -> Self {
        Self {
            id: activity.id().to_string(),
            sport: activity.sport().to_string(),
            sport_category: activity.sport().category().map(|cat| cat.to_string()),
            name: activity.name().map(|name| name.to_string()),
            start_time: *activity.start_time().date(),
            rpe: activity.rpe().as_ref().map(|r| r.value()),
            workout_type: activity.workout_type().as_ref().map(|wt| wt.to_string()),
            feedback: activity.feedback().as_ref().map(|f| f.to_string()),
            nutrition: activity.nutrition().as_ref().map(PublicNutrition::from),
            statistics: activity.statistics().items(),
        }
    }
}

// =============================================================================
// Public representation of an Activity with Timeseries
// =============================================================================

/// Extension of `PublicActivity` that also includes raw timeseries data.
/// Serialises as a flat JSON object (all `PublicActivity` fields at the top level
/// plus a `timeseries` key), so it can be used anywhere `PublicActivity` is
/// accepted on the client side.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PublicActivityWithTimeseries {
    #[serde(flatten)]
    pub activity: PublicActivity,
    pub timeseries: PublicActivityTimeseries,
}

impl From<&ActivityWithTimeseries> for PublicActivityWithTimeseries {
    fn from(activity: &ActivityWithTimeseries) -> Self {
        Self {
            activity: PublicActivity::from(activity.activity()),
            timeseries: activity.timeseries().into(),
        }
    }
}
