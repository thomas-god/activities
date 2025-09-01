use fit_parser::{DataMessage, parse_fit_messages};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct Activity {
    content: FitContent,
}

impl Activity {
    pub fn new(content: FitContent) -> Self {
        Self { content }
    }

    pub fn fit_content(&self) -> &[DataMessage] {
        &self.content.messages
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FitContent {
    raw: Vec<u8>,
    messages: Vec<DataMessage>,
}

#[derive(Debug, Clone, Error)]
#[error("The content is not a valid FIT content")]
pub struct InvalidFitContentError;

impl FitContent {
    pub fn new(bytes: Vec<u8>) -> Result<Self, InvalidFitContentError> {
        match parse_fit_messages(bytes.clone().into_iter()) {
            Err(_) => Err(InvalidFitContentError),
            Ok(messages) => Ok(Self {
                raw: bytes,
                messages,
            }),
        }
    }
}
