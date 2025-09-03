use fit_parser::{
    DataValue, FitEnum, FitParserError, Sport as FitSport, parse_fit_messages,
    utils::{find_field_value_as_float, find_field_value_by_kind},
};
use thiserror::Error;

use crate::domain::{
    models::{ActivityDuration, ActivityStartTime, Sport},
    ports::CreateActivityRequest,
};

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

        let duration = find_field_value_as_float(
            &messages,
            &fit_parser::FitField::Session(fit_parser::SessionField::TotalElapsedTime),
        )
        .map(|val| ActivityDuration(val.round() as usize))
        .ok_or(ParseCreateActivityHttpRequestBodyError::NoDurationFound)?;

        let start_time = find_field_value_by_kind(
            &messages,
            &fit_parser::FitField::Session(fit_parser::SessionField::StartTime),
        )
        .and_then(|values| {
            values.iter().find_map(|val| match val {
                DataValue::DateTime(dt) => ActivityStartTime::new(*dt as usize),
                _ => None,
            })
        })
        .ok_or(ParseCreateActivityHttpRequestBodyError::NoStartTimeFound)?;

        let sport = find_field_value_by_kind(
            &messages,
            &fit_parser::FitField::Session(fit_parser::SessionField::Sport),
        )
        .and_then(|field| {
            field.iter().find_map(|value| match value {
                fit_parser::DataValue::Enum(FitEnum::Sport(sport)) => Some(sport.into()),
                _ => None,
            })
        })
        .unwrap_or(Sport::Other);

        Ok(CreateActivityRequest::new(
            sport, duration, start_time, bytes,
        ))
    }
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
}
