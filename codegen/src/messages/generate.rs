use std::collections::HashMap;

use itertools::join;

use crate::{
    BASE_TYPES,
    messages::{Field, Subfield},
    snake_to_camel_case,
};

pub fn generate_messages_code(
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
    let parent_scale_offset = match (field.scale, field.offset) {
        (None, None) => "None".to_string(),
        (scale, offset) => format!(
            "Some(ScaleOffset {{scale: {}_f32, offset: {}_f32}})",
            scale.unwrap_or(1.),
            offset.unwrap_or(0.)
        ),
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
        let values = {parent_field_parse}(reader, endianness, bytes_to_read)?
            .iter()
            .flat_map(|val| val.apply_scale_offset(&{parent_scale_offset}))
            .collect();
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
            let scale_offset = match (subfield.scale, subfield.offset) {
                (None, None) => "None".to_string(),
                (scale, offset) => format!(
                    "Some(ScaleOffset {{scale: {}_f32, offset: {}_f32}})",
                    scale.unwrap_or(1.),
                    offset.unwrap_or(0.)
                ),
            };
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
            let values = {parse_name}(reader, endianness, bytes_to_read)?
                .iter()
                .flat_map(|val| val.apply_scale_offset(&{scale_offset}))
                .collect();

            Ok(DataMessageField {{
                kind: FitMessage::{message_name}({message_name}Field::{subfield_name}),
                values

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

fn get_parse_function(enums: &[String], type_name: &str) -> String {
    if BASE_TYPES.contains(&type_name) {
        return format!("parse_{}", type_name).to_string();
    }

    if enums.contains(&type_name.to_string()) {
        return format!("{}::parse", snake_to_camel_case(type_name)).to_string();
    }

    "parse_unknown".to_string()
}
