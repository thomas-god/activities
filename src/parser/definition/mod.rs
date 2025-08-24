use std::collections::HashMap;

use crate::parser::{
    definition::custom::CustomDescription,
    reader::Reader,
    records::{DefinitionMessageHeader, RecordError},
    types::{
        DataTypeError,
        generated::{CustomField, DataValue, FitMessage, MesgNum},
        parse_byte_array, parse_float32, parse_float64, parse_sint8, parse_sint16, parse_sint32,
        parse_sint64, parse_string, parse_uint8, parse_uint8z, parse_uint16, parse_uint16z,
        parse_uint32, parse_uint32z, parse_uint64, parse_uint64z, parse_unknown,
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
    pub fields_size: u8,
}

#[derive(Debug, Clone)]
pub struct DefinitionField {
    pub endianness: Endianness,
    pub kind: FitMessage,
    pub parse: fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError>,
    pub size: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseDataType {
    Enum,
    Sint8,
    Uint8,
    Sint16,
    Uint16,
    Sint32,
    Uint32,
    String,
    Float32,
    Float64,
    Uint8z,
    Uint16z,
    Uint32z,
    Byte,
    Sint64,
    Uint64,
    Uint64z,
    Unknown,
}

impl BaseDataType {
    /// Parse the enum variant from the base type field value
    pub fn from_base_type_field(base_type_field: u8) -> Result<Self, DataTypeError> {
        match base_type_field {
            0x00 => Ok(BaseDataType::Enum),
            0x01 => Ok(BaseDataType::Sint8),
            0x02 => Ok(BaseDataType::Uint8),
            0x83 => Ok(BaseDataType::Sint16),
            0x84 => Ok(BaseDataType::Uint16),
            0x85 => Ok(BaseDataType::Sint32),
            0x86 => Ok(BaseDataType::Uint32),
            0x07 => Ok(BaseDataType::String),
            0x88 => Ok(BaseDataType::Float32),
            0x89 => Ok(BaseDataType::Float64),
            0x0A => Ok(BaseDataType::Uint8z),
            0x8B => Ok(BaseDataType::Uint16z),
            0x8C => Ok(BaseDataType::Uint32z),
            0x0D => Ok(BaseDataType::Byte),
            0x8E => Ok(BaseDataType::Sint64),
            0x8F => Ok(BaseDataType::Uint64),
            0x90 => Ok(BaseDataType::Uint64z),
            _ => Ok(BaseDataType::Unknown),
        }
    }

    pub fn get_parse(
        data_type: &Self,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match data_type {
            BaseDataType::Byte => parse_byte_array,
            BaseDataType::Enum => parse_unknown,
            BaseDataType::Float32 => parse_float32,
            BaseDataType::Float64 => parse_float64,
            BaseDataType::Sint8 => parse_sint8,
            BaseDataType::Sint16 => parse_sint16,
            BaseDataType::Sint32 => parse_sint32,
            BaseDataType::Sint64 => parse_sint64,
            BaseDataType::String => parse_string,
            BaseDataType::Uint8 => parse_uint8,
            BaseDataType::Uint8z => parse_uint8z,
            BaseDataType::Uint16 => parse_uint16,
            BaseDataType::Uint16z => parse_uint16z,
            BaseDataType::Uint32 => parse_uint32,
            BaseDataType::Uint32z => parse_uint32z,
            BaseDataType::Uint64 => parse_uint64,
            BaseDataType::Uint64z => parse_uint64z,
            BaseDataType::Unknown => parse_unknown,
        }
    }
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
        parse: BaseDataType::get_parse(&description.base_type),
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
    let size = content.next_u8()?;
    let _ = content.next_u8()?; // Byte for type is not used, but must still be used

    Ok(DefinitionField {
        endianness,
        kind,
        parse,
        size,
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        BaseDataValue,
        parser::types::generated::{DataValue, FitMessage, MesgNum, RecordField},
    };

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
        assert_eq!(definition.message_type, MesgNum::Record);
        assert_eq!(definition.fields_size, 1);
        assert_eq!(definition.fields.len(), 1);

        let field = definition.fields.first().unwrap();
        assert_eq!(field.endianness, Endianness::Little);
        assert_eq!(field.kind, FitMessage::Record(RecordField::HeartRate));

        let mut content = Reader::new(1, 0, vec![12].into_iter());
        assert_eq!(
            (field.parse)(&mut content, &Endianness::Little, 1).unwrap(),
            vec![DataValue::Base(BaseDataValue::Uint8(12))]
        );
    }
}
