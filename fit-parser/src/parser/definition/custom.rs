use std::{collections::HashMap, mem::discriminant};

use crate::{
    DataValue,
    parser::{
        definition::Definition,
        records::DataMessage,
        types::generated::{FieldDescriptionField, FitBaseType, FitEnum, FitField, MesgNum},
    },
};

#[derive(Debug, Clone)]
pub struct CustomDescription {
    pub base_type: FitBaseType,
    pub name: Option<String>,
    pub units: Option<String>,
}

pub fn parse_custom_definition_description(
    message: &DataMessage,
    definitions: &HashMap<u8, Definition>,
    custom_descriptions: &mut HashMap<u8, HashMap<u8, CustomDescription>>,
) {
    let Some(definition) = definitions.get(&message.local_message_type) else {
        // No matching definition, should not be possible if message has been parsed (?)
        return;
    };

    match definition.message_type {
        MesgNum::FieldDescription => {}
        _ => {
            return;
        }
    };

    assert_eq!(message.fields.len(), definition.fields.len());

    let Some(base_type) = find_base_type(message, definition) else {
        return;
    };
    let Some(developer_data_index) = find_value_of_field_as_u8(
        message,
        definition,
        &FieldDescriptionField::DeveloperDataIndex,
    ) else {
        return;
    };
    let Some(field_number) = find_value_of_field_as_u8(
        message,
        definition,
        &FieldDescriptionField::FieldDefinitionNumber,
    ) else {
        return;
    };

    let name =
        find_value_of_field_as_string(message, definition, &FieldDescriptionField::FieldName);
    let units = find_value_of_field_as_string(message, definition, &FieldDescriptionField::Units);

    let description = CustomDescription {
        base_type,
        name,
        units,
    };

    custom_descriptions
        .entry(developer_data_index)
        .or_default()
        .insert(field_number, description);
}

fn find_value_of_field(
    message: &DataMessage,
    definition: &Definition,
    variant: &FieldDescriptionField,
) -> Option<DataValue> {
    let index = definition
        .fields
        .iter()
        .position(|field| match &field.kind {
            FitField::FieldDescription(field) => discriminant(field) == discriminant(variant),
            _ => false,
        })?;

    let field = message.fields.get(index)?;

    match &field.kind {
        FitField::FieldDescription(field) => {
            if discriminant(field) != discriminant(variant) {
                return None;
            }
        }
        _ => return None,
    };

    field.values.first().cloned()
}

fn find_base_type(message: &DataMessage, definition: &Definition) -> Option<FitBaseType> {
    let val = match find_value_of_field(message, definition, &FieldDescriptionField::FitBaseTypeId)
    {
        Some(DataValue::Enum(FitEnum::FitBaseType(t))) => t,

        _ => return None,
    };

    Some(val)
}

fn find_value_of_field_as_string(
    message: &DataMessage,
    definition: &Definition,
    variant: &FieldDescriptionField,
) -> Option<String> {
    match find_value_of_field(message, definition, variant) {
        Some(DataValue::String(val)) => Some(val.clone()),
        _ => None,
    }
}
fn find_value_of_field_as_u8(
    message: &DataMessage,
    definition: &Definition,
    variant: &FieldDescriptionField,
) -> Option<u8> {
    match find_value_of_field(message, definition, variant) {
        Some(DataValue::Uint8(val)) => Some(val),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use crate::{
        DataValue,
        parser::{
            Endianness,
            definition::DefinitionField,
            records::DataMessageField,
            types::{
                generated::{FitBaseType, FitEnum, ParseFunction},
                parse_string, parse_uint8,
            },
        },
    };

    use super::*;

    #[test]
    fn test_parse_custom_definition() {
        let mut descriptions = HashMap::new();
        let mut definitions = HashMap::new();
        definitions.insert(
            0,
            Definition {
                message_type: MesgNum::FieldDescription,
                local_message_type: 0,
                fields: vec![
                    DefinitionField {
                        endianness: Endianness::Little,
                        kind: FitField::FieldDescription(FieldDescriptionField::DeveloperDataIndex),
                        parse: ParseFunction::Simple(parse_uint8),
                        scale_offset: None,
                        size: 1,
                    },
                    DefinitionField {
                        endianness: Endianness::Little,
                        kind: FitField::FieldDescription(
                            FieldDescriptionField::FieldDefinitionNumber,
                        ),
                        parse: ParseFunction::Simple(parse_uint8),
                        scale_offset: None,
                        size: 1,
                    },
                    DefinitionField {
                        endianness: Endianness::Little,
                        kind: FitField::FieldDescription(FieldDescriptionField::FitBaseTypeId),
                        parse: ParseFunction::Simple(FitBaseType::parse),
                        scale_offset: None,
                        size: 1,
                    },
                    DefinitionField {
                        endianness: Endianness::Little,
                        kind: FitField::FieldDescription(FieldDescriptionField::FieldName),
                        parse: ParseFunction::Simple(parse_string),
                        scale_offset: None,
                        size: 64,
                    },
                    DefinitionField {
                        endianness: Endianness::Little,
                        kind: FitField::FieldDescription(FieldDescriptionField::Units),
                        parse: ParseFunction::Simple(parse_string),
                        scale_offset: None,
                        size: 16,
                    },
                ],
            },
        );

        let message = DataMessage {
            local_message_type: 0,
            fields: vec![
                DataMessageField {
                    kind: FitField::FieldDescription(FieldDescriptionField::DeveloperDataIndex),
                    values: vec![DataValue::Uint8(0)],
                },
                DataMessageField {
                    kind: FitField::FieldDescription(FieldDescriptionField::FieldDefinitionNumber),
                    values: vec![DataValue::Uint8(0)],
                },
                DataMessageField {
                    kind: FitField::FieldDescription(FieldDescriptionField::FitBaseTypeId),
                    values: vec![DataValue::Enum(FitEnum::FitBaseType(FitBaseType::Sint8))],
                },
                DataMessageField {
                    kind: FitField::FieldDescription(FieldDescriptionField::FieldName),
                    values: vec![DataValue::String("new field".to_string())],
                },
                DataMessageField {
                    kind: FitField::FieldDescription(FieldDescriptionField::Units),
                    values: vec![DataValue::String("km/h".to_string())],
                },
            ],
        };

        assert!(descriptions.is_empty());
        parse_custom_definition_description(&message, &definitions, &mut descriptions);

        assert!(descriptions.contains_key(&0));

        let fields = descriptions.get(&0).unwrap();
        assert!(fields.contains_key(&0));

        let description = fields.get(&0).unwrap();
        assert_eq!(description.name, Some("new field".to_string()));
        assert_eq!(description.units, Some("km/h".to_string()));
        assert_eq!(description.base_type, FitBaseType::Sint8);
    }
}
