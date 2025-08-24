mod activity;
mod parser;

pub use crate::activity::{Activity, ParseActivityError, parse_activity};
pub use crate::parser::types::{BaseDataType, BaseDataValue, DataField};
pub use crate::parser::{FitParserError, Record, parse_records};
