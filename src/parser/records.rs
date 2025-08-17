use std::collections::HashMap;

use thiserror::Error;

use crate::parser::types::RecordField;

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
pub enum DefinitionMessage {
    RecordDefinition(RecordDefinition),
    UnsupportedDefinition(UnsupportedDefinition),
}

impl DefinitionMessage {
    fn local_message_type(&self) -> u8 {
        match self {
            Self::UnsupportedDefinition(definition) => definition.local_message_type,
            Self::RecordDefinition(definition) => definition.local_message_type,
        }
    }

    fn total_size(&self) -> u8 {
        match self {
            Self::UnsupportedDefinition(definition) => definition.fields_size,
            Self::RecordDefinition(defintion) => defintion.fields_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordDefinition {
    pub local_message_type: u8,
    pub fields: Vec<RecordDefinitionField>,
    pub fields_size: u8,
}

#[derive(Debug, Clone)]
pub struct RecordDefinitionField {
    field: RecordField,
    size: u8,
}

#[derive(Debug, Clone)]
pub struct UnsupportedDefinition {
    pub local_message_type: u8,
    pub fields_size: u8,
}

#[derive(Debug, Clone)]
pub struct DefinitionField {}

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
            definitions.insert(definition.local_message_type(), definition.clone());
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
            parse_definition_message(header, content).map(Record::Definition)
        }
        MessageType::Data => parse_data_message(header, definitions, content).map(Record::Data),
    }
}

fn parse_definition_message<I>(
    header: NormalRecordHeader,
    content: &mut I,
) -> Result<DefinitionMessage, RecordError>
where
    I: Iterator<Item = u8>,
{
    let _reserved = content.next().ok_or(RecordError::InvalidRecord)?;
    let _endianes = content.next().ok_or(RecordError::InvalidRecord)?;
    let global_message_number = u16::from_le_bytes([
        content.next().ok_or(RecordError::InvalidRecord)?,
        content.next().ok_or(RecordError::InvalidRecord)?,
    ]);

    match global_message_number {
        20 => parse_record_definition(header, content).map(DefinitionMessage::RecordDefinition),
        _ => parse_unsupported_definition(header, content)
            .map(DefinitionMessage::UnsupportedDefinition),
    }
}

fn parse_record_definition<I>(
    header: NormalRecordHeader,
    content: &mut I,
) -> Result<RecordDefinition, RecordError>
where
    I: Iterator<Item = u8>,
{
    let number_of_fields = content.next().ok_or(RecordError::InvalidRecord)?;
    let mut fields_size = 0;
    let mut fields: Vec<RecordDefinitionField> = vec![];

    for _ in 0..number_of_fields {
        let field = parse_definition_field(content)?;
        fields_size += field.size;
        fields.push(field);
    }

    // Parse size of optionnal developer fields
    if header.message_type_specific {
        let number_developer_fields = content.next().ok_or(RecordError::InvalidRecord)?;
        for _ in 0..number_developer_fields {
            let field = parse_definition_field(content)?;
            fields_size += field.size;
            fields.push(field);
        }
    }

    Ok(RecordDefinition {
        local_message_type: header.local_message_type,
        fields_size,
        fields,
    })
}

fn parse_unsupported_definition<I>(
    header: NormalRecordHeader,
    content: &mut I,
) -> Result<UnsupportedDefinition, RecordError>
where
    I: Iterator<Item = u8>,
{
    let number_of_fields = content.next().ok_or(RecordError::InvalidRecord)?;
    let mut fields_size = 0;

    for _ in 0..number_of_fields {
        fields_size += parse_definition_field_size(content)?;
    }

    // Parse size of optionnal developer fields
    if header.message_type_specific {
        let number_developer_fields = content.next().ok_or(RecordError::InvalidRecord)?;
        for _ in 0..number_developer_fields {
            fields_size += parse_definition_field_size(content)?;
        }
    }

    Ok(UnsupportedDefinition {
        local_message_type: header.local_message_type,
        fields_size,
    })
}

fn parse_definition_field<I>(content: &mut I) -> Result<RecordDefinitionField, RecordError>
where
    I: Iterator<Item = u8>,
{
    let definition_number = content.next().ok_or(RecordError::InvalidRecord)?;
    let field = RecordField::from(definition_number);
    let size = content.next().ok_or(RecordError::InvalidRecord)?;
    let _base_type = content.next().ok_or(RecordError::InvalidRecord)?;
    Ok(RecordDefinitionField { field, size })
}

fn parse_definition_field_size<I>(content: &mut I) -> Result<u8, RecordError>
where
    I: Iterator<Item = u8>,
{
    let _definition_number = content.next().ok_or(RecordError::InvalidRecord)?;
    let field_size = content.next().ok_or(RecordError::InvalidRecord)?;
    let _base_type = content.next().ok_or(RecordError::InvalidRecord)?;
    Ok(field_size)
}

fn parse_data_message<I>(
    header: NormalRecordHeader,
    definitions: &HashMap<u8, DefinitionMessage>,
    content: &mut I,
) -> Result<DataMessage, RecordError>
where
    I: Iterator<Item = u8>,
{
    let fields_size = match definitions.get(&header.local_message_type) {
        Some(definition) => definition.total_size(),
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
    let fields_size = match definitions.get(&header.local_message_type) {
        Some(definition) => definition.total_size(),
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
