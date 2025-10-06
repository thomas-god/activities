use derive_more::Constructor;
use thiserror::Error;

use crate::domain::{
    models::{
        UserId,
        activity::{ActivityStartTime, ActivityStatistics, ActivityTimeseries, Sport},
    },
    ports::CreateActivityRequest,
};

pub mod fit;
pub mod tcx;

pub use fit::FitParser;
pub use tcx::TcxParser;

#[derive(Debug, Clone, Constructor)]
pub struct ParsedFileContent {
    sport: Sport,
    start_time: ActivityStartTime,
    statistics: ActivityStatistics,
    timeseries: ActivityTimeseries,
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
            self.raw_content,
        )
    }
}

#[derive(Debug, Clone, Error)]
pub enum ParseBytesError {
    #[error("Error when parsing FIT content")]
    InvalidFitContent,
    #[error("No start time data found in activity file")]
    NoStartTimeFound,
    #[error("No activity duration data found in activity file")]
    NoDurationFound,
}

pub trait ParseFile: Clone + Send + Sync + 'static {
    fn try_bytes_into_domain(&self, bytes: Vec<u8>) -> Result<ParsedFileContent, ParseBytesError>;
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
                bytes: Vec<u8>,
            ) -> Result<ParsedFileContent, ParseBytesError>;
        }

    }

    impl MockFileParser {
        pub fn test_default() -> Self {
            let mut mock = Self::new();
            mock.expect_try_bytes_into_domain().returning(|_| {
                Ok(ParsedFileContent::new(
                    Sport::Cycling,
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityStatistics::default(),
                    ActivityTimeseries::default(),
                    vec![1, 2, 3],
                ))
            });
            mock
        }
    }
}
