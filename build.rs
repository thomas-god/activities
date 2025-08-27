#![allow(clippy::const_is_empty)]
#![allow(clippy::type_complexity)]

use itertools::join;
use std::collections::HashMap;
use std::io::Write;
use std::iter::zip;
use std::process::{Command, Stdio};

use calamine::{Data, Reader, Xlsx, open_workbook};

const MESSAGES_TO_IMPORT: &[&str] = &[]; // If empty, every message type is imported
// const MESSAGES_TO_IMPORT: &[&str] = &["Record", "FieldDescription", "DeviceInfo"];
const BASE_TYPES: &[&str] = &[
    "sint8", "uint8", "uintz8", "sint16", "uint16", "uintz16", "sint32", "uint32", "uintz32",
    "sint64", "uint64", "uintz64", "string", "float32", "float64", "byte",
];
const ENUMS_SKIPPED_VARIANTS: &[&str] = &["mfg_range_min", "mfg_range_max", "pad"];

type EnumName = String;
type EnumVariant = String;
type EnumType = String;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let mut enums = parse_enums();
    let (messages, enums_used) = parse_messages_definitions();

    enums.retain(|(name, _, __)| enums_used.contains(name));

    let enums_names = enums.iter().map(|(name, _, __)| name.clone()).collect();

    let mut code = generate_enums_code(&enums);
    code.push_str(&generate_messages_code(messages, enums_names));
    code = format_code(&code);

    std::fs::write("src/parser/types/generated.rs", code).expect("Could not wirte to ouptut file");
}

fn parse_enums() -> Vec<(EnumName, EnumType, Vec<(usize, EnumVariant)>)> {
    let mut workbook: Xlsx<_> = open_workbook("Profile.xlsx").expect("Unable to load profile file");
    let range = workbook
        .worksheet_range("Types")
        .expect("The profile file does not contain a Types sheet");

    let mut iterator = range.rows();
    let _ = iterator.next(); // Skip header

    let mut enums = Vec::new();

    let EnumRow {
        name: mut type_name,
        mut enum_type,
        ..
    } = parse_enum_row(iterator.next().expect("Unable to parse row"));

    loop {
        let (mapping, next_type_name, next_base_type) = parse_enum_variants(&mut iterator);

        enums.push((type_name.unwrap(), enum_type.unwrap(), mapping));

        if next_type_name.is_none() && next_base_type.is_none() {
            break;
        }

        type_name = next_type_name;
        enum_type = next_base_type;
    }

    enums
}

#[derive(Debug)]
struct EnumRow {
    name: Option<EnumName>,
    enum_type: Option<EnumType>,
    variant_name: Option<EnumVariant>,
    variant_value: Option<usize>,
}

fn parse_enum_row(row: &[Data]) -> EnumRow {
    EnumRow {
        name: match row.first() {
            Some(Data::String(name)) => Some(name.clone()),
            _ => None,
        },
        enum_type: match row.get(1) {
            Some(Data::String(enum_type)) => Some(enum_type.clone()),
            _ => None,
        },
        variant_name: match row.get(2) {
            Some(Data::String(variant_name)) => Some(variant_name.clone()),
            _ => None,
        },
        variant_value: match row.get(3) {
            Some(Data::Int(value)) => Some(*value as usize),
            Some(Data::Float(value)) => Some(*value as usize),
            Some(Data::String(value)) => {
                if let Some(stripped) = value.strip_prefix("0x") {
                    usize::from_str_radix(stripped, 16).ok()
                } else {
                    None
                }
            }

            _ => None,
        },
    }
}

fn parse_enum_variants<'a, I>(
    iterator: &mut I,
) -> (
    Vec<(usize, EnumVariant)>,
    Option<EnumName>,
    Option<EnumType>,
)
where
    I: Iterator<Item = &'a [Data]>,
{
    let mut variants = Vec::new();
    let mut next_enum_name = None;
    let mut next_enum_type = None;

    for row in iterator {
        let row = parse_enum_row(row);

        if row.name.is_some() && row.enum_type.is_some() {
            next_enum_name = row.name;
            next_enum_type = row.enum_type;
            break;
        }

        if row.variant_name.is_some()
            && row.variant_value.is_some()
            && !ENUMS_SKIPPED_VARIANTS.contains(&row.variant_name.clone().unwrap().as_str())
        {
            variants.push((row.variant_value.unwrap(), row.variant_name.unwrap()));
        }
    }

    (variants, next_enum_name, next_enum_type)
}

fn map_fit_type_to_rust_type(val: &str) -> Option<String> {
    match val {
        "enum" => Some("u8".to_string()),
        "uint8" => Some("u8".to_string()),
        "uint8z" => Some("u8".to_string()),
        "uint16" => Some("u16".to_string()),
        "uint32" => Some("u32".to_string()),
        "uint32z" => Some("u32".to_string()),
        _ => None,
    }
}

fn fit_type_size(val: &str) -> Option<u8> {
    match val {
        "enum" => Some(1),
        "uint8" => Some(1),
        "uint8z" => Some(1),
        "uint16" => Some(2),
        "uint32" => Some(4),
        "uint32z" => Some(4),
        _ => None,
    }
}

fn generate_enums_code(enums: &[(EnumName, EnumType, Vec<(usize, EnumVariant)>)]) -> String {
    let mut code = String::new();
    code.push_str("// Code generated by build.rs - do not modify manually\n");
    code.push_str("#![allow(dead_code)]\n");
    code.push_str("#![allow(unused_imports)]\n");
    code.push_str("#![allow(unused_variables)]\n");
    code.push_str("#![allow(unreachable_patterns)]\n\n");
    code.push_str("#![allow(clippy::enum_variant_names)]\n");
    code.push_str("#![allow(clippy::upper_case_acronyms)]\n\n");
    code.push_str("#![allow(clippy::identity_op)]\n\n");
    code.push_str("#![allow(clippy::type_complexity)]\n\n");
    code.push_str("#![allow(clippy::match_single_binding)]\n\n");
    code.push_str("#![allow(clippy::match_overlapping_arm)]\n\n");
    code.push_str("use crate::{parser::reader::Reader};\n");
    code.push_str("use crate::{parser::records::DataMessageField};\n");
    code.push_str(
        "use crate::parser::types::{parse_uint8, parse_uint8z, parse_sint8,
        parse_uint16, parse_uint16z, parse_sint16,
        parse_uint32, parse_uint32z, parse_sint32,
        parse_uint64, parse_uint64z, parse_sint64,
        parse_float32, parse_float64, parse_string,
        parse_unknown, parse_byte_array as parse_byte, ScaleOffset,
        DataValue, DataTypeError};",
    );
    code.push_str("use crate::parser::definition::{Endianness};\n\n");

    code.push_str(&main_fit_enum_code(enums));

    for (enum_name, enum_type, enum_variants) in enums.iter() {
        if enum_name.contains("date_time") {
            code.push_str(&generate_datetime_like_code(enum_name));
        } else {
            code.push_str(&generate_enum_code(enum_name, enum_type, enum_variants));
        }
    }

    code.push_str(&fit_base_type_enum_code());

    code
}

fn main_fit_enum_code(enums: &[(EnumName, EnumType, Vec<(usize, EnumVariant)>)]) -> String {
    let enum_names: Vec<String> = enums.iter().map(|(name, _, __)| name.to_string()).collect();
    let variants = join(
        enum_names.iter().map(|name| {
            format!(
                "{}({})",
                snake_to_camel_case(name),
                snake_to_camel_case(name)
            )
        }),
        ",\n",
    );

    format!(
        r#"
#[derive(Debug, PartialEq, Clone)]
pub enum FitEnum {{
    {variants}
}}
"#
    )
}

fn fit_base_type_enum_code() -> String {
    "
impl FitBaseType {
    pub fn get_parse_fn(
        &self
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match self {
            Self::Byte => parse_byte,
            Self::Enum => parse_unknown,
            Self::Float32 => parse_float32,
            Self::Float64 => parse_float64,
            Self::Sint8 => parse_sint8,
            Self::Sint16 => parse_sint16,
            Self::Sint32 => parse_sint32,
            Self::Sint64 => parse_sint64,
            Self::String => parse_string,
            Self::Uint8 => parse_uint8,
            Self::Uint8z => parse_uint8z,
            Self::Uint16 => parse_uint16,
            Self::Uint16z => parse_uint16z,
            Self::Uint32 => parse_uint32,
            Self::Uint32z => parse_uint32z,
            Self::Uint64 => parse_uint64,
            Self::Uint64z => parse_uint64z,
            Self::UnknownVariant(_) => parse_unknown,
        }
    }
}
    "
    .to_string()
}

fn generate_datetime_like_code(name: &str) -> String {
    let name = snake_to_camel_case(name);

    // Datetime like "enums" are directly defined as a struct(u32)
    format!(
        r#"
#[derive(Debug, PartialEq, Clone)]
pub struct {name}(u32);

impl {name} {{
    pub fn from(content: u32) -> {name} {{
        Self(content)
    }}

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8
    ) -> Result<Vec<DataValue>, DataTypeError> {{
        let mut values = Vec::new();

        for _ in 0..number_of_bytes / 4 {{
            values.push(DataValue::DateTime(reader.next_u32(endianness)?));
        }}

        Ok(values)
    }}
}}
            "#
    )
    .to_string()
}

fn generate_enum_code(name: &str, base_type: &str, mapping: &[(usize, EnumVariant)]) -> String {
    let mut code = String::new();
    let enum_name = snake_to_camel_case(name);
    let enum_type = map_fit_type_to_rust_type(base_type).expect("Expected not None enum type");
    let enum_type_size = fit_type_size(base_type).expect("Expected not None enum type");

    let variants = join(
        mapping
            .iter()
            .map(|(_, v)| snake_to_camel_case(v))
            .chain(vec![format!("UnknownVariant({enum_type})").to_string()]),
        ",\n",
    );

    // Define the enum and its variants
    code.push_str(&format!(
        "
#[derive(Debug, PartialEq, Clone)]
pub enum {enum_name} {{
    {variants}
}}"
    ));

    // Start impl block
    code.push_str(&format!("impl {enum_name} {{").to_string());

    // Define the mapping from u8 to enum's variants
    let enum_mapping = join(
        mapping
            .iter()
            .map(|(definition_number, variant)| {
                format!(
                    "{definition_number} => {enum_name}::{}",
                    snake_to_camel_case(variant)
                )
            })
            .chain(vec![format!("val => {enum_name}::UnknownVariant(val)")]),
        ",\n",
    );
    code.push_str(&format!(
        r#"

    pub fn from(content: {enum_type}) -> {enum_name} {{
        match content {{
            {enum_mapping}
        }}
    }}
        "#
    ));

    // Define parsing function
    let parse_arguments = if enum_type.contains("8") {
        ""
    } else {
        "endianness"
    };
    code.push_str(&format!(
        "
        pub fn parse(
            reader: &mut Reader,
            endianness: &Endianness,
            number_of_bytes: u8
        ) -> Result<Vec<DataValue>, DataTypeError> {{
            let mut values = Vec::new();
            for _ in 0..number_of_bytes / {enum_type_size} {{
                values.push(DataValue::Enum(FitEnum::{enum_name}(
                    Self::from(reader.next_{enum_type}({parse_arguments})?)
                )));
            }}
            Ok(values)
        }}"
    ));

    // MesgNum need special treatment to be able to link to a FitMessage
    if enum_name == "MesgNum" {
        code.push_str(&generate_mesg_num_mappings(mapping));
    }

    // Close impl block
    code.push_str("}\n");

    code
}

fn generate_mesg_num_mappings(mapping: &[(usize, String)]) -> String {
    let mut code = String::new();
    let mapping_field = join(
        mapping
            .iter()
            .filter(|(_, v)| {
                if !MESSAGES_TO_IMPORT.is_empty() {
                    MESSAGES_TO_IMPORT.contains(&snake_to_camel_case(v).as_str())
                } else {
                    true
                }
            })
            .map(|(_, v)| {
                format!(
                    "Self::{} => FitMessage::{}({}Field::from(def_number))",
                    snake_to_camel_case(v),
                    snake_to_camel_case(v),
                    snake_to_camel_case(v)
                )
            })
            .chain(vec!["_ => FitMessage::UnknownVariant(0)".to_string()]),
        ",\n",
    );

    let mapping_parse = join(
        mapping
            .iter()
            .filter(|(_, v)| {
                if !MESSAGES_TO_IMPORT.is_empty() {
                    MESSAGES_TO_IMPORT.contains(&snake_to_camel_case(v).as_str())
                } else {
                    true
                }
            })
            .map(|(_, v)| {
                format!(
                    "Self::{} => {}Field::get_parse_function(def_number)",
                    snake_to_camel_case(v),
                    snake_to_camel_case(v)
                )
            })
            .chain(vec![
                "_ => ParseFunction::Simple(parse_unknown)".to_string(),
            ]),
        ",\n",
    );

    let mapping_scale_offset = join(
        mapping
            .iter()
            .filter(|(_, v)| {
                if !MESSAGES_TO_IMPORT.is_empty() {
                    MESSAGES_TO_IMPORT.contains(&snake_to_camel_case(v).as_str())
                } else {
                    true
                }
            })
            .map(|(_, v)| {
                format!(
                    "Self::{} => {}Field::get_scale_offset(def_number)",
                    snake_to_camel_case(v),
                    snake_to_camel_case(v)
                )
            })
            .chain(vec!["_ => None".to_string()]),
        ",\n",
    );

    let mapping_timestamp_field = join(
        mapping
            .iter()
            .filter(|(_, v)| {
                if !MESSAGES_TO_IMPORT.is_empty() {
                    MESSAGES_TO_IMPORT.contains(&snake_to_camel_case(v).as_str())
                } else {
                    true
                }
            })
            .map(|(_, v)| {
                format!(
                    "Self::{} => {}Field::timestamp_field()",
                    snake_to_camel_case(v),
                    snake_to_camel_case(v)
                )
            })
            .chain(vec!["_ => None".to_string()]),
        ",\n",
    );

    code.push_str(&format!(
        "
    pub fn message_field(&self, def_number: u8) -> FitMessage {{
        match self {{
            {mapping_field}
        }}
    }}

    pub fn field_parse(
        &self, def_number: u8
    ) -> ParseFunction {{
        match self {{
            {mapping_parse}
        }}
    }}

    pub fn scale_offset(
        &self,
        def_number: u8
    ) -> Option<ScaleOffset> {{
        match self {{
            {mapping_scale_offset}
        }}
    }}

    pub fn timestamp_field(
        &self,
    ) -> Option<FitMessage> {{
        match self {{
            {mapping_timestamp_field}
        }}
    }}"
    ));
    code
}

fn snake_to_camel_case(input: &str) -> String {
    let trimmed = input.trim_start_matches(char::is_numeric);
    trimmed
        .split('_')
        .map(|w| {
            let mut new = w.to_string();
            if new.is_empty() {
                return new;
            }
            if let Some((idx, c)) = new.char_indices().next() {
                new.replace_range(idx..idx + 1, &c.to_uppercase().to_string());
            }

            new
        })
        .collect()
}

fn format_code(code: &str) -> String {
    let mut child = Command::new("rustfmt")
        .arg("--emit=stdout")
        .arg("--quiet")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn rustfmt");

    // Write code to rustfmt's stdin
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(code.as_bytes())
            .expect("Failed to write to rustfmt");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to read rustfmt output");

    if output.status.success() {
        String::from_utf8(output.stdout).unwrap_or_else(|_| code.to_string())
    } else {
        println!("cargo:warning=rustfmt failed, using unformatted code");
        code.to_string()
    }
}

#[derive(Debug)]
struct Field {
    field_def: u8,
    name: String,
    base_type: String,
    // array: Option<usize>,
    scale: Option<f32>,
    offset: Option<f32>,
}

fn parse_messages_definitions() -> (
    Vec<(String, Vec<Field>, HashMap<String, Vec<Subfield>>)>,
    Vec<EnumName>,
) {
    let mut workbook: Xlsx<_> = open_workbook("Profile.xlsx").expect("Unable to load profile file");
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

    let reference_types: Vec<String> = messages
        .iter()
        .flat_map(|(_, __, subfields)| {
            subfields.values().flat_map(|message_subfields| {
                message_subfields.iter().flat_map(|subfield| {
                    subfield
                        .references
                        .iter()
                        .flat_map(|reference| reference.base_type.clone())
                })
            })
        })
        .collect();
    dbg!(reference_types);

    (messages, enums_used)
}

fn is_fit_enum(type_name: &str) -> Option<String> {
    match type_name {
        "sint8" | "uint8" | "uintz8" | "sint16" | "uint16" | "uintz16" | "sint32" | "uint32"
        | "uintz32" | "sint64" | "uint64" | "uintz64" | "string" | "float32" | "float64"
        | "byte" => None,
        val => Some(val.to_string()),
    }
}

fn get_parse_function(enums: &[String], type_name: &str) -> String {
    if BASE_TYPES.contains(&type_name) {
        return format!("parse_{}", type_name).to_string();
    }

    if enums.contains(&type_name.to_string()) {
        return format!("{}::parse", snake_to_camel_case(type_name)).to_string();
    }

    "parse_unknown".to_string()
}

#[derive(Debug)]
struct Subfield {
    parent_field_definition_number: u8,
    name: String,
    base_type: String,
    references: Vec<SubfieldReference>,
    scale: Option<f32>,
    offset: Option<f32>,
}

#[derive(Debug)]
struct SubfieldReference {
    name: String,
    value: String,
    base_type: Option<String>,
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
    let mut current_definition_field = None;
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
            current_definition_field = Some(field_number);
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
            Some(parent_field_definition_number),
            Some(subfield_name),
            Some(field_type),
            Some(reference_fields),
            Some(reference_field_values),
        ) = (
            &current_field,
            &current_definition_field,
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
                    parent_field_definition_number: *parent_field_definition_number,
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

fn generate_messages_code(
    messages: Vec<(String, Vec<Field>, HashMap<String, Vec<Subfield>>)>,
    enums: Vec<String>,
) -> String {
    let mut code = String::new();

    let mut messages_enum = join(
        messages.iter().map(|(msg, _, __)| {
            format!(
                "{}({}Field)",
                snake_to_camel_case(msg),
                snake_to_camel_case(msg)
            )
        }),
        ",\n",
    );
    if !messages_enum.is_empty() {
        messages_enum.push(',');
    }

    code.push_str(&format!(
        r#"

pub type SimpleParseFunction = fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError>;
pub type DynamicParseFunction = fn(&mut Reader, &Endianness, u8, &[DataMessageField]) -> Result<DataMessageField, DataTypeError>;

#[derive(Debug, Clone)]
pub enum ParseFunction {{
    Simple(SimpleParseFunction),
    Dynamic(DynamicParseFunction)
}}

#[derive(Debug, PartialEq, Clone)]
struct Subfield {{
    parent_field_definition_number: u8,
    name: String,
    base_type: String,
    reference_fields: Vec<String>,
    reference_field_values: Vec<String>,
    scale: Option<f32>,
    offset: Option<f32>,
}}

#[derive(Debug, PartialEq, Clone)]
pub enum FitMessage {{
    {messages_enum}
    Custom(CustomField),
    UnknownVariant(u8)
}}

#[derive(Debug, PartialEq, Clone)]
pub struct CustomField {{
    pub name: Option<String>,
    pub units: Option<String>,
}}"#
    ));

    for (msg, definitions, subfields) in messages.iter() {
        let message = snake_to_camel_case(msg);

        // Generate all variants (fields and their subfields)
        let variants = join(
            definitions
                .iter()
                .map(|def| {
                    let base_variant = snake_to_camel_case(&def.name).to_string();
                    let mut subfields: Vec<String> = subfields
                        .get(&def.name)
                        .map(|fields| {
                            fields
                                .iter()
                                .map(|field| snake_to_camel_case(&field.name))
                                .collect()
                        })
                        .unwrap_or_default();

                    subfields.insert(0, base_variant);
                    join(subfields.into_iter(), ",\n")
                })
                .chain(vec!["Unknown".to_string()]),
            ",\n",
        );

        code.push_str(&format!(
            r#"
#[derive(Debug, PartialEq, Clone)]
pub enum {message}Field {{
    {variants}
}}"#
        ));

        let from_definition_field_mappings = join(
            definitions
                .iter()
                .map(|def| {
                    format!(
                        "{} => Self::{}",
                        def.field_def,
                        snake_to_camel_case(&def.name)
                    )
                })
                .chain(vec!["_ => Self::Unknown".to_string()]),
            ",\n",
        );

        for field in definitions {
            if let Some(subfields_enums) = generate_subfields_enum(&message, field, subfields) {
                code.push_str(&subfields_enums);
            }
        }

        let parse_mappings = join(
            definitions
                .iter()
                .map(|def| match subfields.get(&def.name) {
                    None => format!(
                        "{} => ParseFunction::Simple({})",
                        def.field_def,
                        get_parse_function(&enums, &def.base_type)
                    ),
                    Some(_) => format!(
                        "{} => ParseFunction::Dynamic({}Field{}Subfield::parse)",
                        def.field_def,
                        snake_to_camel_case(&message),
                        snake_to_camel_case(&def.name)
                    ),
                })
                .chain(vec!["_ => ParseFunction::Simple(parse_uint8)".to_string()]),
            ",\n",
        );

        let scale_offset_mapping = join(
            definitions
                .iter()
                .filter_map(|def| {
                    if def.offset.is_some() || def.scale.is_some() {
                        Some(format!(
                            "{} => Some(ScaleOffset {{
                                scale: {}_f32,
                                offset: {}_f32
                            }})",
                            def.field_def,
                            def.scale.unwrap_or(1.),
                            def.offset.unwrap_or(0.)
                        ))
                    } else {
                        None
                    }
                })
                .chain(vec!["_ => None".to_string()]),
            ",\n",
        );

        let timestamp_field = definitions
            .iter()
            .filter_map(|def| {
                if def.name == "timestamp" {
                    Some(format!(
                        "Some(FitMessage::{message}({message}Field::Timestamp))"
                    ))
                } else {
                    None
                }
            })
            .next()
            .unwrap_or("None".to_string());

        code.push_str(&format!(
            r#"
impl {message}Field {{

    fn from(definition_field: u8) -> Self {{
        match definition_field {{
            {from_definition_field_mappings}
        }}
    }}

    fn get_parse_function(
        def_number: u8
    ) -> ParseFunction {{
        match def_number {{
            {parse_mappings}
        }}
    }}

    fn get_scale_offset(
        def_number: u8
    ) -> Option<ScaleOffset> {{
        match def_number {{
            {scale_offset_mapping}
        }}
    }}

    fn timestamp_field() -> Option<FitMessage> {{
        {timestamp_field}
    }}
}}"#
        ));
    }

    code
}

fn generate_subfields_enum(
    message_name: &str,
    field: &Field,
    subfields: &HashMap<String, Vec<Subfield>>,
) -> Option<String> {
    let subfields = subfields.get(&field.name)?;
    let parent_field = snake_to_camel_case(&field.name);
    let parent_field_parse = if BASE_TYPES.contains(&field.base_type.as_str()) {
        format!("parse_{}", field.base_type)
    } else {
        format!("{}::parse", snake_to_camel_case(&field.base_type))
    };
    let mut code = String::new();

    let subfield_variants = join(
        subfields
            .iter()
            .map(|subfield| snake_to_camel_case(&subfield.name)),
        ",\n",
    );

    // Define enum for all subfields
    code.push_str(&format!(
        "#[derive(Debug, PartialEq, Clone)]
pub enum {message_name}Field{parent_field}Subfield {{
    {subfield_variants}
}}"
    ));

    // Start impl block
    code.push_str(&format!("
impl {message_name}Field{parent_field}Subfield {{

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        bytes_to_read: u8,
        fields: &[DataMessageField]
    ) -> Result<DataMessageField, DataTypeError> {{
        for match_subfield in {message_name}Field{parent_field}Subfield::subfields_parse_functions() {{
            if let Some(parse) = match_subfield(fields) {{
                return parse(reader, endianness, bytes_to_read);
            }}
        }};

        // Default parse
        let values = {parent_field_parse}(reader, endianness, bytes_to_read)?;

        Ok(DataMessageField {{
            kind: FitMessage::{message_name}({message_name}Field::{parent_field}),
            values
        }})
    }}", ));

    // Implement subfield detection functions
    let subfield_targets = |subfield: &Subfield| -> String {
        join(
            subfield.references.iter().map(|reference| {
                let base_type = snake_to_camel_case(reference.base_type.as_ref().unwrap());
                let field = format!(
                    "FitMessage::{message_name}({message_name}Field::{})",
                    snake_to_camel_case(&reference.name)
                );
                let value = format!(
                    "DataValue::Enum(FitEnum::{}({}::{}))",
                    base_type,
                    base_type,
                    snake_to_camel_case(&reference.value),
                );
                format!("({field}, {value})")
            }),
            ",",
        )
    };
    let subfields_parse_detection_functions = join(
        subfields.iter().map(|subfield| {
            let subfield_name = snake_to_camel_case(&subfield.name);
            let parse_name = if BASE_TYPES.contains(&subfield.base_type.as_str()) {
                format!("parse_{}", subfield.base_type)
            } else {
                format!("{}::parse", snake_to_camel_case(&subfield.base_type))
            };
            let targets = subfield_targets(subfield);
            format!(
                "
|fields| {{
    // {subfield_name} subfield
    let targets: Vec<(FitMessage, DataValue)> = vec![{targets}];
    let found = fields.iter().find(|field| {{
        targets
            .iter()
            .any(|(msg, value)| &field.kind == msg && field.values.contains(value))
    }});

    match found {{
        Some(_) => Some(|reader, endianness, bytes_to_read| {{
            let value = {parse_name}(reader, endianness, bytes_to_read)?;
            Ok(DataMessageField {{
                kind: FitMessage::{message_name}({message_name}Field::{subfield_name}),
                values: value

            }})
        }}),
        None => {{ None }}
    }}
}}",
            )
        }),
        ",\n",
    );
    code.push_str(&format!(
        "
fn subfields_parse_functions() -> Vec<
    fn(&[DataMessageField]) -> Option<fn(&mut Reader, &Endianness, u8) -> Result<DataMessageField, DataTypeError>>
> {{
    vec![{subfields_parse_detection_functions}]
}}
    "
    ));

    // Close impl block
    code.push('}');

    Some(code)
}
