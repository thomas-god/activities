use std::collections::HashMap;

use crate::{
    BaseDataType,
    parser::{
        reader::Reader,
        records::{DefinitionMessageHeader, RecordError},
        types::global_messages::{CustomField, DataField, GlobalMessage},
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub field_type: BaseDataType,
    pub size: u8,
}

#[derive(Debug, Clone)]
pub struct CustomDescription {
    pub endianness: Endianness,
    pub base_type: BaseDataType,
    pub name: Option<String>,
    pub units: Option<String>,
}

pub fn parse_definition_message(
    header: DefinitionMessageHeader,
    custom_descriptions: &HashMap<u8, HashMap<u8, CustomDescription>>,
    content: &mut Reader,
) -> Result<Definition, RecordError> {
    let _reserved = content.next_u8()?;
    let endianness = Endianness::from(content.next_u8()?);
    let global_message_number = content.next_u16(&endianness)?;
    let message_type = GlobalMessage::from(global_message_number);

    let number_of_fields = content.next_u8()?;
    let mut fields_size = 0;
    let mut fields: Vec<DefinitionField> = vec![];

    println!("first 4 bytes read");

    for _ in 0..number_of_fields {
        println!("reading field");
        let field = parse_definition_field(&message_type, endianness, content)?;
        fields_size += field.size;
        fields.push(field);
    }

    println!("fields done");

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

fn parse_developer_field(
    custom_descriptions: &HashMap<u8, HashMap<u8, CustomDescription>>,
    endianness: Endianness,
    content: &mut Reader,
) -> Result<DefinitionField, RecordError> {
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

fn parse_definition_field(
    message_type: &GlobalMessage,
    endianness: Endianness,
    content: &mut Reader,
) -> Result<DefinitionField, RecordError> {
    let definition_number = content.next_u8()?;
    println!("definition");
    let kind = message_type.parse_field_kind(definition_number);
    let size = content.next_u8()?;
    println!("size");
    let field_type = BaseDataType::from_base_type_field(content.next_u8()?)?;

    println!("tyupe");
    Ok(DefinitionField {
        endianness,
        kind,
        size,
        field_type,
    })
}

#[cfg(test)]
mod tests {
    use crate::parser::types::global_messages::RecordField;

    use super::*;

    #[test]
    fn test_parse_definition() {
        let mut content = Reader::new(8, 0, vec![0, 0, 20, 0, 1, 3, 1, 2].into_iter());
        let definition = parse_definition_message(
            DefinitionMessageHeader {
                message_type_specific: false,
                local_message_type: 0,
            },
            &HashMap::new(),
            &mut content,
        )
        .unwrap();

        assert_eq!(definition.local_message_type, 0);
        assert_eq!(definition.message_type, GlobalMessage::Record);
        assert_eq!(definition.fields_size, 1);
        assert_eq!(definition.fields.len(), 1);

        let field = definition.fields.first().unwrap();
        assert_eq!(field.endianness, Endianness::Little);
        assert_eq!(field.field_type, BaseDataType::Uint8);
        assert_eq!(field.kind, DataField::Record(RecordField::HeartRate));
    }
}
