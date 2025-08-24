use thiserror::Error;

use crate::{
    DataValue, FitParserError, Record, parse_records,
    parser::types::generated::{DeviceInfoField, FitMessage},
};

#[derive(Debug, Default)]
pub struct Activity {
    pub metadata: ActivityMetadata,
}

#[derive(Debug, Default)]
pub struct ActivityMetadata {
    pub product_name: Option<String>,
}

#[derive(Debug, Error)]
pub enum ParseActivityError {
    #[error("Unabale to parse .fit file")]
    ParserError(#[from] FitParserError),
}

pub fn parse_activity(file: &str) -> Result<Activity, ParseActivityError> {
    let records = parse_records(file)?;

    for record in records.iter() {
        if let Some(product_name) = find_product_name(record) {
            return Ok(Activity {
                metadata: ActivityMetadata {
                    product_name: Some(product_name),
                },
            });
        }
    }

    Ok(Activity::default())
}

fn find_product_name(record: &Record) -> Option<String> {
    let Record::Data(data) = record else {
        return None;
    };

    for field in data.fields.iter() {
        if let FitMessage::DeviceInfo(DeviceInfoField::ProductName) = field.kind {
            return field
                .values
                .iter()
                .filter_map(|val| match val {
                    DataValue::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect::<Vec<String>>()
                .first()
                .cloned();
        }
    }

    None
}
