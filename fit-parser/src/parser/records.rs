use std::collections::HashMap;

use thiserror::Error;

use crate::parser::{
    definition::{Definition, custom::CustomDescription, parse_definition_message},
    reader::{Reader, ReaderError},
    types::{
        DataTypeError, DataValue,
        generated::{FitField, ParseFunction},
    },
};

#[derive(Debug)]
enum RecordHeader {
    Definition(DefinitionMessageHeader),
    Data(DataMessageHeader),
    Compressed(CompressedMessageHeader),
}

impl RecordHeader {
    fn from_byte(byte: u8) -> RecordHeader {
        let normal = (byte >> 7) & 1 == 0;
        let data = (byte >> 6) & 1 == 0;
        match (normal, data) {
            (true, false) => {
                let message_type_specific = ((byte >> 5) & 1) == 1;
                let local_message_type = byte & 0b1111;
                RecordHeader::Definition(DefinitionMessageHeader {
                    message_type_specific,
                    local_message_type,
                })
            }
            (true, true) => {
                let local_message_type = byte & 0b1111;
                RecordHeader::Data(DataMessageHeader { local_message_type })
            }
            (false, _) => RecordHeader::Compressed(CompressedMessageHeader {
                local_message_type: (byte >> 5 & 0b11),
                time_offset: (byte & 0b11111),
            }),
        }
    }
}
#[derive(Debug)]
pub struct DefinitionMessageHeader {
    pub message_type_specific: bool,
    pub local_message_type: u8,
}

#[derive(Debug)]
struct DataMessageHeader {
    local_message_type: u8,
}

#[derive(Debug)]
struct CompressedMessageHeader {
    local_message_type: u8,
    time_offset: u8,
}

#[derive(Error, Debug)]
pub enum RecordError {
    #[error("Error while trying to read bytes from content")]
    ReaderError(#[from] ReaderError),
    #[error("Record cannot be parsed")]
    InvalidRecord,
    #[error("No DefinitionMessage found for local id {0}")]
    NoDefinitionMessageFound(u8),
    #[error("No description found for developer data index {0} and field number {1}")]
    NoDescriptionFound(u8, u8),
    #[error("Invalid DataType")]
    DataTypeError(#[from] DataTypeError),
    #[error("No timestamp to rebuild compressed timestamp")]
    TimestampMissingForCompressedTimestamp,
    #[error("Trying to scale a value by")]
    ScaleByZeroError,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataMessage {
    pub local_message_type: u8,
    pub fields: Vec<DataMessageField>,
}

impl DataMessage {
    /// Extract the last (i.e. most recent) [u32] timestamp contains in all the fields and values of
    /// a [DataMessage].
    pub fn last_timestamp(&self) -> Option<u32> {
        let mut last_timestamp: Option<u32> = None;
        for field in self.fields.iter() {
            last_timestamp = field
                .values
                .iter()
                .fold(last_timestamp, |last, value| match value {
                    DataValue::DateTime(val) if *val >= last_timestamp.unwrap_or(0) => Some(*val),

                    _ => last,
                });
        }
        last_timestamp
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DataMessageField {
    pub kind: FitField,
    pub values: Vec<DataValue>,
}

#[derive(Debug)]
pub enum Record {
    Definition(Definition),
    Data(DataMessage),
}

impl Record {
    pub fn parse(
        content: &mut Reader,
        definitions: &HashMap<u8, Definition>,
        custom_descriptions: &HashMap<u8, HashMap<u8, CustomDescription>>,
        compressed_timestamp: &mut CompressedTimestamp,
    ) -> Result<Self, RecordError> {
        let header = RecordHeader::from_byte(content.next_u8()?);

        match header {
            RecordHeader::Data(header) => {
                parse_data_message(header, definitions, content).map(Record::Data)
            }

            RecordHeader::Definition(header) => {
                parse_definition_message(header, custom_descriptions, content)
                    .map(Record::Definition)
            }

            RecordHeader::Compressed(header) => {
                parse_compressed_message(header, definitions, compressed_timestamp, content)
                    .map(Record::Data)
            }
        }
    }
}

fn parse_data_message(
    header: DataMessageHeader,
    definitions: &HashMap<u8, Definition>,
    content: &mut Reader,
) -> Result<DataMessage, RecordError> {
    let Some(definition) = definitions.get(&header.local_message_type) else {
        return Err(RecordError::NoDefinitionMessageFound(
            header.local_message_type,
        ));
    };

    let mut fields = Vec::new();
    for field in definition.fields.iter() {
        let field = match field.parse {
            ParseFunction::Simple(parse) => DataMessageField {
                values: parse(content, &field.endianness, field.size)?
                    .iter()
                    .flat_map(|val| val.apply_scale_offset(&field.scale_offset))
                    .collect(),
                kind: field.kind.clone(),
            },

            ParseFunction::Dynamic(parse) => {
                parse(content, &field.endianness, field.size, &fields)?
            }
        };

        fields.push(field);
    }

    Ok(DataMessage {
        local_message_type: header.local_message_type,
        fields,
    })
}

fn parse_compressed_message(
    header: CompressedMessageHeader,
    definitions: &HashMap<u8, Definition>,
    compressed_timestamp: &mut CompressedTimestamp,
    content: &mut Reader,
) -> Result<DataMessage, RecordError> {
    let timestamp = compressed_timestamp
        .parse_offset(header.time_offset)
        .ok_or(RecordError::TimestampMissingForCompressedTimestamp)?;

    let Some(definition) = definitions.get(&header.local_message_type) else {
        return Err(RecordError::NoDefinitionMessageFound(
            header.local_message_type,
        ));
    };

    let mut fields = Vec::new();

    // Insert reconstructed timestamp
    if let Some(timestamp_field) = definition.message_type.timestamp_field() {
        fields.push(DataMessageField {
            kind: timestamp_field,
            values: vec![DataValue::DateTime(timestamp)],
        });
    }

    // Parse remaining fields
    for field in definition.fields.iter() {
        let field = match field.parse {
            ParseFunction::Simple(parse) => DataMessageField {
                values: parse(content, &field.endianness, field.size)?
                    .iter()
                    .flat_map(|val| val.apply_scale_offset(&field.scale_offset))
                    .collect(),
                kind: field.kind.clone(),
            },

            ParseFunction::Dynamic(parse) => {
                parse(content, &field.endianness, field.size, &fields)?
            }
        };

        fields.push(field);
    }

    Ok(DataMessage {
        local_message_type: header.local_message_type,
        fields,
    })
}

#[derive(Debug, Default)]
pub struct CompressedTimestamp {
    last_timestamp: Option<u32>,
}

impl CompressedTimestamp {
    pub fn set_last_timestamp(&mut self, new_timestamp: Option<u32>) {
        match (self.last_timestamp, new_timestamp) {
            (Some(last), Some(new)) if new > last => {
                self.last_timestamp = new_timestamp;
            }
            (None, Some(_)) => {
                self.last_timestamp = new_timestamp;
            }
            _ => {}
        }
    }
    pub fn parse_offset(&mut self, time_offset: u8) -> Option<u32> {
        // We compare the time_offset (5bits) to the 5 least significants bits of the last_timestamp
        // known. If they are lower then a rollover has happened and we add 0x20 to represent that.
        // In both cases we replace the 5 least significants bits of the last_timestamp with those of
        // the time_offset.
        let last_timestamp = self.last_timestamp?;
        let offset = time_offset as u32;
        let mut new_timestamp = (last_timestamp & 0xFFFFFFE0) + offset;
        if offset < (last_timestamp & 0x0000001F) {
            new_timestamp += 0x20;
        }

        self.last_timestamp = Some(new_timestamp);
        Some(new_timestamp)
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        FitEnum, MesgNum,
        parser::{
            Endianness,
            definition::DefinitionField,
            types::generated::{Event, EventField, EventFieldDataSubfield, RecordField},
        },
    };

    use super::*;

    #[test]
    fn test_data_message_contains_datetime() {
        let message_w_timestamp = DataMessage {
            local_message_type: 0,
            fields: vec![DataMessageField {
                kind: FitField::Record(RecordField::Timestamp),
                values: vec![DataValue::DateTime(0)],
            }],
        };

        assert!(message_w_timestamp.last_timestamp().is_some());
        assert_eq!(message_w_timestamp.last_timestamp().unwrap(), 0);
    }

    #[test]
    fn test_data_message_contains_multiple_datetimes() {
        let message_w_timestamp = DataMessage {
            local_message_type: 0,
            fields: vec![DataMessageField {
                kind: FitField::Record(RecordField::Timestamp),
                values: vec![DataValue::DateTime(0), DataValue::DateTime(3)],
            }],
        };

        assert!(message_w_timestamp.last_timestamp().is_some());
        assert_eq!(message_w_timestamp.last_timestamp().unwrap(), 3);
    }

    #[test]
    fn test_data_message_contains_multiple_fields_with_datetime() {
        let message_w_timestamp = DataMessage {
            local_message_type: 0,
            fields: vec![
                DataMessageField {
                    kind: FitField::Record(RecordField::Timestamp),
                    values: vec![DataValue::DateTime(16)],
                },
                DataMessageField {
                    kind: FitField::Record(RecordField::Timestamp),
                    values: vec![DataValue::DateTime(0), DataValue::DateTime(3)],
                },
            ],
        };

        assert!(message_w_timestamp.last_timestamp().is_some());
        assert_eq!(message_w_timestamp.last_timestamp().unwrap(), 16);
    }

    #[test]
    fn test_data_message_contains_no_timestamp() {
        let message_w_timestamp = DataMessage {
            local_message_type: 0,
            fields: vec![DataMessageField {
                kind: FitField::Record(RecordField::Timestamp),
                values: vec![DataValue::String("toto".to_string())],
            }],
        };

        assert!(message_w_timestamp.last_timestamp().is_none());
    }

    #[test]
    fn test_parse_data_message_with_dynamic_fields_and_scale_offset() {
        let header = DataMessageHeader {
            local_message_type: 0,
        };
        let mut definitions = HashMap::new();
        definitions.insert(
            0,
            Definition {
                message_type: MesgNum::Event,
                local_message_type: 0,
                fields: vec![
                    DefinitionField {
                        endianness: Endianness::Little,
                        kind: FitField::Event(EventField::Event),
                        parse: ParseFunction::Simple(Event::parse),
                        scale_offset: None,
                        size: 1,
                    },
                    DefinitionField {
                        endianness: Endianness::Little,
                        kind: FitField::Event(EventField::Data), // Subfield will depend on the value taken by EventField::Event
                        parse: ParseFunction::Dynamic(EventFieldDataSubfield::parse),
                        scale_offset: None,
                        size: 4,
                    },
                ],
            },
        );

        let mut content = Vec::new();
        content.push(15); // event = SpeedHighAlert
        content.append(&mut 51_u32.to_le_bytes().to_vec()); // speed_high_alert = 51

        let mut reader = Reader::new(5, content.into_iter());

        let message = parse_data_message(header, &definitions, &mut reader).unwrap();

        assert_eq!(
            *message.fields.first().unwrap(),
            DataMessageField {
                kind: FitField::Event(EventField::Event),
                values: vec![DataValue::Enum(FitEnum::Event(Event::SpeedHighAlert))]
            }
        );
        assert_eq!(
            *message.fields.get(1).unwrap(),
            DataMessageField {
                kind: FitField::Event(EventField::SpeedHighAlert),
                values: vec![DataValue::Float32(0.051)] // Scale of 1000
            }
        );
    }

    #[test]
    fn test_parse_data_message_with_dynamic_fields_default_parse() {
        let header = DataMessageHeader {
            local_message_type: 0,
        };
        let mut definitions = HashMap::new();
        definitions.insert(
            0,
            Definition {
                message_type: MesgNum::Event,
                local_message_type: 0,
                fields: vec![
                    DefinitionField {
                        endianness: Endianness::Little,
                        kind: FitField::Event(EventField::Event),
                        parse: ParseFunction::Simple(Event::parse),
                        scale_offset: None,
                        size: 1,
                    },
                    DefinitionField {
                        endianness: Endianness::Little,
                        kind: FitField::Event(EventField::Data), // Subfield will depend on the value taken by EventField::Event
                        parse: ParseFunction::Dynamic(EventFieldDataSubfield::parse),
                        scale_offset: None,
                        size: 4,
                    },
                ],
            },
        );

        let mut content = Vec::new();
        content.push(72); // event = TankPressureCritical
        content.append(&mut 51_u32.to_le_bytes().to_vec());

        let mut reader = Reader::new(5, content.into_iter());

        let message = parse_data_message(header, &definitions, &mut reader).unwrap();

        assert_eq!(
            *message.fields.first().unwrap(),
            DataMessageField {
                kind: FitField::Event(EventField::Event),
                values: vec![DataValue::Enum(FitEnum::Event(Event::TankPressureCritical))]
            }
        );
        assert_eq!(
            *message.fields.get(1).unwrap(),
            DataMessageField {
                kind: FitField::Event(EventField::Data),
                values: vec![DataValue::Uint32(51)]
            }
        );
    }
}

#[cfg(test)]
mod tests_compressed_timestamp {

    use super::*;

    #[test]
    fn test_offset_greater_than_previous_timestamp_part() {
        let mut compressed = CompressedTimestamp::default();

        assert!(compressed.parse_offset(0b11011).is_none());

        compressed.set_last_timestamp(Some(0x1111113B));

        assert_eq!(compressed.parse_offset(0b11011), Some(0x1111113B));
        assert_eq!(compressed.parse_offset(0b11101), Some(0x1111113D));
        assert_eq!(compressed.parse_offset(0b00010), Some(0x11111142));
        assert_eq!(compressed.parse_offset(0b00101), Some(0x11111145));
        assert_eq!(compressed.parse_offset(0b00001), Some(0x11111161));

        compressed.set_last_timestamp(Some(0x11111163));
        assert_eq!(compressed.parse_offset(0b10010), Some(0x11111172));
        assert_eq!(compressed.parse_offset(0b00001), Some(0x11111181));
    }
}
