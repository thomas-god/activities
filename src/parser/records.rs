use std::collections::HashMap;

use thiserror::Error;

use crate::parser::{
    definition::{RecordDefinition, parse_definition_message},
    types::{DataField, DataType, DataTypeError, DataValue, record::RecordField},
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
    Definition(RecordDefinition),
    Data(DataMessage),
    CompressedTimestamp(CompressedTimestampMessage),
}

#[derive(Debug)]
pub enum DataMessage {
    Record(RecordDataMessage),
    Raw(RawDataMessage),
}

#[derive(Debug)]
pub struct RecordDataMessage {
    pub local_message_type: u8,
    pub values: Vec<RecordDataMessageField>,
}

#[derive(Debug)]
pub struct RecordDataMessageField {
    pub field: DataField,
    pub values: Vec<DataValue>,
}

#[derive(Debug)]
pub struct RawDataMessage {
    pub local_message_type: u8,
    pub values: Vec<u8>,
}
#[derive(Debug)]
pub struct CompressedTimestampMessage {
    pub local_message_type: u8,
    pub values: Vec<u8>,
}

impl Record {
    pub fn parse<I>(
        content: &mut I,
        definitions: &mut HashMap<u8, RecordDefinition>,
    ) -> Result<Self, RecordError>
    where
        I: Iterator<Item = u8>,
    {
        let record =
            match RecordHeader::from_byte(content.next().ok_or(RecordError::InvalidRecord)?) {
                RecordHeader::Normal(header) => parse_normal_message(header, definitions, content),
                RecordHeader::Compressed(header) => {
                    parse_compressed_message(header, definitions, content)
                }
            };

        if let Ok(Record::Definition(ref definition)) = record {
            definitions.insert(definition.local_message_type, definition.clone());
        }

        record
    }
}

fn parse_normal_message<I>(
    header: NormalRecordHeader,
    definitions: &HashMap<u8, RecordDefinition>,
    content: &mut I,
) -> Result<Record, RecordError>
where
    I: Iterator<Item = u8>,
{
    match header.message_type {
        MessageType::Definition => {
            parse_definition_message(header, content).map(Record::Definition)
        }
        MessageType::Data => parse_data_message(header, definitions, content).map(Record::Data),
    }
}

fn parse_data_message<I>(
    header: NormalRecordHeader,
    definitions: &HashMap<u8, RecordDefinition>,
    content: &mut I,
) -> Result<DataMessage, RecordError>
where
    I: Iterator<Item = u8>,
{
    match definitions.get(&header.local_message_type) {
        Some(definition) => {
            let mut values = Vec::new();
            for field in definition.fields.iter() {
                values.push(RecordDataMessageField {
                    field: field.field,
                    values: field.field_type.parse_values(content, field.size)?,
                })
            }

            Ok(DataMessage::Record(RecordDataMessage {
                local_message_type: header.local_message_type,

                values,
            }))
        }

        None => {
            return Err(RecordError::NoDefinitionMessageFound(
                header.local_message_type,
            ));
        }
    }
}

fn parse_compressed_message<I>(
    header: CompressedRecordHeader,
    definitions: &HashMap<u8, RecordDefinition>,
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
    Normal(NormalRecordHeader),
    Compressed(CompressedRecordHeader),
}

impl RecordHeader {
    fn from_byte(byte: u8) -> RecordHeader {
        match (byte >> 7) & 1 == 1 {
            false => Self::Normal(NormalRecordHeader {
                message_type: if ((byte >> 6) & 1) == 1 {
                    MessageType::Definition
                } else {
                    MessageType::Data
                },
                message_type_specific: ((byte >> 5) & 1) == 1,
                local_message_type: (byte & 0b1111),
            }),
            true => Self::Compressed(CompressedRecordHeader {
                local_message_type: (byte & 0b1111),
                time_offset: 0,
            }),
        }
    }
}
pub struct NormalRecordHeader {
    pub message_type: MessageType,
    pub message_type_specific: bool,
    pub local_message_type: u8,
}

enum MessageType {
    Definition,
    Data,
}

struct CompressedRecordHeader {
    local_message_type: u8,
    time_offset: u8,
}
