mod parser;

pub use crate::parser::types::{DataField, DataType, DataValue};
pub use crate::parser::{ParseError, Record, parse_records};
