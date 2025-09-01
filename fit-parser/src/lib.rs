mod parser;
pub mod utils;

pub use crate::parser::types::DataValue;
pub use crate::parser::types::generated::*;
pub use crate::parser::{DataMessage, DataMessageField, FitParserError, parse_fit_messages};
