use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use fit_parser::{
    DataMessage, DataValue, Event, EventField, EventType, FitEnum, FitField, FitParserError,
    LapField, MesgNum, RecordField, SessionField, Sport as FitSport, SubSport as FitSubSport,
    parse_fit_messages,
    utils::{find_field_value_as_float, find_field_value_by_kind},
};

use crate::{
    domain::models::activity::{
        ActiveTime, ActivityStartTime, ActivityStatistic, ActivityStatistics, ActivityTimeseries,
        Lap, Sport, Timeseries, TimeseriesActiveTime, TimeseriesMetric, TimeseriesTime,
        TimeseriesValue,
    },
    inbound::parser::{ParseBytesError, ParsedFileContent, SupportedExtension},
};

/// FIT datetimes have 00:00 Dec 31 1989 as their reference instead of January 1, 1970
const FIT_DATETIME_OFFSET: usize = 631065600;

pub fn try_fit_bytes_into_domain(bytes: Vec<u8>) -> Result<ParsedFileContent, ParseBytesError> {
    let Ok(messages) = parse_fit_messages(bytes.clone().into_iter(), false) else {
        return Err(ParseBytesError::InvalidContent);
    };

    let (start_time, reference_timestamp) =
        extract_start_time(&messages).ok_or(ParseBytesError::NoStartTimeFound)?;

    let sport = extract_sport(&messages);

    let timeseries = extract_timeseries(reference_timestamp, &messages)?;

    let statistics = extract_statistics(&messages);

    Ok(ParsedFileContent::new(
        sport,
        start_time,
        statistics,
        timeseries,
        SupportedExtension::FIT.suffix().to_string(),
        bytes,
    ))
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
    let Some(sport) = find_field_value_by_kind(
        messages,
        &fit_parser::FitField::Session(fit_parser::SessionField::Sport),
    )
    .and_then(|field| {
        field.iter().find_map(|value| match value {
            fit_parser::DataValue::Enum(FitEnum::Sport(sport)) => Some(sport),
            _ => None,
        })
    }) else {
        return Sport::Other;
    };

    let sub_sport = find_field_value_by_kind(
        messages,
        &fit_parser::FitField::Session(fit_parser::SessionField::SubSport),
    )
    .and_then(|field| {
        field.iter().find_map(|value| match value {
            fit_parser::DataValue::Enum(FitEnum::SubSport(sub_sport)) => Some(sub_sport),
            _ => None,
        })
    });

    Sport::from((sport, sub_sport))
}

fn extract_timeseries(
    reference_timestamp: u32,
    messages: &[DataMessage],
) -> Result<ActivityTimeseries, ParseBytesError> {
    let mut time = vec![];
    let mut active_time = vec![];
    let mut speed_values = vec![];
    let mut pace_values = vec![];
    let mut power_values = vec![];
    let mut cadence_values = vec![];
    let mut distance_values = vec![];
    let mut altitude_values = vec![];
    let mut heart_rate_values = vec![];

    let mut laps = vec![];

    let mut pauses_duration = 0;
    let mut paused = false;
    let mut last_paused_timestamp = None;

    for message in messages {
        if message.message_kind == MesgNum::Event {
            let Some((event, timestamp)) = extract_pause_event(message, reference_timestamp) else {
                continue;
            };

            match event {
                PauseEvent::Start => {
                    if paused {
                        paused = false;
                        if let Some(last_dt) = last_paused_timestamp {
                            let pause_duration = timestamp - last_dt;
                            pauses_duration += pause_duration;
                        }
                        last_paused_timestamp = None;
                    }
                }
                PauseEvent::Stop => {
                    if !paused {
                        paused = true;
                        last_paused_timestamp = Some(timestamp);
                    }
                }
            }
        }

        if message.message_kind == MesgNum::Lap {
            if let Some(lap) = extract_lap(message, reference_timestamp) {
                laps.push(lap);
            } else {
                continue;
            }
        }

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
        active_time.push(if paused {
            ActiveTime::Paused
        } else {
            ActiveTime::Running((timestamp - pauses_duration) as usize)
        });

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
        speed_values.push(speed.clone());
        pace_values.push(speed.map(|val| val.inverse()).flatten());

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

        let cadence = message.fields.iter().find_map(|field| match field.kind {
            FitField::Record(RecordField::Cadence) => field.values.iter().find_map(|val| {
                if val.is_invalid() {
                    return None;
                }
                match val {
                    DataValue::Uint8(cadence) => Some(TimeseriesValue::Int(*cadence as usize)),
                    _ => None,
                }
            }),
            _ => None,
        });
        cadence_values.push(cadence);

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
        Timeseries::new(TimeseriesMetric::Pace, pace_values),
        Timeseries::new(TimeseriesMetric::Distance, distance_values),
        Timeseries::new(TimeseriesMetric::HeartRate, heart_rate_values),
        Timeseries::new(TimeseriesMetric::Power, power_values),
        Timeseries::new(TimeseriesMetric::Cadence, cadence_values),
        Timeseries::new(TimeseriesMetric::Altitude, altitude_values),
    ];

    ActivityTimeseries::new(
        TimeseriesTime::new(time),
        TimeseriesActiveTime::new(active_time),
        laps,
        metrics,
    )
    .map_err(|_err| ParseBytesError::IncoherentTimeseriesLengths)
}

enum PauseEvent {
    Start,
    Stop,
}

fn extract_pause_event(
    message: &DataMessage,
    reference_timestamp: u32,
) -> Option<(PauseEvent, u32)> {
    let event_type = message.fields.iter().find_map(|field| match field.kind {
        FitField::Event(EventField::EventType) => {
            field.values.iter().find_map(|value| match value {
                DataValue::Enum(FitEnum::EventType(event_type)) => Some(event_type.clone()),
                _ => None,
            })
        }
        _ => None,
    })?;

    // Only timer related events
    message.fields.iter().find_map(|field| match field.kind {
        FitField::Event(EventField::Event) => field.values.iter().find_map(|value| match value {
            DataValue::Enum(FitEnum::Event(Event::Timer)) => Some(()),
            _ => None,
        }),
        _ => None,
    })?;

    let timestamp = message.fields.iter().find_map(|field| match field.kind {
        FitField::Event(EventField::Timestamp) => {
            field.values.iter().find_map(|value| match value {
                DataValue::DateTime(dt) => dt.checked_sub(reference_timestamp),
                _ => None,
            })
        }
        _ => None,
    })?;

    let pause_event = match event_type {
        EventType::Start => PauseEvent::Start,
        EventType::Stop | EventType::StopAll => PauseEvent::Stop,
        _ => return None,
    };

    Some((pause_event, timestamp))
}

fn extract_lap(message: &DataMessage, reference_timestamp: u32) -> Option<Lap> {
    let start_timestamp = message
        .fields
        .iter()
        .find(|field| field.kind == FitField::Lap(LapField::StartTime))
        .and_then(|field| {
            field.values.iter().find_map(|value| match value {
                DataValue::DateTime(dt) => Some(*dt),
                _ => None,
            })
        })?;

    let end_timestamp = message
        .fields
        .iter()
        .find(|field| field.kind == FitField::Lap(LapField::Timestamp))
        .and_then(|field| {
            field.values.iter().find_map(|value| match value {
                DataValue::DateTime(dt) => Some(*dt),
                _ => None,
            })
        })?;

    Some(Lap::new(
        (start_timestamp - reference_timestamp) as usize,
        (end_timestamp - reference_timestamp) as usize,
    ))
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
        Self::InvalidContent
    }
}

impl From<(&FitSport, Option<&FitSubSport>)> for Sport {
    fn from(value: (&FitSport, Option<&FitSubSport>)) -> Self {
        // Try to parse existing FIT sub sport first
        if let Some(sport) = match value.1 {
            Some(FitSubSport::Treadmill) => Some(Sport::IndoorRunning),
            Some(FitSubSport::IndoorRunning) => Some(Sport::IndoorRunning),
            Some(FitSubSport::Trail) => Some(Sport::TrailRunning),
            Some(FitSubSport::Track) => Some(Sport::TrackRunning),

            Some(FitSubSport::IndoorCycling) => Some(Sport::IndoorCycling),
            Some(FitSubSport::Road) => Some(Sport::Cycling),
            Some(FitSubSport::Mountain) => Some(Sport::MountainBiking),
            Some(FitSubSport::Downhill) => Some(Sport::MountainBiking),
            Some(FitSubSport::Cyclocross) => Some(Sport::Cyclocross),
            Some(FitSubSport::TrackCycling) => Some(Sport::TrackCycling),
            Some(FitSubSport::GravelCycling) => Some(Sport::GravelCycling),
            Some(FitSubSport::EBikeMountain) => Some(Sport::EBiking),

            Some(FitSubSport::IndoorRowing) => Some(Sport::IndoorRowing),
            Some(FitSubSport::Elliptical) => Some(Sport::CardioTraining),
            Some(FitSubSport::StairClimbing) => Some(Sport::CardioTraining),
            Some(FitSubSport::Hiit) => Some(Sport::Hiit),

            Some(FitSubSport::LapSwimming) => Some(Sport::Swimming),
            Some(FitSubSport::OpenWater) => Some(Sport::OpenWaterSwimming),

            Some(FitSubSport::FlexibilityTraining) => Some(Sport::CardioTraining),
            Some(FitSubSport::StrengthTraining) => Some(Sport::StrengthTraining),
            Some(FitSubSport::CardioTraining) => Some(Sport::CardioTraining),
            Some(FitSubSport::IndoorWalking) => Some(Sport::CardioTraining),

            Some(FitSubSport::CasualWalking) => Some(Sport::Walking),
            Some(FitSubSport::SpeedWalking) => Some(Sport::Walking),

            Some(FitSubSport::Whitewater) => Some(Sport::Whitewater),

            Some(FitSubSport::Yoga) => Some(Sport::Yoga),
            Some(FitSubSport::Pilates) => Some(Sport::Pilates),

            Some(FitSubSport::IndoorClimbing) => Some(Sport::IndoorClimbing),
            Some(FitSubSport::Bouldering) => Some(Sport::Bouldering),

            Some(FitSubSport::Pickleball) => Some(Sport::Pickleball),
            Some(FitSubSport::Padel) => Some(Sport::Padel),
            Some(FitSubSport::Squash) => Some(Sport::Squash),
            Some(FitSubSport::Badminton) => Some(Sport::Badminton),
            Some(FitSubSport::Racquetball) => Some(Sport::Racquetball),
            Some(FitSubSport::TableTennis) => Some(Sport::TableTennis),
            _ => None,
        } {
            return sport;
        }

        match value.0 {
            FitSport::Running => Self::Running,
            FitSport::Cycling => Self::Cycling,
            FitSport::FitnessEquipment => Self::CardioTraining,
            FitSport::Swimming => Self::Swimming,
            FitSport::Basketball => Self::Basketball,
            FitSport::Soccer => Self::Soccer,
            FitSport::Cricket => Self::Cricket,
            FitSport::Rugby => Self::Rugby,
            FitSport::Hockey => Self::Hockey,
            FitSport::Lacrosse => Self::Lacrosse,
            FitSport::Volleyball => Self::Volleyball,
            FitSport::Baseball => Self::Baseball,
            FitSport::AmericanFootball => Self::AmericanFootball,
            FitSport::Tennis => Self::Tennis,
            FitSport::Training => Self::CardioTraining,
            FitSport::Hiit => Self::CardioTraining,
            FitSport::Walking => Self::Walking,
            FitSport::Hiking => Self::Hiking,
            FitSport::AlpineSkiing => Self::AlpineSki,
            FitSport::Snowboarding => Self::Snowboarding,
            FitSport::CrossCountrySkiing => Self::CrossCountrySkiing,
            FitSport::Rowing => Self::Rowing,
            FitSport::Mountaineering => Self::Mountaineering,
            FitSport::EBiking => Self::EBiking,
            FitSport::Paddling => Self::Paddling,
            FitSport::RockClimbing => Self::Climbing,
            FitSport::Sailing => Self::Sailing,
            FitSport::Snowshoeing => Self::Snowshoeing,
            FitSport::StandUpPaddleboarding => Self::StandUpPaddleboarding,
            FitSport::Surfing => Self::Surfing,
            FitSport::Wakeboarding => Self::Wakeboarding,
            FitSport::WaterSkiing => Self::WaterSkiing,
            FitSport::Kayaking => Self::Kayaking,
            FitSport::Rafting => Self::Rafting,
            FitSport::Windsurfing => Self::Windsurfing,
            FitSport::Kitesurfing => Self::Kitesurfing,
            FitSport::FloorClimbing => Self::IndoorClimbing,
            FitSport::Racket => Self::Racket,
            FitSport::Wakesurfing => Self::Wakesurfing,
            FitSport::InlineSkating => Self::InlineSkating,
            FitSport::JumpRope => Self::CardioTraining,
            FitSport::Jumpmaster => Self::CardioTraining,
            FitSport::Golf => Self::Golf,
            FitSport::Boxing => Self::Boxing,
            FitSport::MixedMartialArts => Self::MixedMartialArts,
            FitSport::WaterTubing => Self::Whitewater,
            FitSport::Dance => Self::CardioTraining,
            FitSport::Snorkeling => Self::Snorkeling,

            FitSport::Generic => Self::Other,
            FitSport::Transition => Self::Other,
            FitSport::Multisport => Self::Other,
            FitSport::Flying => Self::Other,
            FitSport::Motorcycling => Self::Other,
            FitSport::Boating => Self::Other,
            FitSport::Driving => Self::Other,
            FitSport::HangGliding => Self::Other,
            FitSport::HorsebackRiding => Self::Other,
            FitSport::Hunting => Self::Other,
            FitSport::Fishing => Self::Other,
            FitSport::IceSkating => Self::Other,
            FitSport::Snowmobiling => Self::Other,
            FitSport::SkyDiving => Self::Other,
            FitSport::Tactical => Self::Other,
            FitSport::Diving => Self::Other,
            FitSport::WheelchairPushRun => Self::Other,
            FitSport::WheelchairPushWalk => Self::Other,
            FitSport::Meditation => Self::Other,
            FitSport::DiscGolf => Self::Other,
            FitSport::All => Self::Other,
            FitSport::UnknownVariant(_) => Self::Other,
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
        let content = fs::read("src/inbound/parser/test.fit").unwrap();

        let res = try_fit_bytes_into_domain(content).unwrap();

        // Check for same point in time
        let expected_time = "2025-10-11T11:43:33+02:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        assert_eq!(*res.start_time().date(), expected_time);

        // Check for correct offset to UTC
        assert_eq!(
            (*res.start_time().date()).to_rfc3339(),
            "2025-10-11T11:43:33+02:00".to_string()
        );
    }

    #[test]
    fn test_parsing_of_timeseries() {
        let content = fs::read("src/inbound/parser/test.fit").unwrap();

        let res = try_fit_bytes_into_domain(content).unwrap();

        assert_eq!(res.timeseries().time().len(), 3901);

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
            Some(TimeseriesValue::Float(7.0))
        );
        match speed.unwrap().get(3).unwrap().as_ref().unwrap() {
            TimeseriesValue::Float(val) => assert_approx_eq!(val, 3.831),
            _ => unreachable!("Should be a float"),
        }
        assert_eq!(
            power.unwrap().get(3).unwrap().as_ref().unwrap(),
            &TimeseriesValue::Int(140)
        );
        assert_eq!(
            heart_rate.unwrap().get(3).unwrap().as_ref().unwrap(),
            &TimeseriesValue::Int(109)
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

    #[test]
    fn test_takes_pauses_into_account() {
        let content = fs::read("src/inbound/parser/test.fit").unwrap();

        let res = try_fit_bytes_into_domain(content).unwrap();

        // Last active time should be different from last absolute time
        assert_ne!(
            res.timeseries
                .active_time()
                .values()
                .iter()
                .rev()
                .next()
                .map(|val| val.value().unwrap()),
            res.timeseries.time().values().iter().rev().next().cloned()
        );
    }

    #[test]
    fn test_parse_timeseries_laps() {
        let content = fs::read("src/inbound/parser/test.fit").unwrap();

        let res = try_fit_bytes_into_domain(content).unwrap();

        assert_eq!(
            res.timeseries.laps(),
            &vec![
                Lap::new(0, 300),
                Lap::new(300, 1763),
                Lap::new(1763, 2063),
                Lap::new(2063, 2243),
                Lap::new(2243, 2543),
                Lap::new(2543, 2723),
                Lap::new(2723, 3023),
                Lap::new(3023, 3203),
                Lap::new(3203, 3503),
                Lap::new(3503, 3683),
                Lap::new(3683, 3983),
            ]
        );
    }
}
