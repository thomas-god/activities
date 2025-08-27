use std::collections::HashMap;

use crate::parser::{
    definition::custom::CustomDescription,
    reader::Reader,
    records::{DefinitionMessageHeader, RecordError},
    types::{
        ScaleOffset,
        generated::{CustomField, FitMessage, MesgNum, ParseFunction},
    },
};

pub mod custom;

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
    pub message_type: MesgNum,
    pub local_message_type: u8,
    pub fields: Vec<DefinitionField>,
}

#[derive(Debug, Clone)]
pub struct DefinitionField {
    pub endianness: Endianness,
    pub kind: FitMessage,
    pub parse: ParseFunction,
    pub scale_offset: Option<ScaleOffset>,
    pub size: u8,
}

pub fn parse_definition_message(
    header: DefinitionMessageHeader,
    custom_descriptions: &HashMap<u8, HashMap<u8, CustomDescription>>,
    content: &mut Reader,
) -> Result<Definition, RecordError> {
    let _reserved = content.next_u8()?;
    let endianness = Endianness::from(content.next_u8()?);
    let message_type = MesgNum::from(content.next_u16(&endianness)?);

    let number_of_fields = content.next_u8()?;
    let mut fields: Vec<DefinitionField> = vec![];

    for _ in 0..number_of_fields {
        let field = parse_definition_field(&message_type, endianness, content)?;
        fields.push(field);
    }

    // Parse size of optionnal developer fields
    if header.message_type_specific {
        let number_developer_fields = content.next_u8()?;
        for _ in 0..number_developer_fields {
            let field = parse_developer_field(custom_descriptions, endianness, content)?;
            fields.push(field);
        }
    }

    Ok(Definition {
        message_type,
        local_message_type: header.local_message_type,
        fields,
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
        kind: FitMessage::Custom(CustomField {
            name: description.name.clone(),
            units: description.units.clone(),
        }),
        parse: ParseFunction::Simple(description.base_type.get_parse_fn()),
        scale_offset: None,
        size,
    };
    Ok(field)
}

fn parse_definition_field(
    message_type: &MesgNum,
    endianness: Endianness,
    content: &mut Reader,
) -> Result<DefinitionField, RecordError> {
    let definition_number = content.next_u8()?;
    let kind = message_type.message_field(definition_number);
    let parse = message_type.field_parse(definition_number);
    let scale_offset = message_type.scale_offset(definition_number);
    let size = content.next_u8()?;
    let _ = content.next_u8()?; // Byte for type is not used, but must still be consumed

    Ok(DefinitionField {
        endianness,
        kind,
        parse,
        scale_offset,
        size,
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        DataValue,
        parser::types::generated::{FitMessage, MesgNum, RecordField},
    };

    use super::*;

    #[test]
    fn test_parse_definition() {
        let mut content = Reader::new(8, vec![0, 0, 20, 0, 1, 3, 1, 2].into_iter());
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
        assert_eq!(definition.message_type, MesgNum::Record);
        assert_eq!(definition.fields.len(), 1);

        let field = definition.fields.first().unwrap();
        assert_eq!(field.endianness, Endianness::Little);
        assert_eq!(field.kind, FitMessage::Record(RecordField::HeartRate));

        let mut content = Reader::new(1, vec![12].into_iter());
        let ParseFunction::Simple(parse) = field.parse else {
            unreachable!("Should be a simple field");
        };
        assert_eq!(
            parse(&mut content, &Endianness::Little, 1).unwrap(),
            vec![DataValue::Uint8(12)]
        );
    }
}
