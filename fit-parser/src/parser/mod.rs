use std::{collections::HashMap, fs};

use thiserror::Error;

use crate::parser::{
    definition::custom::{CustomDescription, parse_custom_definition_description},
    header::{FileHeader, FileHeaderError, HEADER_SIZE_WITH_CRC},
    reader::{Reader, ReaderError},
    records::{CompressedTimestamp, RecordError},
};

pub use crate::parser::definition::{Definition, Endianness};
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

    #[error("Invalid body CRC: expected {0} but got {1}")]
    InvalidBodyCRC(u16, u16),

    #[error("Reader error")]
    ReaderError(#[from] ReaderError),

    #[error("Body parsing error")]
    ParserError(#[from] RecordError),
}

pub fn parse_fit_messages(
    content: std::vec::IntoIter<u8>,
) -> Result<Vec<DataMessage>, FitParserError> {
    let mut header_reader = Reader::new(HEADER_SIZE_WITH_CRC as u32, content);
    let header = FileHeader::from_bytes(&mut header_reader)?;

    let mut reader = Reader::new(header.data_size, header_reader.remaining_content());

    let mut definitions: HashMap<u8, Definition> = HashMap::new();
    let mut custom_descriptions: HashMap<u8, HashMap<u8, CustomDescription>> = HashMap::new();
    let mut compressed_timestamp = CompressedTimestamp::default();
    let mut messages = Vec::new();

    loop {
        if reader.is_empty() {
            break;
        }

        let record = Record::parse(
            &mut reader,
            &definitions,
            &custom_descriptions,
            &mut compressed_timestamp,
        )?;

        match record {
            Record::Definition(ref definition) => {
                definitions.insert(definition.local_message_type, definition.clone());
            }
            Record::Data(data) => {
                parse_custom_definition_description(&data, &definitions, &mut custom_descriptions);
                compressed_timestamp.set_last_timestamp(data.last_timestamp());
                messages.push(data);
            }
        }

        if reader.is_empty() {
            break;
        }
    }

    let body_crc = reader.current_crc();

    let mut crc_reader = Reader::new(2, reader.remaining_content());
    let expected_crc = crc_reader.next_u16(&Endianness::Little)?;

    if body_crc != expected_crc {
        return Err(FitParserError::InvalidBodyCRC(expected_crc, body_crc));
    }

    Ok(messages)
}

pub fn parse_fit_file(file: &str) -> Result<Vec<DataMessage>, FitParserError> {
    let content = fs::read(file)?.into_iter();
    parse_fit_messages(content)
}

#[cfg(test)]
mod tests {

    use crate::parser::parse_fit_file;

    #[test]
    fn test_no_error() {
        let _ = parse_fit_file("test.fit");
    }
}
