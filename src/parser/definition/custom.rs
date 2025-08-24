use std::{collections::HashMap, mem::discriminant};

use crate::parser::{
    definition::{BaseDataType, Definition, Endianness},
    records::DataMessage,
    types::{
        self,
        generated::{DataValue, FieldDescriptionField, FitMessage, MesgNum},
    },
};

#[derive(Debug, Clone)]
pub struct CustomDescription {
    pub endianness: Endianness,
    pub base_type: BaseDataType,
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
        _ => return,
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
    let Some(endianness) = definition.fields.first().map(|f| f.endianness) else {
        return;
    };
    let name =
        find_value_of_field_as_string(message, definition, &FieldDescriptionField::FieldName);
    let units = find_value_of_field_as_string(message, definition, &FieldDescriptionField::Units);

    let description = CustomDescription {
        base_type,
        endianness,
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
            FitMessage::FieldDescription(field) => discriminant(field) == discriminant(variant),
            _ => false,
        })?;

    let field = message.fields.get(index)?;

    match &field.kind {
        FitMessage::FieldDescription(field) => {
            if discriminant(field) != discriminant(variant) {
                return None;
            }
        }
        _ => return None,
    };

    field.values.first().cloned()
}

fn find_base_type(message: &DataMessage, definition: &Definition) -> Option<BaseDataType> {
    let val = match find_value_of_field(message, definition, &FieldDescriptionField::FitBaseTypeId)
    {
        Some(DataValue::Base(types::BaseDataValue::Uint8(val))) => val,
        _ => return None,
    };

    BaseDataType::from_base_type_field(val).ok()
}

fn find_value_of_field_as_string(
    message: &DataMessage,
    definition: &Definition,
    variant: &FieldDescriptionField,
) -> Option<String> {
    match find_value_of_field(message, definition, variant) {
        Some(DataValue::Base(types::BaseDataValue::String(val))) => Some(val.clone()),
        _ => None,
    }
}
fn find_value_of_field_as_u8(
    message: &DataMessage,
    definition: &Definition,
    variant: &FieldDescriptionField,
) -> Option<u8> {
    match find_value_of_field(message, definition, variant) {
        Some(DataValue::Base(types::BaseDataValue::Uint8(val))) => Some(val),
        _ => None,
    }
}
