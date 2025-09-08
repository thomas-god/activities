use chrono::{DateTime, FixedOffset};
use fit_parser::{
    DataMessage, DataValue, FitEnum, FitField, FitParserError, MesgNum, RecordField,
    Sport as FitSport, parse_fit_messages,
    utils::{find_field_value_as_float, find_field_value_by_kind},
};
use thiserror::Error;

use crate::domain::{
    models::{
        ActivityDuration, ActivityStartTime, Sport, Timeseries, TimeseriesItem, TimeseriesMetric,
        TimeseriesTime,
    },
    ports::CreateActivityRequest,
};

/// FIT datetimes have 00:00 Dec 31 1989 as their reference instead of January 1, 1970
const FIT_DATETIME_OFFSET: usize = 631065600;

pub trait ParseFile: Clone + Send + Sync + 'static {
    fn try_bytes_into_domain(
        &self,
        bytes: Vec<u8>,
    ) -> Result<CreateActivityRequest, ParseCreateActivityHttpRequestBodyError>;
}

#[derive(Clone)]
pub struct FitParser {}

impl ParseFile for FitParser {
    fn try_bytes_into_domain(
        &self,
        bytes: Vec<u8>,
    ) -> Result<CreateActivityRequest, ParseCreateActivityHttpRequestBodyError> {
        let Ok(messages) = parse_fit_messages(bytes.clone().into_iter()) else {
            return Err(ParseCreateActivityHttpRequestBodyError::InvalidFitContent);
        };

        let duration = extract_duration(&messages)
            .ok_or(ParseCreateActivityHttpRequestBodyError::NoDurationFound)?;

        let (start_time, reference_timestamp) = extract_start_time(&messages)
            .ok_or(ParseCreateActivityHttpRequestBodyError::NoStartTimeFound)?;

        let sport = extract_sport(&messages);

        let timeseries = extract_timeseries(reference_timestamp, &messages)?;

        Ok(CreateActivityRequest::new(
            sport, duration, start_time, timeseries, bytes,
        ))
    }
}

fn extract_start_time(messages: &[DataMessage]) -> Option<(ActivityStartTime, u32)> {
    let start_timestamp = find_field_value_by_kind(
        messages,
        &fit_parser::FitField::Activity(fit_parser::ActivityField::Timestamp),
    )
    .and_then(|values| {
        values.iter().find_map(|val| match val {
            DataValue::DateTime(dt) => Some(dt),
            _ => None,
        })
    })?;

    let start_local_timestamp = find_field_value_by_kind(
        messages,
        &fit_parser::FitField::Activity(fit_parser::ActivityField::LocalTimestamp),
    )
    .and_then(|values| {
        values.iter().find_map(|val| match val {
            DataValue::DateTime(dt) => Some(dt),
            _ => None,
        })
    })?;

    let offset = *start_local_timestamp as isize - *start_timestamp as isize;

    let start_datetime =
        DateTime::from_timestamp((*start_timestamp as usize + FIT_DATETIME_OFFSET) as i64, 0)?;
    let start_datetime = start_datetime.with_timezone(&FixedOffset::east_opt(offset as i32)?);

    Some((ActivityStartTime::new(start_datetime), *start_timestamp))
}

fn extract_duration(messages: &[DataMessage]) -> Option<ActivityDuration> {
    find_field_value_as_float(
        messages,
        &fit_parser::FitField::Session(fit_parser::SessionField::TotalElapsedTime),
    )
    .map(|val| ActivityDuration(val.round() as usize))
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
) -> Result<Timeseries, ParseCreateActivityHttpRequestBodyError> {
    Ok(Timeseries::new(
        messages
            .iter()
            .filter_map(|msg| {
                if msg.message_kind != MesgNum::Record {
                    return None;
                }

                let timestamp = msg.fields.iter().find_map(|field| match field.kind {
                    FitField::Record(RecordField::Timestamp) => {
                        field.values.iter().find_map(|val| match val {
                            DataValue::DateTime(timestamp) => {
                                timestamp.checked_sub(reference_timestamp)
                            }
                            _ => None,
                        })
                    }
                    _ => None,
                })?;

                let mut metrics = vec![];

                if let Some(heart_rate) = msg.fields.iter().find_map(|field| match field.kind {
                    FitField::Record(RecordField::HeartRate) => {
                        field.values.iter().find_map(|val| {
                            if val.is_invalid() {
                                return None;
                            }
                            match val {
                                DataValue::Uint8(hr) => Some(*hr),
                                _ => None,
                            }
                        })
                    }
                    _ => None,
                }) {
                    metrics.push(TimeseriesMetric::HeartRate(heart_rate as usize));
                };

                if let Some(speed) = msg.fields.iter().find_map(|field| match field.kind {
                    FitField::Record(RecordField::Speed) => field.values.iter().find_map(|val| {
                        if val.is_invalid() {
                            return None;
                        }
                        match val {
                            DataValue::Float32(speed) => Some(*speed),
                            _ => None,
                        }
                    }),
                    _ => None,
                }) {
                    metrics.push(TimeseriesMetric::Speed(speed as f64));
                };

                if let Some(power) = msg.fields.iter().find_map(|field| match field.kind {
                    FitField::Record(RecordField::Power) => field.values.iter().find_map(|val| {
                        if val.is_invalid() {
                            return None;
                        }
                        match val {
                            DataValue::Uint16(power) => Some(*power),
                            _ => None,
                        }
                    }),
                    _ => None,
                }) {
                    metrics.push(TimeseriesMetric::Power(power as usize));
                };

                Some(TimeseriesItem::new(
                    TimeseriesTime::new(timestamp as usize),
                    metrics,
                ))
            })
            .collect(),
    ))
}

#[derive(Debug, Clone, Error)]
pub enum ParseCreateActivityHttpRequestBodyError {
    #[error("Error when parsing FIT content")]
    InvalidFitContent,
    #[error("No start time data found in activity file")]
    NoStartTimeFound,
    #[error("No activity duration data found in activity file")]
    NoDurationFound,
}

impl From<FitParserError> for ParseCreateActivityHttpRequestBodyError {
    fn from(_value: FitParserError) -> Self {
        Self::InvalidFitContent
    }
}

impl From<&FitSport> for Sport {
    fn from(value: &FitSport) -> Self {
        match value {
            FitSport::Running => Self::Running,
            FitSport::Cycling => Self::Cycling,
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
pub mod test_utils {

    use std::{
        mem,
        sync::{Arc, Mutex},
    };

    use super::*;

    #[derive(Clone)]
    pub struct MockFileParser {
        pub try_into_domain_result:
            Arc<Mutex<Result<CreateActivityRequest, ParseCreateActivityHttpRequestBodyError>>>,
    }

    impl ParseFile for MockFileParser {
        fn try_bytes_into_domain(
            &self,
            _bytes: Vec<u8>,
        ) -> Result<CreateActivityRequest, ParseCreateActivityHttpRequestBodyError> {
            let mut guard = self.try_into_domain_result.lock();
            let mut result = Err(ParseCreateActivityHttpRequestBodyError::InvalidFitContent);
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }
    }

    impl Default for MockFileParser {
        fn default() -> Self {
            Self {
                try_into_domain_result: Arc::new(Mutex::new(Ok(CreateActivityRequest::new(
                    Sport::Cycling,
                    ActivityDuration(3600),
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    Timeseries::new(vec![]),
                    vec![1, 2, 3],
                )))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use assert_approx_eq::assert_approx_eq;
    use chrono::{DateTime, FixedOffset, Utc};
    use fit_parser::DataMessageField;

    use super::*;

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
        assert_eq!(**res.start_time(), expected_time);

        // Check for correct offset to UTC
        assert_eq!(
            (**res.start_time()).to_rfc3339(),
            "2025-08-08T19:14:54+02:00".to_string()
        );
    }

    #[test]
    fn test_parsing_of_timeseries() {
        let content = fs::read("../test.fit").unwrap();
        let parser = FitParser {};

        let res = parser.try_bytes_into_domain(content).unwrap();

        assert_eq!(res.timeseries().len(), 3602);

        let first = res.timeseries().first().unwrap();

        // First timestamp should 0 (i.e. equal to activity start_time)
        assert_eq!(*first.time(), TimeseriesTime::new(0));
        assert_eq!(*first.metrics(), vec![]);

        // Check 4th element as the first 3 have no power/speed/hr data
        let fourth = res.timeseries().get(3).unwrap();
        assert_eq!(*fourth.time(), TimeseriesTime::new(3));
        let metrics = fourth.metrics();
        assert_eq!(*metrics.first().unwrap(), TimeseriesMetric::HeartRate(77));
        if let TimeseriesMetric::Speed(speed) = *metrics.get(1).unwrap() {
            assert_approx_eq!(speed, 3.969);
        }
        assert_eq!(*metrics.get(2).unwrap(), TimeseriesMetric::Power(74));
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

        assert_eq!(timeseries.len(), 1);
        assert_eq!(timeseries.first().unwrap().time(), &TimeseriesTime::new(0));
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

        assert_eq!(timeseries.len(), 0);
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

        assert_eq!(timeseries.len(), 0);
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

        assert_eq!(timeseries.len(), 2);
        assert_eq!(
            timeseries.first().unwrap(),
            &TimeseriesItem::new(TimeseriesTime::new(0), vec![TimeseriesMetric::Power(12)])
        );
        assert_eq!(
            timeseries.get(1).unwrap(),
            &TimeseriesItem::new(TimeseriesTime::new(1), vec![])
        )
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

        assert_eq!(timeseries.len(), 2);
        assert_eq!(
            timeseries.first().unwrap(),
            &TimeseriesItem::new(
                TimeseriesTime::new(0),
                vec![TimeseriesMetric::HeartRate(120)]
            )
        );
        assert_eq!(
            timeseries.get(1).unwrap(),
            &TimeseriesItem::new(TimeseriesTime::new(1), vec![])
        )
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

        assert_eq!(timeseries.len(), 2);
        assert_eq!(
            timeseries.first().unwrap(),
            &TimeseriesItem::new(TimeseriesTime::new(0), vec![TimeseriesMetric::Speed(12.)])
        );
        assert_eq!(
            timeseries.get(1).unwrap(),
            &TimeseriesItem::new(TimeseriesTime::new(1), vec![])
        )
    }
}
