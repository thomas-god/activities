use std::path::Path;

use calamine::{Data, Reader, Xlsx, open_workbook};

use crate::types::{ENUMS_SKIPPED_VARIANTS, EnumName, EnumType, EnumVariant};

pub fn parse_enums(profile: &Path) -> Vec<(EnumName, EnumType, Vec<(usize, EnumVariant)>)> {
    let mut workbook: Xlsx<_> = open_workbook(profile).expect("Unable to load profile file");
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
