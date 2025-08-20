use std::collections::HashMap;

use crate::parser::{
    records::{DefinitionMessageHeader, RecordError},
    types::{
        CustomField, DataField, DataType, developer::DeveloperDataIdField,
        field_description::FieldDescriptionField, file_id::FileIdField, record::RecordField,
    },
};

#[derive(Debug, Clone, Copy)]
pub enum Endianness {
    Little,
    Big,
}

impl From<u8> for Endianness {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Little,
            _ => Self::Big,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub message_type: GlobalMessage,
    pub local_message_type: u8,
    pub fields: Vec<DefinitionField>,
    pub fields_size: u8,
}

#[derive(Debug, Clone)]
pub struct DefinitionField {
    pub endianness: Endianness,
    pub field: DataField,
    pub field_type: DataType,
    pub size: u8,
}

#[derive(Debug, Clone)]
pub struct CustomDescription {
    pub endianness: Endianness,
    pub base_type: DataType,
    pub name: Option<String>,
    pub units: Option<String>,
}

#[derive(Debug, Clone)]
pub enum GlobalMessage {
    FileId,
    Record,
    FieldDescription,
    DeveloperDataId,
    Unsupported(u16),
}

impl GlobalMessage {
    pub fn parse_field(&self, definition_number: u8) -> DataField {
        match self {
            GlobalMessage::Record => DataField::Record(RecordField::from(definition_number)),
            GlobalMessage::FileId => DataField::FileId(FileIdField::from(definition_number)),
            GlobalMessage::FieldDescription => {
                DataField::FieldDescription(FieldDescriptionField::from(definition_number))
            }
            GlobalMessage::DeveloperDataId => {
                DataField::DeveloperDataId(DeveloperDataIdField::from(definition_number))
            }
            _ => DataField::Unknown,
        }
    }
}

impl From<u16> for GlobalMessage {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::FileId,
            20 => Self::Record,
            206 => Self::FieldDescription,
            207 => Self::DeveloperDataId,
            val => Self::Unsupported(val),
        }
    }
}

pub fn parse_definition_message<I>(
    header: DefinitionMessageHeader,
    custom_descriptions: &HashMap<u8, HashMap<u8, CustomDescription>>,
    content: &mut I,
) -> Result<Definition, RecordError>
where
    I: Iterator<Item = u8>,
{
    let _reserved = content.next().ok_or(RecordError::InvalidRecord)?;
    let endianness = Endianness::from(content.next().ok_or(RecordError::InvalidRecord)?);
    let global_message_number = match endianness {
        Endianness::Little => u16::from_le_bytes([
            content.next().ok_or(RecordError::InvalidRecord)?,
            content.next().ok_or(RecordError::InvalidRecord)?,
        ]),
        Endianness::Big => u16::from_be_bytes([
            content.next().ok_or(RecordError::InvalidRecord)?,
            content.next().ok_or(RecordError::InvalidRecord)?,
        ]),
    };
    let message_type = GlobalMessage::from(global_message_number);

    let number_of_fields = content.next().ok_or(RecordError::InvalidRecord)?;
    let mut fields_size = 0;
    let mut fields: Vec<DefinitionField> = vec![];

    for _ in 0..number_of_fields {
        let field = parse_definition_field(&message_type, endianness, content)?;
        fields_size += field.size;
        fields.push(field);
    }

    // Parse size of optionnal developer fields
    if header.message_type_specific {
        let number_developer_fields = content.next().ok_or(RecordError::InvalidRecord)?;
        for _ in 0..number_developer_fields {
            let field = parse_developer_field(custom_descriptions, endianness, content)?;
            fields_size += field.size;
            fields.push(field);
        }
    }

    Ok(Definition {
        message_type,
        local_message_type: header.local_message_type,
        fields,
        fields_size,
    })
}

fn parse_developer_field<I>(
    custom_descriptions: &HashMap<u8, HashMap<u8, CustomDescription>>,
    endianness: Endianness,
    content: &mut I,
) -> Result<DefinitionField, RecordError>
where
    I: Iterator<Item = u8>,
{
    let field_number = content.next().ok_or(RecordError::InvalidRecord)?;
    let size = content.next().ok_or(RecordError::InvalidRecord)?;
    let developer_data_index = content.next().ok_or(RecordError::InvalidRecord)?;
    let description = custom_descriptions
        .get(&developer_data_index)
        .and_then(|des| des.get(&field_number))
        .ok_or(RecordError::NoDescriptionFound(
            developer_data_index,
            field_number,
        ))?;
    let field = DefinitionField {
        endianness,
        field: DataField::Custom(CustomField {
            name: description.name.clone(),
            units: description.units.clone(),
        }),
        field_type: description.base_type,
        size,
    };
    Ok(field)
}

fn parse_definition_field<I>(
    message_type: &GlobalMessage,
    endianness: Endianness,
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
        endianness,
        field,
        size,
        field_type,
    })
}
