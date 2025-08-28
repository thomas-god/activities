mod generate;
mod parse;

pub use generate::generate_messages_code;
pub use parse::parse_messages_definitions;

#[derive(Debug)]
pub struct Field {
    field_def: u8,
    name: String,
    base_type: String,
    // array: Option<usize>,
    scale: Option<f32>,
    offset: Option<f32>,
}

#[derive(Debug)]
pub struct Subfield {
    name: String,
    base_type: String,
    references: Vec<SubfieldReference>,
    scale: Option<f32>,
    offset: Option<f32>,
}

#[derive(Debug)]
pub struct SubfieldReference {
    name: String,
    value: String,
    base_type: Option<String>,
}
