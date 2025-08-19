use crate::parser::{
    records::{NormalRecordHeader, RecordError},
    types::{DataField, DataType, file_id::FileIdField, record::RecordField},
};

#[derive(Debug, Clone)]
pub struct RecordDefinition {
    pub message_type: GlobalMessage,
    pub local_message_type: u8,
    pub fields: Vec<DefinitionField>,
    pub fields_size: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct DefinitionField {
    pub field: DataField,
    pub field_type: DataType,
    pub size: u8,
}

#[derive(Debug, Clone)]
pub enum GlobalMessage {
    FileId,
    Record,
    Unsupported(u16),
}

impl GlobalMessage {
    pub fn parse_field(&self, definition_number: u8) -> DataField {
        match self {
            GlobalMessage::Record => DataField::Record(RecordField::from(definition_number)),
            GlobalMessage::FileId => DataField::FileId(FileIdField::from(definition_number)),
            _ => DataField::Unknown,
        }
    }
}

impl From<u16> for GlobalMessage {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::FileId,
            20 => Self::Record,
            val => Self::Unsupported(val),
        }
    }
}

pub fn parse_definition_message<I>(
    header: NormalRecordHeader,
    content: &mut I,
) -> Result<RecordDefinition, RecordError>
where
    I: Iterator<Item = u8>,
{
    let _reserved = content.next().ok_or(RecordError::InvalidRecord)?;
    let _endianes = content.next().ok_or(RecordError::InvalidRecord)?;
    let global_message_number = u16::from_le_bytes([
        content.next().ok_or(RecordError::InvalidRecord)?,
        content.next().ok_or(RecordError::InvalidRecord)?,
    ]);
    let message_type = GlobalMessage::from(global_message_number);

    let number_of_fields = content.next().ok_or(RecordError::InvalidRecord)?;
    let mut fields_size = 0;
    let mut fields: Vec<DefinitionField> = vec![];

    for _ in 0..number_of_fields {
        let field = parse_definition_field(&message_type, content)?;
        fields_size += field.size;
        fields.push(field);
    }

    // Parse size of optionnal developer fields
    if header.message_type_specific {
        let number_developer_fields = content.next().ok_or(RecordError::InvalidRecord)?;
        for _ in 0..number_developer_fields {
            let field = parse_definition_field(&message_type, content)?;
            fields_size += field.size;
            fields.push(field);
        }
    }

    Ok(RecordDefinition {
        message_type,
        local_message_type: header.local_message_type,
        fields,
        fields_size,
    })
}

fn parse_definition_field<I>(
    message_type: &GlobalMessage,
    content: &mut I,
) -> Result<DefinitionField, RecordError>
where
    I: Iterator<Item = u8>,
{
    let definition_number = content.next().ok_or(RecordError::InvalidRecord)?;
    let field = message_type.parse_field(definition_number);
    let size = content.next().ok_or(RecordError::InvalidRecord)?;
    let field_type =
        DataType::from_base_type_field(content.next().ok_or(RecordError::InvalidRecord)?)?;
    Ok(DefinitionField {
        field,
        size,
        field_type,
    })
}
