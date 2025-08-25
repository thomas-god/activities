mod activity;
mod parser;

pub use crate::activity::{Activity, ParseActivityError};
pub use crate::parser::types::DataValue;
pub use crate::parser::types::generated::{FitEnum, MesgNum};
pub use crate::parser::{
    DataMessage, DataMessageField, Definition, DefinitionField, Endianness, FitParserError, Record,
    parse_records,
};
