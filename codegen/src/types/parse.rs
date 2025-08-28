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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_enum_row_numeric_field_cell_is_int() {
        let row = vec![Data::Empty, Data::Empty, Data::Empty, Data::Int(12)];
        let parsed_row = parse_enum_row(&row);

        assert_eq!(parsed_row.variant_value, Some(12))
    }

    #[test]
    fn test_parse_enum_row_numeric_field_cell_is_float() {
        let row = vec![Data::Empty, Data::Empty, Data::Empty, Data::Float(12.)];
        let parsed_row = parse_enum_row(&row);

        assert_eq!(parsed_row.variant_value, Some(12))
    }

    #[test]
    fn test_parse_enum_row_numeric_field_cell_is_base16_string() {
        let row = vec![
            Data::Empty,
            Data::Empty,
            Data::Empty,
            Data::String("0x20".to_string()),
        ];
        let parsed_row = parse_enum_row(&row);

        assert_eq!(parsed_row.variant_value, Some(32))
    }

    #[test]
    fn test_parse_enum_variants_no_enum_after() {
        let first_row: &[Data] = &[
            Data::Empty,
            Data::Empty,
            Data::String("variant_1".to_string()),
            Data::Int(0),
        ];
        let second_row: &[Data] = &[
            Data::Empty,
            Data::Empty,
            Data::String("variant_2".to_string()),
            Data::Int(12),
        ];
        let content = vec![first_row, second_row];
        let mut iter = content.into_iter();

        let (variants, next_enum, next_enum_type) = parse_enum_variants(&mut iter);

        assert!(next_enum.is_none());
        assert!(next_enum_type.is_none());

        assert_eq!(
            variants,
            vec![(0, "variant_1".to_string()), (12, "variant_2".to_string())]
        );
    }

    #[test]
    fn test_parse_enum_variants_with_enum_after() {
        let first_row: &[Data] = &[
            Data::Empty,
            Data::Empty,
            Data::String("variant_1".to_string()),
            Data::Int(0),
        ];
        let second_row: &[Data] = &[
            Data::Empty,
            Data::Empty,
            Data::String("variant_2".to_string()),
            Data::Int(12),
        ];
        let new_enum_row: &[Data] = &[
            Data::String("new_enum".to_string()),
            Data::String("new enum type".to_string()),
            Data::Empty,
            Data::Empty,
        ];

        let content = vec![first_row, second_row, new_enum_row];
        let mut iter = content.into_iter();

        let (variants, next_enum, next_enum_type) = parse_enum_variants(&mut iter);

        assert_eq!(next_enum, Some("new_enum".to_string()));
        assert_eq!(next_enum_type, Some("new enum type".to_string()));

        assert_eq!(
            variants,
            vec![(0, "variant_1".to_string()), (12, "variant_2".to_string())]
        );
    }

    #[test]
    fn test_parse_enum_variants_skip_variants() {
        let first_row: &[Data] = &[
            Data::Empty,
            Data::Empty,
            Data::String("variant_1".to_string()),
            Data::Int(0),
        ];
        let second_row: &[Data] = &[
            Data::Empty,
            Data::Empty,
            Data::String(ENUMS_SKIPPED_VARIANTS[0].to_string()),
            Data::Int(12),
        ];
        let content = vec![first_row, second_row];
        let mut iter = content.into_iter();

        let (variants, _next_enum, _next_enum_type) = parse_enum_variants(&mut iter);

        assert_eq!(variants, vec![(0, "variant_1".to_string())]);
    }
}
