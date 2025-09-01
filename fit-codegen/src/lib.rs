#![allow(clippy::const_is_empty)]
#![allow(clippy::type_complexity)]

use std::path::Path;

use crate::messages::{generate_messages_code, parse_messages_definitions};
use crate::types::{generate_enums_code, parse_enums};
use crate::utils::format_code;

mod messages;
mod types;
mod utils;

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
