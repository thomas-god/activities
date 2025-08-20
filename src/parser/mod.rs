use std::{collections::HashMap, fs};

use thiserror::Error;

pub use crate::parser::records::Record;
use crate::parser::{
    definition::Definition,
    header::{FileHeader, FileHeaderError},
    records::DataMessage,
};

mod definition;
mod header;
mod records;
pub mod types;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Header parsing failed: {0}")]
    Header(#[from] FileHeaderError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn parse_file(file: &str) -> Result<Vec<DataMessage>, ParseError> {
    let mut content = fs::read(file)?.into_iter();

    let _header = FileHeader::from_bytes(&mut content);

    let mut definitions: HashMap<u8, Definition> = HashMap::new();
    let mut messages = Vec::new();

    loop {
        match Record::parse(&mut content, &definitions) {
            Ok(Record::Definition(definition)) => {
                definitions.insert(definition.local_message_type, definition.clone());
                println!("{:?}", definition);
            }
            Ok(Record::Data(data)) => {
                println!("{:?}", data);
                messages.push(data);
            }
            Ok(Record::CompressedTimestamp(data)) => {
                println!("{:?}", data)
            }
            Err(err) => {
                println!("error: {err:?}");
                break;
            }
        }
    }

    println!("Parsed {:?} messages", messages.len());

    Ok(messages)
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_file;

    #[test]
    fn test_no_error() {
        let _ = parse_file("test.fit");
    }
}
