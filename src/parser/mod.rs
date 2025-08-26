use std::{collections::HashMap, fs};

use thiserror::Error;

use crate::parser::{
    definition::custom::{CustomDescription, parse_custom_definition_description},
    header::{FileHeader, FileHeaderError},
    reader::Reader,
    records::CompressedTimestamp,
};

pub use crate::parser::definition::{Definition, DefinitionField, Endianness};
pub use crate::parser::records::Record;
pub use crate::parser::records::{DataMessage, DataMessageField};

mod definition;
mod header;
mod reader;
mod records;
pub mod types;

#[derive(Error, Debug)]
pub enum FitParserError {
    #[error("Header parsing failed: {0}")]
    Header(#[from] FileHeaderError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn parse_records(file: &str) -> Result<Vec<Record>, FitParserError> {
    let mut content = fs::read(file)?.into_iter();

    let header = FileHeader::from_bytes(&mut content)?;
    let mut reader = Reader::new(header.data_size, header.crc, content);

    let mut definitions: HashMap<u8, Definition> = HashMap::new();
    let mut custom_descriptions: HashMap<u8, HashMap<u8, CustomDescription>> = HashMap::new();
    let mut compressed_timestamp = CompressedTimestamp::default();
    let mut records = Vec::new();

    loop {
        let record = match Record::parse(
            &mut reader,
            &definitions,
            &custom_descriptions,
            &mut compressed_timestamp,
        ) {
            Ok(record) => record,
            Err(err) => {
                println!("error: {:?}", err);
                break;
            }
        };
        match record {
            Record::Definition(ref definition) => {
                definitions.insert(definition.local_message_type, definition.clone());
            }
            Record::Data(ref data) => {
                parse_custom_definition_description(data, &definitions, &mut custom_descriptions);
                compressed_timestamp.set_last_timestamp(data.last_timestamp());
            }
        }
        records.push(record);

        if reader.is_empty() {
            break;
        }
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_records;

    #[test]
    fn test_no_error() {
        let _ = parse_records("test.fit");
    }
}
