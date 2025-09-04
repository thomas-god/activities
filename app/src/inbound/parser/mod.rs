use chrono::{DateTime, FixedOffset};
use fit_parser::{
    DataMessage, DataValue, FitEnum, FitParserError, Sport as FitSport, parse_fit_messages,
    utils::{find_field_value_as_float, find_field_value_by_kind},
};
use thiserror::Error;

use crate::domain::{
    models::{ActivityDuration, ActivityStartTime, Sport},
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

        let start_time = extract_start_time(&messages)
            .ok_or(ParseCreateActivityHttpRequestBodyError::NoStartTimeFound)?;

        let sport = extract_sport(&messages);

        Ok(CreateActivityRequest::new(
            sport, duration, start_time, bytes,
        ))
    }
}

fn extract_start_time(messages: &[DataMessage]) -> Option<ActivityStartTime> {
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
    let start_datetime =
        start_datetime.with_timezone(&FixedOffset::east_opt(offset as i32).unwrap());

    Some(ActivityStartTime::new(start_datetime))
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
                    vec![1, 2, 3],
                )))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{DateTime, FixedOffset, Utc};

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
}
