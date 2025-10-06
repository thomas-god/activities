use derive_more::Constructor;
use thiserror::Error;

use crate::{
    domain::{
        models::{
            UserId,
            activity::{ActivityStartTime, ActivityStatistics, ActivityTimeseries, Sport},
        },
        ports::{CreateActivityRequest, RawContent},
    },
    inbound::parser::fit::try_fit_bytes_into_domain,
};

pub mod fit;
pub mod tcx;

#[derive(Debug, Clone, Constructor)]
pub struct ParsedFileContent {
    sport: Sport,
    start_time: ActivityStartTime,
    statistics: ActivityStatistics,
    timeseries: ActivityTimeseries,
    extension: String,
    raw_content: Vec<u8>,
}

impl ParsedFileContent {
    pub fn start_time(&self) -> &ActivityStartTime {
        &self.start_time
    }

    pub fn raw_content(&self) -> &[u8] {
        &self.raw_content
    }

    pub fn sport(&self) -> &Sport {
        &self.sport
    }

    pub fn statistics(&self) -> &ActivityStatistics {
        &self.statistics
    }

    pub fn timeseries(&self) -> &ActivityTimeseries {
        &self.timeseries
    }

    pub fn into_request(self, user: &UserId) -> CreateActivityRequest {
        CreateActivityRequest::new(
            user.clone(),
            self.sport,
            self.start_time,
            self.statistics,
            self.timeseries,
            RawContent::new(self.extension, self.raw_content),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SupportedExtension {
    FIT,
    TCX,
}

impl SupportedExtension {
    pub fn suffix(&self) -> &str {
        match self {
            Self::FIT => "fit",
            Self::TCX => "tcx",
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq)]
pub enum ParseBytesError {
    #[error("File content is not a valid activity file")]
    InvalidContent,
    #[error("No start time data found in activity file")]
    NoStartTimeFound,
}

pub trait ParseFile: Clone + Send + Sync + 'static {
    fn try_bytes_into_domain(
        &self,
        extention: &SupportedExtension,
        bytes: Vec<u8>,
    ) -> Result<ParsedFileContent, ParseBytesError>;
}

#[derive(Clone)]
pub struct Parser {}

impl ParseFile for Parser {
    fn try_bytes_into_domain(
        &self,
        extension: &SupportedExtension,
        bytes: Vec<u8>,
    ) -> Result<ParsedFileContent, ParseBytesError> {
        match extension {
            SupportedExtension::FIT => try_fit_bytes_into_domain(bytes),
            SupportedExtension::TCX => todo!(),
        }
    }
}

#[cfg(test)]
pub mod test_utils {

    use mockall::mock;

    use super::*;

    mock! {
        pub FileParser {}

        impl Clone for  FileParser {
            fn clone(&self) -> Self;
        }

        impl ParseFile for FileParser {
            fn try_bytes_into_domain(
                &self,
                extention: &SupportedExtension,
                bytes: Vec<u8>,
            ) -> Result<ParsedFileContent, ParseBytesError>;
        }

    }

    impl MockFileParser {
        pub fn test_default() -> Self {
            let mut mock = Self::new();
            mock.expect_try_bytes_into_domain().returning(|_, __| {
                Ok(ParsedFileContent::new(
                    Sport::Cycling,
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityStatistics::default(),
                    ActivityTimeseries::default(),
                    "fit".to_string(),
                    vec![1, 2, 3],
                ))
            });
            mock
        }
    }
}
