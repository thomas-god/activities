use std::collections::HashMap;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RecordError {
    #[error("Record cannot be parsed")]
    InvalidRecord,
    #[error("No DefinitionMessage found for local id {0}")]
    NoDefinitionMessageFound(u8),
}

#[derive(Debug)]
pub enum Record {
    Definition(DefinitionMessage),
    Data(DataMessage),
    CompressedTimestamp(CompressedTimestampMessage),
}

#[derive(Debug, Clone)]
pub struct DefinitionMessage {
    pub local_message_type: u8,
    pub fields_size: u8,
}
#[derive(Debug)]
pub struct DataMessage {
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
        definitions: &mut HashMap<u8, DefinitionMessage>,
    ) -> Option<Self>
    where
        I: Iterator<Item = u8>,
    {
        let record = match RecordHeader::from_byte(content.next()?) {
            RecordHeader::Normal(header) => parse_normal_message(header, definitions, content).ok(),
            RecordHeader::Compressed(header) => {
                parse_compressed_message(header, definitions, content).ok()
            }
        };

        match record {
            Some(Record::Definition(ref definition_record)) => {
                definitions.insert(
                    definition_record.local_message_type,
                    definition_record.clone(),
                );
            }
            _ => {}
        }

        record
    }
}

fn parse_normal_message<I>(
    header: NormalRecordHeader,
    definitions: &HashMap<u8, DefinitionMessage>,
    content: &mut I,
) -> Result<Record, RecordError>
where
    I: Iterator<Item = u8>,
{
    match header.message_type {
        MessageType::Definition => {
            return parse_definition_message(header, content).map(|msg| Record::Definition(msg));
        }
        MessageType::Data => {
            return parse_data_message(header, definitions, content).map(|msg| Record::Data(msg));
        }
    };
}

fn parse_definition_message<I>(
    header: NormalRecordHeader,
    content: &mut I,
) -> Result<DefinitionMessage, RecordError>
where
    I: Iterator<Item = u8>,
{
    // First 5 bytes are reserved
    let _reserved = content.next().ok_or(RecordError::InvalidRecord)?;
    let _endianes = content.next().ok_or(RecordError::InvalidRecord)?;
    let _global_message_number = u16::from_le_bytes([
        content.next().ok_or(RecordError::InvalidRecord)?,
        content.next().ok_or(RecordError::InvalidRecord)?,
    ]);
    let number_of_fields = content.next().ok_or(RecordError::InvalidRecord)?;

    // Parse all fields
    let mut fields_size = 0;
    for _ in 0..number_of_fields {
        fields_size += parse_definition_field(content)?;
    }

    // Parse optionnal developer fields
    if header.message_type_specific {
        let number_developer_fields = content.next().ok_or(RecordError::InvalidRecord)?;
        for _ in 0..number_developer_fields {
            fields_size += parse_definition_field(content)?;
        }
    }

    Ok(DefinitionMessage {
        local_message_type: header.local_message_type,
        fields_size,
    })
}

fn parse_definition_field<I>(content: &mut I) -> Result<u8, RecordError>
where
    I: Iterator<Item = u8>,
{
    let _definition_number = content.next().ok_or(RecordError::InvalidRecord)?;
    let field_size = content.next().ok_or(RecordError::InvalidRecord)?;
    let _base_type = content.next().ok_or(RecordError::InvalidRecord)?;
    return Ok(field_size);
}

fn parse_data_message<I>(
    header: NormalRecordHeader,
    definitions: &HashMap<u8, DefinitionMessage>,
    content: &mut I,
) -> Result<DataMessage, RecordError>
where
    I: Iterator<Item = u8>,
{
    let Some(DefinitionMessage { fields_size, .. }) = definitions.get(&header.local_message_type)
    else {
        return Err(RecordError::NoDefinitionMessageFound(
            header.local_message_type,
        ));
    };
    let mut values: Vec<u8> = Vec::new();
    for _ in 0..*fields_size {
        values.push(content.next().ok_or(RecordError::InvalidRecord)?);
    }
    Ok(DataMessage {
        local_message_type: header.local_message_type,
        values,
    })
}

fn parse_compressed_message<I>(
    header: CompressedRecordHeader,
    definitions: &HashMap<u8, DefinitionMessage>,
    content: &mut I,
) -> Result<Record, RecordError>
where
    I: Iterator<Item = u8>,
{
    let Some(DefinitionMessage { fields_size, .. }) = definitions.get(&header.local_message_type)
    else {
        return Err(RecordError::NoDefinitionMessageFound(
            header.local_message_type,
        ));
    };
    let mut values: Vec<u8> = Vec::new();
    for _ in 0..*fields_size {
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
                local_message_type: ((byte >> 0) & 0b1111),
            }),
            true => Self::Compressed(CompressedRecordHeader {
                local_message_type: ((byte >> 0) & 0b1111),
                time_offset: 0,
            }),
        }
    }
}
struct NormalRecordHeader {
    message_type: MessageType,
    message_type_specific: bool,
    local_message_type: u8,
}

enum MessageType {
    Definition,
    Data,
}

struct CompressedRecordHeader {
    local_message_type: u8,
    time_offset: u8,
}
