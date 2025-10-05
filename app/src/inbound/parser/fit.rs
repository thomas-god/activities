use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use fit_parser::{
    DataMessage, DataValue, FitEnum, FitField, FitParserError, MesgNum, RecordField, SessionField,
    Sport as FitSport, parse_fit_messages,
    utils::{find_field_value_as_float, find_field_value_by_kind},
};

use crate::{
    domain::models::activity::{
        ActivityStartTime, ActivityStatistic, ActivityStatistics, ActivityTimeseries, Sport,
        Timeseries, TimeseriesMetric, TimeseriesTime, TimeseriesValue,
    },
    inbound::parser::{ParseBytesError, ParseFile, ParsedFileContent},
};

/// FIT datetimes have 00:00 Dec 31 1989 as their reference instead of January 1, 1970
const FIT_DATETIME_OFFSET: usize = 631065600;

#[derive(Clone)]
pub struct FitParser {}

impl ParseFile for FitParser {
    fn try_bytes_into_domain(&self, bytes: Vec<u8>) -> Result<ParsedFileContent, ParseBytesError> {
        let Ok(messages) = parse_fit_messages(bytes.clone().into_iter(), false) else {
            return Err(ParseBytesError::InvalidFitContent);
        };

        let (start_time, reference_timestamp) =
            extract_start_time(&messages).ok_or(ParseBytesError::NoStartTimeFound)?;

        let sport = extract_sport(&messages);

        let timeseries = extract_timeseries(reference_timestamp, &messages)?;

        let statistics = extract_statistics(&messages);

        Ok(ParsedFileContent::new(
            sport, start_time, statistics, timeseries, bytes,
        ))
    }
}

fn extract_start_time(messages: &[DataMessage]) -> Option<(ActivityStartTime, u32)> {
    let start_timestamp = find_field_value_by_kind(
        messages,
        &fit_parser::FitField::Session(fit_parser::SessionField::StartTime),
    )
    .and_then(|values| {
        values.iter().find_map(|val| match val {
            DataValue::DateTime(dt) => Some(dt),
            _ => None,
        })
    })?;

    let activity_timestamp = find_field_value_by_kind(
        messages,
        &fit_parser::FitField::Activity(fit_parser::ActivityField::Timestamp),
    )
    .and_then(|values| {
        values.iter().find_map(|val| match val {
            DataValue::DateTime(dt) => Some(dt),
            _ => None,
        })
    })?;

    let activity_local_timestamp = find_field_value_by_kind(
        messages,
        &fit_parser::FitField::Activity(fit_parser::ActivityField::LocalTimestamp),
    )
    .and_then(|values| {
        values.iter().find_map(|val| match val {
            DataValue::DateTime(dt) => Some(dt),
            _ => None,
        })
    })
    .unwrap_or(&0);

    let offset = *activity_local_timestamp as isize - *activity_timestamp as isize;

    let start_datetime =
        DateTime::from_timestamp((*start_timestamp as usize + FIT_DATETIME_OFFSET) as i64, 0)?;

    let start_datetime_with_offset = match FixedOffset::east_opt(offset as i32) {
        Some(offset) => start_datetime.with_timezone(&offset),
        None => start_datetime.fixed_offset(),
    };

    Some((
        ActivityStartTime::new(start_datetime_with_offset),
        *start_timestamp,
    ))
}

fn extract_sport(messages: &[DataMessage]) -> Sport {
    find_field_value_by_kind(
        messages,
        &fit_parser::FitField::Session(fit_parser::SessionField::Sport),
    )
    .and_then(|field| {
        field.iter().find_map(|value| match value {
            fit_parser::DataValue::Enum(FitEnum::Sport(sport)) => Some(sport.into()),
            _ => None,
        })
    })
    .unwrap_or(Sport::Other)
}

fn extract_timeseries(
    reference_timestamp: u32,
    messages: &[DataMessage],
) -> Result<ActivityTimeseries, ParseBytesError> {
    let mut time = vec![];
    let mut speed_values = vec![];
    let mut power_values = vec![];
    let mut distance_values = vec![];
    let mut altitude_values = vec![];
    let mut heart_rate_values = vec![];

    for message in messages {
        if message.message_kind != MesgNum::Record {
            continue;
        }

        let Some(timestamp) = message.fields.iter().find_map(|field| match field.kind {
            FitField::Record(RecordField::Timestamp) => {
                field.values.iter().find_map(|val| match val {
                    DataValue::DateTime(timestamp) => timestamp.checked_sub(reference_timestamp),
                    _ => None,
                })
            }
            _ => None,
        }) else {
            continue;
        };
        time.push(timestamp as usize);

        let heart_rate = message.fields.iter().find_map(|field| match field.kind {
            FitField::Record(RecordField::HeartRate) => field.values.iter().find_map(|val| {
                if val.is_invalid() {
                    return None;
                }
                match val {
                    DataValue::Uint8(hr) => Some(TimeseriesValue::Int(*hr as usize)),
                    _ => None,
                }
            }),
            _ => None,
        });
        heart_rate_values.push(heart_rate);

        let speed = message.fields.iter().find_map(|field| match field.kind {
            FitField::Record(RecordField::Speed) | FitField::Record(RecordField::EnhancedSpeed) => {
                field.values.iter().find_map(|val| {
                    if val.is_invalid() {
                        return None;
                    }
                    match val {
                        DataValue::Float32(speed) => Some(TimeseriesValue::Float(*speed as f64)),
                        _ => None,
                    }
                })
            }
            _ => None,
        });
        speed_values.push(speed);

        let power = message.fields.iter().find_map(|field| match field.kind {
            FitField::Record(RecordField::Power) => field.values.iter().find_map(|val| {
                if val.is_invalid() {
                    return None;
                }
                match val {
                    DataValue::Uint16(power) => Some(TimeseriesValue::Int(*power as usize)),
                    _ => None,
                }
            }),
            _ => None,
        });
        power_values.push(power);

        let distance = message.fields.iter().find_map(|field| match field.kind {
            FitField::Record(RecordField::Distance) => field.values.iter().find_map(|val| {
                if val.is_invalid() {
                    return None;
                }
                match val {
                    DataValue::Float32(distance) => Some(TimeseriesValue::Float(*distance as f64)),
                    _ => None,
                }
            }),
            _ => None,
        });
        distance_values.push(distance);

        let altitude = message.fields.iter().find_map(|field| match field.kind {
            FitField::Record(RecordField::Altitude)
            | FitField::Record(RecordField::EnhancedAltitude) => {
                field.values.iter().find_map(|val| {
                    if val.is_invalid() {
                        return None;
                    }
                    match val {
                        DataValue::Float32(altitude) => {
                            Some(TimeseriesValue::Float(*altitude as f64))
                        }
                        _ => None,
                    }
                })
            }
            _ => None,
        });
        altitude_values.push(altitude);
    }

    let metrics = vec![
        Timeseries::new(TimeseriesMetric::Speed, speed_values),
        Timeseries::new(TimeseriesMetric::Distance, distance_values),
        Timeseries::new(TimeseriesMetric::HeartRate, heart_rate_values),
        Timeseries::new(TimeseriesMetric::Power, power_values),
        Timeseries::new(TimeseriesMetric::Altitude, altitude_values),
    ];
    Ok(ActivityTimeseries::new(TimeseriesTime::new(time), metrics))
}

fn extract_statistics(messages: &[DataMessage]) -> ActivityStatistics {
    let mut stats = HashMap::new();
    let pairs = [
        (
            FitField::Session(SessionField::TotalCalories),
            ActivityStatistic::Calories,
        ),
        (
            FitField::Session(SessionField::TotalDistance),
            ActivityStatistic::Distance,
        ),
        (
            FitField::Session(SessionField::TotalAscent),
            ActivityStatistic::Elevation,
        ),
        (
            FitField::Session(SessionField::TotalElapsedTime),
            ActivityStatistic::Duration,
        ),
        (
            FitField::Session(SessionField::NormalizedPower),
            ActivityStatistic::NormalizedPower,
        ),
    ];

    for (field, statistic) in pairs.iter() {
        if let Some(value) = find_field_value_as_float(messages, field) {
            stats.insert(*statistic, value);
        }
    }

    ActivityStatistics::new(stats)
}

impl From<FitParserError> for ParseBytesError {
    fn from(_value: FitParserError) -> Self {
        Self::InvalidFitContent
    }
}

impl From<&FitSport> for Sport {
    fn from(value: &FitSport) -> Self {
        match value {
            FitSport::Running => Self::Running,
            FitSport::Cycling => Self::Cycling,
            FitSport::AlpineSkiing => Self::AlpineSKi,
            FitSport::Swimming => Self::Swimming,
            FitSport::FitnessEquipment => Self::StrengthTraining,
            FitSport::Training => Self::StrengthTraining,
            FitSport::Hiit => Self::StrengthTraining,
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use assert_approx_eq::assert_approx_eq;
    use chrono::{DateTime, FixedOffset, Utc};
    use fit_parser::{ActivityField, DataMessageField};

    use crate::domain::models::activity::{TimeseriesMetric, TimeseriesValue};

    use super::*;

    struct TestMetrics<'a> {
        speed: Option<&'a [Option<TimeseriesValue>]>,
        power: Option<&'a [Option<TimeseriesValue>]>,
        heart_rate: Option<&'a [Option<TimeseriesValue>]>,
        distance: Option<&'a [Option<TimeseriesValue>]>,
    }
    fn extract_metrics_from_req<'a>(req: &'a ParsedFileContent) -> TestMetrics<'a> {
        extract_metrics(req.timeseries())
    }

    fn extract_metrics<'a>(timeseries: &'a ActivityTimeseries) -> TestMetrics<'a> {
        let speed = timeseries
            .metrics()
            .iter()
            .find_map(|metric| match metric.metric() {
                TimeseriesMetric::Speed => Some(metric.values()),
                _ => None,
            });
        let power = timeseries
            .metrics()
            .iter()
            .find_map(|metric| match metric.metric() {
                TimeseriesMetric::Power => Some(metric.values()),
                _ => None,
            });
        let distance = timeseries
            .metrics()
            .iter()
            .find_map(|metric| match metric.metric() {
                TimeseriesMetric::Distance => Some(metric.values()),
                _ => None,
            });
        let heart_rate = timeseries
            .metrics()
            .iter()
            .find_map(|metric| match metric.metric() {
                TimeseriesMetric::HeartRate => Some(metric.values()),
                _ => None,
            });

        TestMetrics {
            speed,
            power,
            heart_rate,
            distance,
        }
    }

    #[test]
    fn test_fit_datetime_reference_utc_offset() {
        let fit_zero_datetime = DateTime::from_timestamp(FIT_DATETIME_OFFSET as i64, 0).unwrap();
        let expected = "1989-12-31T00:00:00Z".parse::<DateTime<Utc>>().unwrap();

        assert_eq!(fit_zero_datetime, expected);
    }

    #[test]
    fn test_parse_fit_timestamp() {
        let content = fs::read("../test.fit").unwrap();
        let parser = FitParser {};

        let res = parser.try_bytes_into_domain(content).unwrap();

        // Check for same point in time
        let expected_time = "2025-08-08T19:14:54+02:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        assert_eq!(*res.start_time().date(), expected_time);

        // Check for correct offset to UTC
        assert_eq!(
            (*res.start_time().date()).to_rfc3339(),
            "2025-08-08T19:14:54+02:00".to_string()
        );
    }

    #[test]
    fn test_parsing_of_timeseries() {
        let content = fs::read("../test.fit").unwrap();
        let parser = FitParser {};

        let res = parser.try_bytes_into_domain(content).unwrap();

        assert_eq!(res.timeseries().time().len(), 3602);

        let TestMetrics {
            speed,
            power,
            heart_rate,
            distance,
        } = extract_metrics_from_req(&res);

        // First timestamp should be 0 (i.e. equal to activity start_time), speed, power and
        // heart rate are none/absent
        assert_eq!(*res.timeseries().time().values().first().unwrap(), 0);
        assert_eq!(
            *distance.unwrap().first().unwrap(),
            Some(TimeseriesValue::Float(0.0))
        );
        assert!(speed.unwrap().first().unwrap().is_none());
        assert!(power.unwrap().first().unwrap().is_none());
        assert!(heart_rate.unwrap().first().unwrap().is_none());

        // Check 4th element as the first 3 have no power/speed/hr data
        assert_eq!(*res.timeseries().time().values().get(3).unwrap(), 3);
        assert_eq!(
            *distance.unwrap().get(3).unwrap(),
            Some(TimeseriesValue::Float(0.0))
        );
        match speed.unwrap().get(3).unwrap().as_ref().unwrap() {
            TimeseriesValue::Float(val) => assert_approx_eq!(val, 3.969),
            _ => unreachable!("Should be a float"),
        }
        assert_eq!(
            power.unwrap().get(3).unwrap().as_ref().unwrap(),
            &TimeseriesValue::Int(74)
        );
        assert_eq!(
            heart_rate.unwrap().get(3).unwrap().as_ref().unwrap(),
            &TimeseriesValue::Int(77)
        );
    }

    #[test]
    fn test_extract_timeseries_timestamp_from_activity_start_reference() {
        let messages = vec![DataMessage {
            local_message_type: 0,
            message_kind: MesgNum::Record,
            fields: vec![DataMessageField {
                kind: FitField::Record(RecordField::Timestamp),
                values: vec![DataValue::DateTime(10)],
            }],
        }];
        let reference = 10;

        let timeseries = extract_timeseries(reference, &messages);
        assert!(timeseries.is_ok());
        let timeseries = timeseries.unwrap();

        assert_eq!(timeseries.time().len(), 1);
        assert_eq!(*timeseries.time().values().first().unwrap(), 0);
    }

    #[test]
    fn test_extract_timeseries_skip_records_without_timestamp() {
        let messages = vec![DataMessage {
            local_message_type: 0,
            message_kind: MesgNum::Record,
            fields: vec![DataMessageField {
                kind: FitField::Record(RecordField::Power),
                values: vec![DataValue::Uint16(12)],
            }],
        }];
        let reference = 10;

        let timeseries = extract_timeseries(reference, &messages);
        assert!(timeseries.is_ok());
        let timeseries = timeseries.unwrap();

        assert_eq!(timeseries.time().len(), 0);
    }

    #[test]
    fn test_extract_timeseries_skip_records_before_reference() {
        let messages = vec![DataMessage {
            local_message_type: 0,
            message_kind: MesgNum::Record,
            fields: vec![DataMessageField {
                kind: FitField::Record(RecordField::Timestamp),
                values: vec![DataValue::DateTime(5)],
            }],
        }];
        let reference = 10;

        let timeseries = extract_timeseries(reference, &messages);
        assert!(timeseries.is_ok());
        let timeseries = timeseries.unwrap();

        assert_eq!(timeseries.time().len(), 0);
    }

    #[test]
    fn test_extract_timeseries_skip_invalid_power_values() {
        let messages = vec![
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Record,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Record(RecordField::Timestamp),
                        values: vec![DataValue::DateTime(10)],
                    },
                    DataMessageField {
                        kind: FitField::Record(RecordField::Power),
                        values: vec![DataValue::Uint16(12)],
                    },
                ],
            },
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Record,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Record(RecordField::Timestamp),
                        values: vec![DataValue::DateTime(11)],
                    },
                    DataMessageField {
                        kind: FitField::Record(RecordField::Power),
                        values: vec![DataValue::Uint16(u16::MAX)],
                    },
                ],
            },
        ];
        let reference = 10;

        let timeseries = extract_timeseries(reference, &messages);
        assert!(timeseries.is_ok());
        let timeseries = timeseries.unwrap();

        let TestMetrics { power, .. } = extract_metrics(&timeseries);
        let power = power.unwrap();

        assert_eq!(timeseries.time().len(), 2);

        assert_eq!(*timeseries.time().values().first().unwrap(), 0);
        assert_eq!(*power.first().unwrap(), Some(TimeseriesValue::Int(12)));

        assert_eq!(*timeseries.time().values().get(1).unwrap(), 1);
        assert_eq!(*power.get(1).unwrap(), None);
    }

    #[test]
    fn test_extract_timeseries_skip_invalid_heart_rate_values() {
        let messages = vec![
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Record,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Record(RecordField::Timestamp),
                        values: vec![DataValue::DateTime(10)],
                    },
                    DataMessageField {
                        kind: FitField::Record(RecordField::HeartRate),
                        values: vec![DataValue::Uint8(120)],
                    },
                ],
            },
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Record,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Record(RecordField::Timestamp),
                        values: vec![DataValue::DateTime(11)],
                    },
                    DataMessageField {
                        kind: FitField::Record(RecordField::HeartRate),
                        values: vec![DataValue::Uint8(u8::MAX)],
                    },
                ],
            },
        ];
        let reference = 10;

        let timeseries = extract_timeseries(reference, &messages);
        assert!(timeseries.is_ok());
        let timeseries = timeseries.unwrap();

        let TestMetrics { heart_rate, .. } = extract_metrics(&timeseries);
        let hear_rate = heart_rate.unwrap();

        assert_eq!(timeseries.time().len(), 2);

        assert_eq!(*timeseries.time().values().first().unwrap(), 0);
        assert_eq!(*hear_rate.first().unwrap(), Some(TimeseriesValue::Int(120)));

        assert_eq!(*timeseries.time().values().get(1).unwrap(), 1);
        assert_eq!(*hear_rate.get(1).unwrap(), None);
    }

    #[test]
    fn test_extract_timeseries_skip_invalid_distance_values() {
        let messages = vec![
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Record,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Record(RecordField::Timestamp),
                        values: vec![DataValue::DateTime(10)],
                    },
                    DataMessageField {
                        kind: FitField::Record(RecordField::Distance),
                        values: vec![DataValue::Float32(120.)],
                    },
                ],
            },
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Record,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Record(RecordField::Timestamp),
                        values: vec![DataValue::DateTime(11)],
                    },
                    DataMessageField {
                        kind: FitField::Record(RecordField::Distance),
                        values: vec![DataValue::Uint32(u32::MAX)],
                    },
                ],
            },
        ];
        let reference = 10;

        let timeseries = extract_timeseries(reference, &messages);
        assert!(timeseries.is_ok());
        let timeseries = timeseries.unwrap();

        let TestMetrics { distance, .. } = extract_metrics(&timeseries);
        let distance = distance.unwrap();

        assert_eq!(timeseries.time().len(), 2);

        assert_eq!(*timeseries.time().values().first().unwrap(), 0);
        assert_eq!(
            *distance.first().unwrap(),
            Some(TimeseriesValue::Float(120.))
        );

        assert_eq!(*timeseries.time().values().get(1).unwrap(), 1);
        assert_eq!(*distance.get(1).unwrap(), None);
    }

    #[test]
    fn test_extract_timeseries_skip_invalid_speed_values() {
        let messages = vec![
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Record,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Record(RecordField::Timestamp),
                        values: vec![DataValue::DateTime(10)],
                    },
                    DataMessageField {
                        kind: FitField::Record(RecordField::Speed),
                        values: vec![DataValue::Float32(12.)],
                    },
                ],
            },
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Record,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Record(RecordField::Timestamp),
                        values: vec![DataValue::DateTime(11)],
                    },
                    DataMessageField {
                        kind: FitField::Record(RecordField::Speed),
                        values: vec![DataValue::Float32(f32::from_le_bytes([
                            0xFF, 0xFF, 0xFF, 0xFF,
                        ]))],
                    },
                ],
            },
        ];
        let reference = 10;

        let timeseries = extract_timeseries(reference, &messages);
        assert!(timeseries.is_ok());
        let timeseries = timeseries.unwrap();

        let TestMetrics { speed, .. } = extract_metrics(&timeseries);
        let speed = speed.unwrap();

        assert_eq!(timeseries.time().len(), 2);

        assert_eq!(*timeseries.time().values().first().unwrap(), 0);
        assert_eq!(*speed.first().unwrap(), Some(TimeseriesValue::Float(12.)));

        assert_eq!(*timeseries.time().values().get(1).unwrap(), 1);
        assert_eq!(*speed.get(1).unwrap(), None);
    }

    #[test]
    fn test_extract_timeseries_enhanced_speed_field_as_speed() {
        let messages = vec![DataMessage {
            local_message_type: 0,
            message_kind: MesgNum::Record,
            fields: vec![
                DataMessageField {
                    kind: FitField::Record(RecordField::Timestamp),
                    values: vec![DataValue::DateTime(10)],
                },
                DataMessageField {
                    kind: FitField::Record(RecordField::EnhancedSpeed),
                    values: vec![DataValue::Float32(12.)],
                },
            ],
        }];
        let reference = 10;

        let timeseries = extract_timeseries(reference, &messages);
        assert!(timeseries.is_ok());
        let timeseries = timeseries.unwrap();

        let TestMetrics { speed, .. } = extract_metrics(&timeseries);
        let speed = speed.unwrap();

        assert_eq!(timeseries.time().len(), 1);

        assert_eq!(*timeseries.time().values().first().unwrap(), 0);
        assert_eq!(*speed.first().unwrap(), Some(TimeseriesValue::Float(12.)));
    }

    #[test]
    fn test_extract_start_time_ok_with_timezone() {
        let messages = vec![
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Session,
                fields: vec![DataMessageField {
                    kind: FitField::Session(SessionField::StartTime),
                    values: vec![DataValue::DateTime(983185076)],
                }],
            },
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Activity,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Activity(ActivityField::Timestamp),
                        values: vec![DataValue::DateTime(983187416)],
                    },
                    DataMessageField {
                        kind: FitField::Activity(ActivityField::LocalTimestamp),
                        values: vec![DataValue::DateTime(983191016)],
                    },
                ],
            },
        ];

        let (start, reference_timestamp) =
            extract_start_time(&messages).expect("Should have returned Some");

        assert_eq!(reference_timestamp, 983185076);
        assert_eq!(
            start,
            ActivityStartTime::new(
                "2021-02-25T11:57:56+01:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap()
            )
        )
    }

    #[test]
    fn test_extract_start_time_ok_with_invalid_local_timestamp() {
        let messages = vec![
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Session,
                fields: vec![DataMessageField {
                    kind: FitField::Session(SessionField::StartTime),
                    values: vec![DataValue::DateTime(983185076)],
                }],
            },
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Activity,
                fields: vec![
                    DataMessageField {
                        kind: FitField::Activity(ActivityField::Timestamp),
                        values: vec![DataValue::DateTime(983187416)],
                    },
                    DataMessageField {
                        kind: FitField::Activity(ActivityField::LocalTimestamp),
                        values: vec![DataValue::DateTime(0)],
                    },
                ],
            },
        ];

        let (start, reference_timestamp) =
            extract_start_time(&messages).expect("Should have returned Some");

        assert_eq!(reference_timestamp, 983185076);
        assert_eq!(
            start,
            ActivityStartTime::new(
                "2021-02-25T10:57:56Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap()
            )
        )
    }

    #[test]
    fn test_extract_start_time_ok_without_local_timestamp() {
        let messages = vec![
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Session,
                fields: vec![DataMessageField {
                    kind: FitField::Session(SessionField::StartTime),
                    values: vec![DataValue::DateTime(983185076)],
                }],
            },
            DataMessage {
                local_message_type: 0,
                message_kind: MesgNum::Activity,
                fields: vec![DataMessageField {
                    kind: FitField::Activity(ActivityField::Timestamp),
                    values: vec![DataValue::DateTime(983187416)],
                }],
            },
        ];

        let (start, reference_timestamp) =
            extract_start_time(&messages).expect("Should have returned Some");

        assert_eq!(reference_timestamp, 983185076);
        assert_eq!(
            start,
            ActivityStartTime::new(
                "2021-02-25T10:57:56Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap()
            )
        )
    }
}
