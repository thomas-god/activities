use thiserror::Error;

use crate::{
    DataMessage, DataValue, FitParserError, parse_fit_messages,
    parser::types::generated::{DeviceInfoField, FitMessage, RecordField},
};

#[derive(Debug, Default)]
pub struct Activity {
    pub metadata: ActivityMetadata,
    pub values: Vec<DataValue>,
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

impl Activity {
    pub fn from_file(file: &str) -> Result<Self, ParseActivityError> {
        let records = parse_fit_messages(file)?;
        Ok(Self::from_records(&records))
    }
    pub fn from_records(records: &[DataMessage]) -> Self {
        // Metadata
        let mut metadata = None;
        for record in records.iter() {
            if let Some(product_name) = find_product_name(record) {
                metadata = Some(ActivityMetadata {
                    product_name: Some(product_name),
                });
            }
        }

        // Values
        let mut values = Vec::new();
        let reader = MessagesReader::new(records);
        for val in reader {
            values.push(val.clone());
        }

        Self {
            metadata: metadata.unwrap_or_default(),
            values,
        }
    }
}

fn find_product_name(message: &DataMessage) -> Option<String> {
    for field in message.fields.iter() {
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

struct MessagesReader<'a> {
    messages_iterator: std::slice::Iter<'a, DataMessage>,
    current_values: Option<Vec<&'a DataValue>>,
}

impl<'a> MessagesReader<'a> {
    pub fn new(content: &'a [DataMessage]) -> Self {
        Self {
            messages_iterator: content.iter(),
            current_values: None,
        }
    }
}

impl<'a> Iterator for MessagesReader<'a> {
    type Item = &'a DataValue;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.current_values.as_mut().and_then(|val| val.pop()) {
            return Some(value);
        };

        loop {
            let msg = self.messages_iterator.next()?;

            // Parse fields and search for timestamp
            let mut values = Vec::new();
            for field in msg.fields.iter() {
                match field.kind {
                    FitMessage::Record(RecordField::Timestamp) => {
                        // timestamp = Some(field.values.first().unwrap());
                    }
                    _ => values.push(field.values.first().unwrap()),
                }
            }

            // Pop last value to return and save the others
            if !values.is_empty() {
                let last_value = values.pop();
                self.current_values = Some(values);
                return last_value;
            }
        }
    }
}

#[cfg(test)]
mod test {

    use crate::{DataMessage, DataMessageField, parser::types::generated::RecordField};

    use super::*;

    #[test]
    fn test() {
        let records = vec![DataMessage {
            local_message_type: 0,
            fields: vec![
                DataMessageField {
                    kind: FitMessage::Record(RecordField::Timestamp),
                    values: vec![DataValue::DateTime(1000)],
                },
                DataMessageField {
                    kind: FitMessage::Record(RecordField::Speed),
                    values: vec![DataValue::Float32(9.8)],
                },
            ],
        }];

        let activity = Activity::from_records(&records);

        assert_eq!(activity.values.len(), 1);
        assert_eq!(activity.values.first().unwrap(), &DataValue::Float32(9.8));
    }
}
