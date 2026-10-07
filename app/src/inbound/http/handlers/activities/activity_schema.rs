use std::{collections::HashMap, ops::Mul};

use chrono::{DateTime, FixedOffset};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::domain::{
    models::{
        activity::{
            Activity, ActivityDurationCurve, ActivityMetric, ActivityMetrics, ActivityNutrition,
            ActivityTimeseries, ActivityWithParsedData, Lap, Timeseries, TimeseriesMetric,
            TimeseriesValue, ToUnit, Unit,
        },
        training::BestDurationCurve,
    },
    ports::training::{ActivityWithTrainingContext, TrainingContext},
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
    pub duration_curves: Vec<PublicDurationCurve>,
}

impl From<&ActivityWithTrainingContext> for PublicActivityWithTimeseries {
    fn from(activity: &ActivityWithTrainingContext) -> Self {
        Self {
            activity: PublicActivity::from(activity.activity().activity(), activity.metrics()),
            training_context: PublicTrainingContext::from(activity.training_context()),
            timeseries: activity.activity().timeseries().into(),
            duration_curves: activity
                .activity()
                .duration_curves()
                .iter()
                .map(PublicDurationCurve::from)
                .collect(),
        }
    }
}

// =============================================================================
// Duration curves
// =============================================================================

/// Best rolling-average values of an activity for a fixed set of durations
/// (5s, 10s, 30s, 1min, 2min, 5min, 10min, 20min, 30min, 1h, 2h, 5h),
/// e.g. peak power or peak speed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicDurationCurve {
    pub curve_type: String,
    pub unit: String,
    pub values: Vec<Option<PublicTimeseriesValue>>,
}

impl From<&ActivityDurationCurve> for PublicDurationCurve {
    fn from(curve: &ActivityDurationCurve) -> Self {
        let (unit, values) = public_curve_values(curve.unit(), curve.values());
        Self {
            curve_type: curve.curve_type().to_string(),
            unit: unit.to_string(),
            values,
        }
    }
}

impl From<&BestDurationCurve> for PublicDurationCurve {
    fn from(curve: &BestDurationCurve) -> Self {
        let (unit, values) = public_curve_values(curve.unit(), curve.values());
        Self {
            curve_type: curve.curve_type().to_string(),
            unit: unit.to_string(),
            values,
        }
    }
}

/// Convert raw curve values to their public representation: speeds are
/// exposed in km/h instead of m/s, everything else is left unchanged.
fn public_curve_values(
    unit: Unit,
    values: &[Option<f32>; 12],
) -> (Unit, Vec<Option<PublicTimeseriesValue>>) {
    match unit {
        Unit::MeterPerSecond => (
            Unit::KilometerPerHour,
            values
                .iter()
                .map(|value| value.map(|value| PublicTimeseriesValue::Float(value as f64 * 3.6)))
                .collect(),
        ),
        unit => (
            unit,
            values
                .iter()
                .map(|value| value.map(|value| PublicTimeseriesValue::Float(value as f64)))
                .collect(),
        ),
    }
}

// =============================================================================
// Public representation of an Activity (without timeseries)
// =============================================================================

/// Additional training context used to interpret or derive statistics of an
/// activity (e.g. the athlete weight at the time of the activity, used for
/// W/kg, or the best duration curves over the 12 weeks before the activity).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicTrainingContext {
    pub weight: Option<f32>,
    pub best_duration_12w_curves: Vec<PublicDurationCurve>,
}

impl From<&TrainingContext> for PublicTrainingContext {
    fn from(context: &TrainingContext) -> Self {
        Self {
            weight: *context.weight(),
            best_duration_12w_curves: context
                .best_duration_12w_curves()
                .iter()
                .map(PublicDurationCurve::from)
                .collect(),
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
        ActiveTime, ActivityDuration, ActivityDurationCurve, ActivityDurationCurves, ActivityId,
        ActivityStartTime, ActivityStatistic, ActivityStatistics, ActivityTimeseries,
        ActivityWithParsedData, DurationCurveType, Sport, Timeseries, TimeseriesActiveTime,
        TimeseriesMetric, TimeseriesTime, TimeseriesValue,
    };
    use crate::domain::models::training::BestDurationCurve;

    fn activity_id() -> ActivityId {
        ActivityId::from("activity_id")
    }

    fn start_time() -> DateTime<FixedOffset> {
        "2025-09-03T00:00:00Z"
            .parse::<DateTime<FixedOffset>>()
            .unwrap()
    }

    fn activity_with_parsed_data() -> ActivityWithParsedData {
        activity_with_parsed_data_with_curves(ActivityDurationCurves::default())
    }

    fn activity_with_parsed_data_with_curves(
        curves: ActivityDurationCurves,
    ) -> ActivityWithParsedData {
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
            curves,
        )
    }

    fn metrics() -> ActivityMetrics {
        ActivityMetrics::new(HashMap::from([(ActivityMetric::Duration, Some(1200.0))]))
    }

    #[test]
    fn test_public_activity_with_timeseries_json_roundtrip() {
        let activity = ActivityWithTrainingContext::new(
            activity_with_parsed_data(),
            TrainingContext::new(Some(70.0), vec![]),
            metrics(),
        );

        let public = PublicActivityWithTimeseries::from(&activity);

        let json = serde_json::to_string(&public).unwrap();
        let parsed: PublicActivityWithTimeseries = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, public);
    }

    #[test]
    fn test_public_activity_with_timeseries_serialises_training_context() {
        let activity = ActivityWithTrainingContext::new(
            activity_with_parsed_data(),
            TrainingContext::new(
                Some(70.0),
                vec![BestDurationCurve::new(
                    DurationCurveType::Power,
                    [
                        Some(250.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ],
                )],
            ),
            metrics(),
        );

        let public = PublicActivityWithTimeseries::from(&activity);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&public).unwrap()).unwrap();

        assert_eq!(json["training_context"]["weight"], serde_json::json!(70.0));
        assert_eq!(
            json["training_context"]["best_duration_12w_curves"][0]["curve_type"],
            serde_json::json!("Power")
        );
        assert_eq!(
            json["training_context"]["best_duration_12w_curves"][0]["unit"],
            serde_json::json!("W")
        );
        assert_eq!(
            json["training_context"]["best_duration_12w_curves"][0]["values"][0],
            serde_json::json!(250.0)
        );
        assert_eq!(
            json["training_context"]["best_duration_12w_curves"][0]["values"][1],
            serde_json::json!(null)
        );
    }

    #[test]
    fn test_public_activity_with_timeseries_serialises_empty_training_context_curves() {
        let activity = ActivityWithTrainingContext::new(
            activity_with_parsed_data(),
            TrainingContext::new(Some(70.0), vec![]),
            metrics(),
        );

        let public = PublicActivityWithTimeseries::from(&activity);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&public).unwrap()).unwrap();

        assert_eq!(
            json["training_context"]["best_duration_12w_curves"],
            serde_json::json!([])
        );
    }

    #[test]
    fn test_public_activity_with_timeseries_serialises_null_weight() {
        let activity = ActivityWithTrainingContext::new(
            activity_with_parsed_data(),
            TrainingContext::new(None, vec![]),
            metrics(),
        );

        let public = PublicActivityWithTimeseries::from(&activity);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&public).unwrap()).unwrap();

        assert_eq!(json["training_context"]["weight"], serde_json::json!(null));
    }

    #[test]
    fn test_public_activity_with_timeseries_serialises_duration_curves() {
        let curve = ActivityDurationCurve::new(
            DurationCurveType::Power,
            [
                Some(120.0),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
        );
        let activity = ActivityWithTrainingContext::new(
            activity_with_parsed_data_with_curves(ActivityDurationCurves::new(vec![curve])),
            TrainingContext::new(Some(70.0), vec![]),
            metrics(),
        );

        let public = PublicActivityWithTimeseries::from(&activity);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&public).unwrap()).unwrap();

        assert_eq!(
            json["duration_curves"][0]["curve_type"],
            serde_json::json!("Power")
        );
        assert_eq!(json["duration_curves"][0]["unit"], serde_json::json!("W"));
        assert_eq!(
            json["duration_curves"][0]["values"][0],
            serde_json::json!(120.0)
        );
        assert_eq!(
            json["duration_curves"][0]["values"][1],
            serde_json::json!(null)
        );
    }

    #[test]
    fn test_public_duration_curve_converts_pace_to_kmh() {
        let curve = ActivityDurationCurve::new(
            DurationCurveType::Pace,
            [
                Some(5.0),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
        );

        let public = PublicDurationCurve::from(&curve);

        assert_eq!(public.curve_type, "Pace");
        assert_eq!(public.unit, "km/h");
        assert_eq!(public.values[0], Some(PublicTimeseriesValue::Float(18.0)));
    }

    #[test]
    fn test_public_best_duration_curve_converts_pace_to_kmh() {
        let curve = BestDurationCurve::new(
            DurationCurveType::Pace,
            [
                Some(5.0),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
        );

        let public = PublicDurationCurve::from(&curve);

        assert_eq!(public.curve_type, "Pace");
        assert_eq!(public.unit, "km/h");
        assert_eq!(public.values[0], Some(PublicTimeseriesValue::Float(18.0)));
    }
}
