#![allow(clippy::const_is_empty)]
#![allow(clippy::type_complexity)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::messages::{generate_messages_code, parse_messages_definitions};
use crate::types::{generate_enums_code, parse_enums};

mod messages;
mod types;

const MESSAGES_TO_IMPORT: &[&str] = &[]; // If empty, every message type is imported
// const MESSAGES_TO_IMPORT: &[&str] = &["Record", "FieldDescription", "DeviceInfo"];
const BASE_TYPES: &[&str] = &[
    "sint8", "uint8", "uint8z", "sint16", "uint16", "uint16z", "sint32", "uint32", "uint32z",
    "sint64", "uint64", "uint64z", "string", "float32", "float64", "byte",
];

pub fn generate_code(profile: &Path) -> String {
    let mut enums = parse_enums(profile);
    let (messages, enums_used) = parse_messages_definitions(profile);

    enums.retain(|(name, _, __)| enums_used.contains(name));

    let enums_names = enums.iter().map(|(name, _, __)| name.clone()).collect();

    let mut code = generate_enums_code(&enums);
    code.push_str(&generate_messages_code(messages, enums_names));
    code = format_code(&code);

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
