use std::{collections::HashMap, fs};

use thiserror::Error;

use crate::parser::{
    header::{FileHeader, FileHeaderError},
    records::{DefinitionMessage, Record},
};

mod header;
mod records;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Header parsing failed: {0}")]
    Header(#[from] FileHeaderError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn parse_file(file: &str) -> Result<usize, ParseError> {
    let mut content = fs::read(file)?.into_iter();

    let _header = FileHeader::from_bytes(&mut content);

    let mut definitions: HashMap<u8, DefinitionMessage> = HashMap::new();
    let mut messages: Vec<Record> = Vec::new();

    loop {
        match Record::parse(&mut content, &mut definitions) {
            Some(record) => {
                println!("{record:?}");
                messages.push(record);
            }
            None => break,
        }
    }

    println!("Parsed {:?} messages", messages.len());

    Ok(0)
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_file;

    #[test]
    fn test_no_error() {
        let _ = parse_file("test.fit");
    }
}
