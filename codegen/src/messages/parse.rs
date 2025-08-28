use std::{collections::HashMap, path::Path};

use calamine::{Data, Reader, Xlsx, open_workbook};
use std::iter::zip;

use crate::{
    MESSAGES_TO_IMPORT,
    messages::{Field, Subfield, SubfieldReference},
    snake_to_camel_case,
    types::EnumName,
};

pub fn parse_messages_definitions(
    profile: &Path,
) -> (
    Vec<(String, Vec<Field>, HashMap<String, Vec<Subfield>>)>,
    Vec<EnumName>,
) {
    let mut workbook: Xlsx<_> = open_workbook(profile).expect("Unable to load profile file");
    let range = workbook
        .worksheet_range("Messages")
        .expect("The profile file does not contain a Types sheet");

    let mut iterator = range.rows();
    let _ = iterator.next(); // Skip header

    let mut messages = Vec::new();

    let mut message_name = match iterator.next().and_then(|r| r.first()) {
        Some(Data::String(name)) => name.to_string(),
        _ => return (messages, Vec::new()),
    };

    loop {
        let (next_message_name, definitions, subfields) = parse_fields_definitions(&mut iterator);

        messages.push((message_name, definitions, subfields));

        if next_message_name.is_none() {
            break;
        }

        message_name = next_message_name.unwrap();
    }

    if !MESSAGES_TO_IMPORT.is_empty() {
        messages.retain(|(msg, _, __)| {
            MESSAGES_TO_IMPORT.contains(&snake_to_camel_case(msg.as_str()).as_str())
        })
    }

    let enums_used = messages
        .iter()
        .flat_map(|(_, definitions, subfields)| {
            definitions
                .iter()
                .filter_map(|def| is_fit_enum(&def.base_type))
                .chain(subfields.iter().flat_map(|(_, subfields)| {
                    subfields
                        .iter()
                        .filter_map(|field| is_fit_enum(&field.base_type))
                }))
        })
        .collect();

    (messages, enums_used)
}

fn parse_fields_definitions<'a, I>(
    iter: &mut I,
) -> (Option<String>, Vec<Field>, HashMap<String, Vec<Subfield>>)
where
    I: Iterator<Item = &'a [Data]>,
{
    let mut fields = Vec::new();
    let mut next_message_name: Option<String> = None;
    let mut subfields: HashMap<String, Vec<Subfield>> = HashMap::new();

    let mut current_field = None;
    for row in iter {
        let colums = parse_columns(row);

        // Start of the next message, ends the loop there
        if let Some(message_name) = colums.message_name {
            next_message_name = Some(message_name);
            break;
        }

        // Start of a new field
        if let (Some(field_number), Some(field_name), Some(field_type)) = (
            colums.field_definition_number,
            &colums.field_name,
            &colums.field_type,
        ) {
            current_field = Some(field_name.to_string());
            fields.push(Field {
                field_def: field_number,
                name: field_name.to_string(),
                base_type: field_type.to_string(),
                scale: colums.scale,
                offset: colums.offset,
            });
        }

        // New subfield for the current field
        if let (
            Some(current_field),
            Some(subfield_name),
            Some(field_type),
            Some(reference_fields),
            Some(reference_field_values),
        ) = (
            &current_field,
            &colums.field_name,
            &colums.field_type,
            colums.subfield_references,
            colums.subfield_reference_values,
        ) {
            assert_eq!(reference_fields.len(), reference_field_values.len());

            let references = zip(reference_fields, reference_field_values)
                .map(|(name, value)| {
                    SubfieldReference {
                        name,
                        value,
                        base_type: None, // All message's fields must parsed before we can lookup the type of the reference field
                    }
                })
                .collect();

            subfields
                .entry(current_field.to_string())
                .or_default()
                .push(Subfield {
                    name: subfield_name.to_string(),
                    base_type: field_type.to_string(),
                    references,
                    scale: colums.scale,
                    offset: colums.offset,
                });
        }
    }

    // Update the type of reference fields
    for (_, message_subfields) in subfields.iter_mut() {
        for subfield in message_subfields.iter_mut() {
            for reference in subfield.references.iter_mut() {
                if let Some(field) = fields.iter().find(|field| field.name == reference.name) {
                    reference.base_type = Some(field.base_type.clone())
                }
            }
        }
    }

    (next_message_name, fields, subfields)
}

struct Columns {
    message_name: Option<String>,
    field_definition_number: Option<u8>,
    field_name: Option<String>,
    field_type: Option<String>,
    scale: Option<f32>,
    offset: Option<f32>,
    subfield_references: Option<Vec<String>>,
    subfield_reference_values: Option<Vec<String>>,
}
fn parse_columns(row: &[Data]) -> Columns {
    let message_name = column_string_content(row, 0);
    let field_definition_number = row.get(1).and_then(|data| match data {
        Data::Int(field_def) => Some(*field_def as u8),
        Data::Float(field_def) => Some(*field_def as u8),
        _ => None,
    });
    let field_name = column_string_content(row, 2);
    let field_type = column_string_content(row, 3);
    let scale = row.get(6).and_then(|data| match data {
        Data::Int(field_def) => Some(*field_def as f32),
        Data::Float(field_def) => Some(*field_def as f32),
        _ => None,
    });
    let offset = row.get(7).and_then(|data| match data {
        Data::Int(field_def) => Some(*field_def as f32),
        Data::Float(field_def) => Some(*field_def as f32),
        _ => None,
    });
    let subfield_reference =
        column_string_content(row, 11).map(|s| s.split(",").map(|w| w.to_string()).collect());
    let subfield_reference_value =
        column_string_content(row, 12).map(|s| s.split(",").map(|w| w.to_string()).collect());

    Columns {
        message_name,
        field_definition_number,
        field_name,
        field_type,
        scale,
        offset,
        subfield_references: subfield_reference,
        subfield_reference_values: subfield_reference_value,
    }
}

fn column_string_content(row: &[Data], index: usize) -> Option<String> {
    row.get(index).and_then(|data| match data {
        Data::String(name) => Some(name.to_string()),
        _ => None,
    })
}

fn is_fit_enum(type_name: &str) -> Option<String> {
    match type_name {
        "sint8" | "uint8" | "uint8z" | "sint16" | "uint16" | "uint16z" | "sint32" | "uint32"
        | "uint32z" | "sint64" | "uint64" | "uint64z" | "string" | "float32" | "float64"
        | "byte" => None,
        val => Some(val.to_string()),
    }
}
