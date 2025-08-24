use std::{collections::HashMap, fs, mem::discriminant};

use thiserror::Error;

pub use crate::parser::records::Record;
use crate::{
    BaseDataType, BaseDataValue,
    parser::{
        definition::{CustomDescription, Definition},
        header::{FileHeader, FileHeaderError},
        reader::Reader,
        records::{CompressedTimestamp, DataMessage},
        types::global_messages::{DataField, FieldDescriptionField, GlobalMessage},
    },
};

mod definition;
mod header;
mod reader;
mod records;
pub mod types;

#[derive(Error, Debug)]
pub enum FitParserError {
    #[error("Header parsing failed: {0}")]
    Header(#[from] FileHeaderError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn parse_records(file: &str) -> Result<Vec<Record>, FitParserError> {
    let mut content = fs::read(file)?.into_iter();

    let header = FileHeader::from_bytes(&mut content)?;
    let mut reader = Reader::new(header.data_size, header.crc, content);

    let mut definitions: HashMap<u8, Definition> = HashMap::new();
    let mut custom_descriptions: HashMap<u8, HashMap<u8, CustomDescription>> = HashMap::new();
    let mut compressed_timestamp = CompressedTimestamp::default();
    let mut records = Vec::new();

    loop {
        let record = match Record::parse(
            &mut reader,
            &definitions,
            &custom_descriptions,
            &mut compressed_timestamp,
        ) {
            Ok(record) => record,
            Err(err) => {
                println!("error: {:?}", err);
                break;
            }
        };
        match record {
            Record::Definition(ref definition) => {
                definitions.insert(definition.local_message_type, definition.clone());
            }
            Record::Data(ref data) => {
                parse_custom_definition_description(data, &definitions, &mut custom_descriptions);
                compressed_timestamp.set_last_timestamp(data.last_timestamp());
            }
            Record::CompressedTimestamp(_) => {}
        }
        records.push(record);

        if reader.is_empty() {
            break;
        }
    }

    Ok(records)
}

fn parse_custom_definition_description(
    message: &DataMessage,
    definitions: &HashMap<u8, Definition>,
    custom_descriptions: &mut HashMap<u8, HashMap<u8, CustomDescription>>,
) {
    let Some(definition) = definitions.get(&message.local_message_type) else {
        // No matching definition, should not be possible if message has been parsed (?)
        return;
    };

    match definition.message_type {
        GlobalMessage::FieldDescription => {}
        _ => return,
    };

    assert_eq!(message.fields.len(), definition.fields.len());

    let Some(base_type) = find_base_type(message, definition) else {
        return;
    };
    let Some(developer_data_index) = find_value_of_field_as_u8(
        message,
        definition,
        FieldDescriptionField::DeveloperDataIndex,
    ) else {
        return;
    };
    let Some(field_number) = find_value_of_field_as_u8(
        message,
        definition,
        FieldDescriptionField::FieldDefinitionNumber,
    ) else {
        return;
    };
    let Some(endianness) = definition.fields.first().map(|f| f.endianness) else {
        return;
    };
    let name = find_value_of_field_as_string(message, definition, FieldDescriptionField::FieldName);
    let units = find_value_of_field_as_string(message, definition, FieldDescriptionField::Units);

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
    variant: FieldDescriptionField,
) -> Option<BaseDataValue> {
    let index = definition
        .fields
        .iter()
        .position(|field| match field.kind {
            DataField::FieldDescription(field) => discriminant(&field) == discriminant(&variant),
            _ => false,
        })?;

    let field = message.fields.get(index)?;

    match field.kind {
        DataField::FieldDescription(field) => {
            if discriminant(&field) != discriminant(&variant) {
                return None;
            }
        }
        _ => return None,
    };

    field.values.first().cloned()
}

fn find_base_type(message: &DataMessage, definition: &Definition) -> Option<BaseDataType> {
    let val = match find_value_of_field(message, definition, FieldDescriptionField::FitBaseTypeId) {
        Some(types::BaseDataValue::Uint8(val)) => val,
        _ => return None,
    };

    BaseDataType::from_base_type_field(val).ok()
}

fn find_value_of_field_as_string(
    message: &DataMessage,
    definition: &Definition,
    variant: FieldDescriptionField,
) -> Option<String> {
    match find_value_of_field(message, definition, variant) {
        Some(types::BaseDataValue::String(val)) => Some(val.clone()),
        _ => None,
    }
}
fn find_value_of_field_as_u8(
    message: &DataMessage,
    definition: &Definition,
    variant: FieldDescriptionField,
) -> Option<u8> {
    match find_value_of_field(message, definition, variant) {
        Some(types::BaseDataValue::Uint8(val)) => Some(val),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_records;

    #[test]
    fn test_no_error() {
        let _ = parse_records("test.fit");
    }
}
