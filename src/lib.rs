mod activity;
mod parser;

pub use crate::activity::{Activity, ParseActivityError, parse_activity};
pub use crate::parser::types::{DataField, DataType, DataValue};
pub use crate::parser::{FitParserError, Record, parse_records};
