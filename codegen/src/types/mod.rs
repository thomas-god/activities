mod generate;
mod parse;

pub use generate::generate_enums_code;
pub use parse::parse_enums;

pub type EnumName = String;
pub type EnumVariant = String;
pub type EnumType = String;

const ENUMS_SKIPPED_VARIANTS: &[&str] = &["mfg_range_min", "mfg_range_max", "pad"];
