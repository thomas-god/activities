use std::{collections::HashMap, mem::discriminant};

use thiserror::Error;

use crate::parser::{
    definition::{CustomDescription, Definition, parse_definition_message},
    types::{DataTypeError, DataValue, global_messages::DataField},
};

#[derive(Error, Debug)]
pub enum RecordError {
    #[error("Record cannot be parsed")]
    InvalidRecord,
    #[error("No DefinitionMessage found for local id {0}")]
    NoDefinitionMessageFound(u8),
    #[error("No description found for developer data index {0} and field number {1}")]
    NoDescriptionFound(u8, u8),
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

impl DataMessage {
    /// Extract the last (i.e. most recent) [u32] timestamp contains in all the fiels and values of a [DataMessage].
    pub fn last_timestamp(&self) -> Option<u32> {
        let mut last_timestamp: Option<u32> = None;
        for value in self.values.iter() {
            if discriminant(&value.field) == discriminant(&DataField::Timestamp) {
                last_timestamp =
                    value
                        .values
                        .iter()
                        .fold(last_timestamp, |last, value| match value {
                            DataValue::Uint32(val) if *val >= last_timestamp.unwrap_or(0) => {
                                Some(*val)
                            }
                            _ => last,
                        });
            }
        }
        last_timestamp
    }
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
        custom_descriptions: &HashMap<u8, HashMap<u8, CustomDescription>>,
    ) -> Result<Self, RecordError>
    where
        I: Iterator<Item = u8>,
    {
        let header = RecordHeader::from_byte(content.next().ok_or(RecordError::InvalidRecord)?);

        match header {
            RecordHeader::Data(header) => {
                parse_data_message(header, definitions, content).map(Record::Data)
            }

            RecordHeader::Definition(header) => {
                parse_definition_message(header, custom_descriptions, content)
                    .map(Record::Definition)
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_data_message_contains_u32_timestamp() {
        let message_w_timestamp = DataMessage {
            local_message_type: 0,
            values: vec![DataMessageField {
                field: DataField::Timestamp,
                values: vec![DataValue::Uint32(0)],
            }],
        };

        assert!(message_w_timestamp.last_timestamp().is_some());
        assert_eq!(message_w_timestamp.last_timestamp().unwrap(), 0);
    }

    #[test]
    fn test_data_message_contains_multiple_u32_timestamps() {
        let message_w_timestamp = DataMessage {
            local_message_type: 0,
            values: vec![DataMessageField {
                field: DataField::Timestamp,
                values: vec![DataValue::Uint32(0), DataValue::Uint32(3)],
            }],
        };

        assert!(message_w_timestamp.last_timestamp().is_some());
        assert_eq!(message_w_timestamp.last_timestamp().unwrap(), 3);
    }

    #[test]
    fn test_data_message_contains_multiple_fields_with_u32_timestamps() {
        let message_w_timestamp = DataMessage {
            local_message_type: 0,
            values: vec![
                DataMessageField {
                    field: DataField::Timestamp,
                    values: vec![DataValue::Uint32(16)],
                },
                DataMessageField {
                    field: DataField::Timestamp,
                    values: vec![DataValue::Uint32(0), DataValue::Uint32(3)],
                },
            ],
        };

        assert!(message_w_timestamp.last_timestamp().is_some());
        assert_eq!(message_w_timestamp.last_timestamp().unwrap(), 16);
    }

    #[test]
    fn test_data_message_contains_timestamp_but_not_u32() {
        let message_w_timestamp = DataMessage {
            local_message_type: 0,
            values: vec![DataMessageField {
                field: DataField::Timestamp,
                values: vec![DataValue::String("toto".to_string())],
            }],
        };

        assert!(message_w_timestamp.last_timestamp().is_none());
    }

    #[test]
    fn test_data_message_contains_no_timestamp() {
        let message_w_timestamp = DataMessage {
            local_message_type: 0,
            values: vec![DataMessageField {
                field: DataField::Unknown,
                values: vec![DataValue::String("toto".to_string())],
            }],
        };

        assert!(message_w_timestamp.last_timestamp().is_none());
    }
}
