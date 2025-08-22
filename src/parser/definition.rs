use std::collections::HashMap;

use crate::{
    DataType,
    parser::{
        reader::Reader,
        records::{DefinitionMessageHeader, RecordError},
        types::global_messages::{CustomField, DataField, GlobalMessage},
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
    pub kind: DataField,
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

pub fn parse_definition_message<I>(
    header: DefinitionMessageHeader,
    custom_descriptions: &HashMap<u8, HashMap<u8, CustomDescription>>,
    content: &mut Reader<I>,
) -> Result<Definition, RecordError>
where
    I: Iterator<Item = u8>,
{
    let _reserved = content.next_u8()?;
    let endianness = Endianness::from(content.next_u8()?);
    let global_message_number = content.next_u16(&endianness)?;
    let message_type = GlobalMessage::from(global_message_number);

    let number_of_fields = content.next_u8()?;
    let mut fields_size = 0;
    let mut fields: Vec<DefinitionField> = vec![];

    for _ in 0..number_of_fields {
        let field = parse_definition_field(&message_type, endianness, content)?;
        fields_size += field.size;
        fields.push(field);
    }

    // Parse size of optionnal developer fields
    if header.message_type_specific {
        let number_developer_fields = content.next_u8()?;
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
    content: &mut Reader<I>,
) -> Result<DefinitionField, RecordError>
where
    I: Iterator<Item = u8>,
{
    let field_number = content.next_u8()?;
    let size = content.next_u8()?;
    let developer_data_index = content.next_u8()?;
    let description = custom_descriptions
        .get(&developer_data_index)
        .and_then(|des| des.get(&field_number))
        .ok_or(RecordError::NoDescriptionFound(
            developer_data_index,
            field_number,
        ))?;
    let field = DefinitionField {
        endianness,
        kind: DataField::Custom(CustomField {
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
    content: &mut Reader<I>,
) -> Result<DefinitionField, RecordError>
where
    I: Iterator<Item = u8>,
{
    let definition_number = content.next_u8()?;
    let kind = message_type.parse_field_kind(definition_number);
    let size = content.next_u8()?;
    let field_type = DataType::from_base_type_field(content.next_u8()?)?;

    Ok(DefinitionField {
        endianness,
        kind,
        size,
        field_type,
    })
}
