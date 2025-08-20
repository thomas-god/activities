use std::collections::HashMap;

use thiserror::Error;

use crate::parser::{
    definition::{CustomDescription, Definition, parse_definition_message},
    types::{DataField, DataTypeError, DataValue},
};

#[derive(Error, Debug)]
pub enum RecordError {
    #[error("Record cannot be parsed")]
    InvalidRecord,
    #[error("No DefinitionMessage found for local id {0}")]
    NoDefinitionMessageFound(u8),
    #[error("Invalid DataType")]
    DataTypeError(#[from] DataTypeError),
}

#[derive(Debug)]
pub enum Record {
    Definition(Definition),
    Data(DataMessage),
    CompressedTimestamp(CompressedTimestampMessage),
}

#[derive(Debug)]
pub struct DataMessage {
    pub local_message_type: u8,
    pub values: Vec<DataMessageField>,
}

#[derive(Debug)]
pub struct DataMessageField {
    pub field: DataField,
    pub values: Vec<DataValue>,
}

#[derive(Debug)]
pub struct CompressedTimestampMessage {
    pub local_message_type: u8,
    pub values: Vec<u8>,
}

impl Record {
    pub fn parse<I>(
        content: &mut I,
        definitions: &HashMap<u8, Definition>,
        custom_descriptions: &HashMap<u8, HashMap<u8, Vec<CustomDescription>>>,
    ) -> Result<Self, RecordError>
    where
        I: Iterator<Item = u8>,
    {
        let header = RecordHeader::from_byte(content.next().ok_or(RecordError::InvalidRecord)?);

        match header {
            RecordHeader::Data(header) => {
                parse_data_message(header, definitions, custom_descriptions, content)
                    .map(Record::Data)
            }

            RecordHeader::Definition(header) => {
                parse_definition_message(header, content).map(Record::Definition)
            }

            RecordHeader::Compressed(header) => {
                parse_compressed_message(header, definitions, content)
            }
        }
    }
}

fn parse_data_message<I>(
    header: DataMessageHeader,
    definitions: &HashMap<u8, Definition>,
    custom_descriptions: &HashMap<u8, HashMap<u8, Vec<CustomDescription>>>,
    content: &mut I,
) -> Result<DataMessage, RecordError>
where
    I: Iterator<Item = u8>,
{
    match definitions.get(&header.local_message_type) {
        Some(definition) => {
            let mut values = Vec::new();
            for field in definition.fields.iter() {
                values.push(DataMessageField {
                    field: field.field.clone(),
                    values: field.field_type.parse_values(
                        content,
                        &field.endianness,
                        field.size,
                    )?,
                })
            }

            Ok(DataMessage {
                local_message_type: header.local_message_type,
                values,
            })
        }

        None => Err(RecordError::NoDefinitionMessageFound(
            header.local_message_type,
        )),
    }
}

fn parse_compressed_message<I>(
    header: CompressedMessageHeader,
    definitions: &HashMap<u8, Definition>,
    content: &mut I,
) -> Result<Record, RecordError>
where
    I: Iterator<Item = u8>,
{
    let fields_size = match definitions.get(&header.local_message_type) {
        Some(definition) => definition.fields_size,
        None => {
            return Err(RecordError::NoDefinitionMessageFound(
                header.local_message_type,
            ));
        }
    };
    let mut values: Vec<u8> = Vec::new();
    for _ in 0..fields_size {
        values.push(content.next().ok_or(RecordError::InvalidRecord)?);
    }
    Ok(Record::CompressedTimestamp(CompressedTimestampMessage {
        local_message_type: header.local_message_type,
        values,
    }))
}

enum RecordHeader {
    Definition(DefinitionMessageHeader),
    Data(DataMessageHeader),
    Compressed(CompressedMessageHeader),
}

impl RecordHeader {
    fn from_byte(byte: u8) -> RecordHeader {
        let normal = (byte >> 7) & 1 == 0;
        let data = (byte >> 6) & 1 == 0;
        match (normal, data) {
            (true, false) => {
                let message_type_specific = ((byte >> 5) & 1) == 1;
                let local_message_type = byte & 0b1111;
                RecordHeader::Definition(DefinitionMessageHeader {
                    message_type_specific,
                    local_message_type,
                })
            }
            (true, true) => {
                let message_type_specific = ((byte >> 5) & 1) == 1;
                let local_message_type = byte & 0b1111;
                RecordHeader::Data(DataMessageHeader {
                    message_type_specific,
                    local_message_type,
                })
            }
            (false, _) => RecordHeader::Compressed(CompressedMessageHeader {
                local_message_type: (byte & 0b1111),
                time_offset: 0,
            }),
        }
    }
}
pub struct DefinitionMessageHeader {
    pub message_type_specific: bool,
    pub local_message_type: u8,
}

pub struct DataMessageHeader {
    pub message_type_specific: bool,
    pub local_message_type: u8,
}

struct CompressedMessageHeader {
    local_message_type: u8,
    time_offset: u8,
}
