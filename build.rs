use itertools::join;
use std::io::Write;
use std::{
    collections::HashMap,
    process::{Command, Stdio},
};

use calamine::Data;

fn main() {
    let enums = generate_enums();
    let code = format_code(&enums);
    std::fs::write("src/parser/types/generated.rs", code).expect("Could not wirte to ouptut file");
}

fn generate_enums() -> String {
    use calamine::{Reader, Xlsx, open_workbook};

    let mut workbook: Xlsx<_> = open_workbook("Profile.xlsx").expect("Unable to load profile file");
    let range = workbook
        .worksheet_range("Types")
        .expect("Profile file does not contain a Types sheet");

    let mut iterator = range.rows();
    let _ = iterator.next(); // Skip header

    let mut enums: Vec<(String, String, HashMap<usize, String>)> = Vec::new();

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

    generate_enums_code(enums)
}

#[derive(Debug)]
struct EnumRow {
    name: Option<String>,
    enum_type: Option<String>,
    variant_name: Option<String>,
    value: Option<usize>,
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
        value: match row.get(3) {
            Some(Data::Int(variant)) => Some(*variant as usize),
            Some(Data::Float(variant)) => Some(*variant as usize),
            _ => None,
        },
    }
}

fn parse_enum_variants<'a, I>(
    iterator: &mut I,
) -> (HashMap<usize, String>, Option<String>, Option<String>)
where
    I: Iterator<Item = &'a [Data]>,
{
    let mut mapping = HashMap::new();
    let mut next_type_name = None;
    let mut next_base_type = None;

    for row in iterator {
        let row = parse_enum_row(row);

        if row.name.is_some() && row.enum_type.is_some() {
            next_type_name = row.name;
            next_base_type = row.enum_type;
            break;
        }

        if row.variant_name.is_some() && row.value.is_some() {
            mapping.insert(row.value.unwrap(), row.variant_name.unwrap());
        }
    }

    (mapping, next_type_name, next_base_type)
}

fn map_type(val: &str) -> Option<String> {
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

fn generate_enums_code(enums: Vec<(String, String, HashMap<usize, String>)>) -> String {
    let mut code = String::new();
    code.push_str("#![allow(dead_code)]\n");
    code.push_str("#![allow(clippy::enum_variant_names)]\n");
    code.push_str("#![allow(clippy::upper_case_acronyms)]\n");

    let main_enum = join(enums.iter().map(|(e, _, __)| snake_to_camel_case(e)), ",\n");

    code.push_str(&format!(
        r#"
#[derive(Debug, PartialEq)]
pub enum FitEnum {{
    {main_enum}
}}
"#
    ));

    for (name, base_type, mapping) in enums.iter() {
        if mapping.is_empty() {
            continue;
        }

        let name = snake_to_camel_case(name);
        let variants = join(mapping.values().map(|v| snake_to_camel_case(v)), ",\n");
        let enum_mapping = join(
            mapping
                .iter()
                .map(|(k, v)| format!("{k} => {name}::{}", snake_to_camel_case(v))),
            ",\n",
        );
        let enum_type = map_type(base_type).expect("Expected not None enum type");

        code.push_str(&format!(
            "
#[derive(Debug, PartialEq)]
pub enum {name} {{
    {variants},
    UnknownVariant
}}"
        ));

        code.push_str(&format!(
            "
impl {name} {{
    pub fn from(content: {enum_type}) -> {name} {{
        match content {{
            {enum_mapping},
            _ => {name}::UnknownVariant
        }}
    }}
}}
        "
        ));
    }

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
