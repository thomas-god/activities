use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::{
    domain::models::activity::{
        ActivityDuration, ActivityStartTime, ActivityStatistic, ActivityStatistics,
        ActivityTimeseries, Sport,
    },
    inbound::parser::{ParseBytesError, ParsedFileContent, SupportedExtension},
};

/// Custom JSON-based format to persist standalone activities, i.e. ones manually created by an user
/// to represent an activity not recorded with a sport device. It is supposed to contains minimal
/// information to track basic metrics, and especially is not intended to contains any timeseries.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StandaloneActivity {
    start_time: DateTime<FixedOffset>,
    duration: f64,
    sport: Sport,
    distance: Option<f64>,
    elevation: Option<f64>,
    calories: Option<f64>,
}

pub fn try_custom_json_bytes_into_domain(
    bytes: Vec<u8>,
) -> Result<ParsedFileContent, ParseBytesError> {
    let activity = serde_json::from_slice::<StandaloneActivity>(&bytes)
        .map_err(|_err| ParseBytesError::InvalidContent)?;

    let mut statistics = HashMap::from([(ActivityStatistic::Duration, activity.duration)]);
    if let Some(calories) = activity.calories {
        statistics.insert(ActivityStatistic::Calories, calories);
    }
    if let Some(elevation) = activity.elevation {
        statistics.insert(ActivityStatistic::Elevation, elevation);
    }
    if let Some(distance) = activity.distance {
        statistics.insert(ActivityStatistic::Distance, distance);
    }

    Ok(ParsedFileContent {
        sport: activity.sport,
        start_time: ActivityStartTime::from(activity.start_time),
        duration: ActivityDuration::from(activity.duration),
        statistics: ActivityStatistics::new(statistics),
        timeseries: ActivityTimeseries::empty(),
        extension: SupportedExtension::CustomJSON.suffix().to_string(),
        raw_content: bytes,
    })
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use crate::domain::models::activity::ActivityStatistic;

    use super::*;

    fn make_bytes(json: &str) -> Vec<u8> {
        json.as_bytes().to_vec()
    }

    #[test]
    fn test_minimal_valid_activity() {
        let json = r#"{
            "start_time": "2024-03-15T08:30:00+01:00",
            "duration": 3600.0,
            "sport": "Running"
        }"#;

        let result = try_custom_json_bytes_into_domain(make_bytes(json));
        let content = result.unwrap();

        let expected_time = "2024-03-15T08:30:00+01:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        assert_eq!(*content.start_time().date(), expected_time);
        assert_eq!(*content.duration().as_f64(), 3600.0);
        assert_eq!(content.sport(), &Sport::Running);
        assert_eq!(
            content.statistics().get(&ActivityStatistic::Duration),
            Some(&3600.0)
        );
        assert_eq!(content.statistics().get(&ActivityStatistic::Calories), None);
        assert_eq!(
            content.statistics().get(&ActivityStatistic::Elevation),
            None
        );
        assert_eq!(content.statistics().get(&ActivityStatistic::Distance), None);
        assert!(content.timeseries().metrics().is_empty());
    }

    #[test]
    fn test_activity_with_all_optional_fields() {
        let json = r#"{
            "start_time": "2024-06-01T06:00:00+00:00",
            "duration": 7200.0,
            "sport": "Cycling",
            "distance": 80000.0,
            "elevation": 500.0,
            "calories": 1200.0
        }"#;

        let content = try_custom_json_bytes_into_domain(make_bytes(json)).unwrap();

        assert_eq!(content.sport(), &Sport::Cycling);
        assert_eq!(
            content.statistics().get(&ActivityStatistic::Duration),
            Some(&7200.0)
        );
        assert_eq!(
            content.statistics().get(&ActivityStatistic::Distance),
            Some(&80000.0)
        );
        assert_eq!(
            content.statistics().get(&ActivityStatistic::Elevation),
            Some(&500.0)
        );
        assert_eq!(
            content.statistics().get(&ActivityStatistic::Calories),
            Some(&1200.0)
        );
    }

    #[test]
    fn test_invalid_json_returns_error() {
        let result = try_custom_json_bytes_into_domain(make_bytes("not json"));
        assert!(matches!(result, Err(ParseBytesError::InvalidContent)));
    }

    #[test]
    fn test_missing_required_field_returns_error() {
        let json = r#"{"duration": 3600.0, "sport": "Running"}"#;
        let result = try_custom_json_bytes_into_domain(make_bytes(json));
        assert!(matches!(result, Err(ParseBytesError::InvalidContent)));
    }

    #[test]
    fn test_unknown_sport_returns_error() {
        let json = r#"{
            "start_time": "2024-03-15T08:30:00+01:00",
            "duration": 3600.0,
            "sport": "UnknownSport"
        }"#;
        let result = try_custom_json_bytes_into_domain(make_bytes(json));
        assert!(matches!(result, Err(ParseBytesError::InvalidContent)));
    }

    #[test]
    fn test_raw_content_preserved() {
        let json =
            r#"{"start_time":"2024-03-15T08:30:00+01:00","duration":1800.0,"sport":"Walking"}"#;
        let bytes = make_bytes(json);
        let content = try_custom_json_bytes_into_domain(bytes.clone()).unwrap();
        assert_eq!(content.raw_content(), bytes.as_slice());
    }
}
