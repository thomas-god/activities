use std::collections::HashMap;

use thiserror::Error;

use crate::parser::{
    definition::{Definition, custom::CustomDescription, parse_definition_message},
    reader::{Reader, ReaderError},
    types::{DataTypeError, DataValue, generated::FitMessage},
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
}

#[derive(Debug)]
pub struct DataMessage {
    pub local_message_type: u8,
    pub fields: Vec<DataMessageField>,
}

impl DataMessage {
    /// Extract the last (i.e. most recent) [u32] timestamp contains in all the fields and values of
    /// a [DataMessage].
    pub fn last_timestamp(&self) -> Option<u32> {
        Some(0)
        // let mut last_timestamp: Option<u32> = None;
        // for field in self.fields.iter() {
        //     if discriminant(&field.kind) == discriminant(&DataField::Timestamp) {
        //         last_timestamp =
        //             field
        //                 .values
        //                 .iter()
        //                 .fold(last_timestamp, |last, value| match value {
        //                     BaseDataValue::Uint32(val) if *val >= last_timestamp.unwrap_or(0) => {
        //                         Some(*val)
        //                     }
        //                     _ => last,
        //                 });
        //     }
        // }
        // last_timestamp
    }
}

#[derive(Debug)]
pub struct DataMessageField {
    pub kind: FitMessage,
    pub values: Vec<DataValue>,
}

#[derive(Debug)]
pub struct CompressedTimestampMessage {
    pub timestamp: u32,
    pub local_message_type: u8,
    pub values: Vec<u8>,
}

#[derive(Debug)]
pub enum Record {
    Definition(Definition),
    Data(DataMessage),
    CompressedTimestamp(CompressedTimestampMessage),
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
            }
        }
    }
}

fn parse_data_message(
    header: DataMessageHeader,
    definitions: &HashMap<u8, Definition>,
    content: &mut Reader,
) -> Result<DataMessage, RecordError> {
    match definitions.get(&header.local_message_type) {
        Some(definition) => {
            let mut fields = Vec::new();
            for field in definition.fields.iter() {
                let values = (field.parse)(content, &field.endianness, field.size)?;
                fields.push(DataMessageField {
                    kind: field.kind.clone(),
                    values,
                })
            }

            Ok(DataMessage {
                local_message_type: header.local_message_type,
                fields,
            })
        }

        None => Err(RecordError::NoDefinitionMessageFound(
            header.local_message_type,
        )),
    }
}

fn parse_compressed_message(
    header: CompressedMessageHeader,
    definitions: &HashMap<u8, Definition>,
    compressed_timestamp: &mut CompressedTimestamp,
    content: &mut Reader,
) -> Result<Record, RecordError> {
    let timestamp = compressed_timestamp
        .parse_offset(header.time_offset)
        .ok_or(RecordError::TimestampMissingForCompressedTimestamp)?;
    let fields_size = match definitions.get(&header.local_message_type) {
        Some(definition) => definition.fields_size,
        None => {
            return Err(RecordError::NoDefinitionMessageFound(
                header.local_message_type,
            ));
        }
    };
    let mut values: Vec<u8> = Vec::new();
    for _ in 0..fields_size {
        values.push(content.next_u8()?);
    }
    Ok(Record::CompressedTimestamp(CompressedTimestampMessage {
        timestamp,
        local_message_type: header.local_message_type,
        values,
    }))
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

// #[cfg(test)]
// mod tests {

//     use super::*;

//     #[test]
//     fn test_data_message_contains_u32_timestamp() {
//         let message_w_timestamp = DataMessage {
//             local_message_type: 0,
//             fields: vec![DataMessageField {
//                 kind: DataField::Timestamp,
//                 values: vec![BaseDataValue::Uint32(0)],
//             }],
//         };

//         assert!(message_w_timestamp.last_timestamp().is_some());
//         assert_eq!(message_w_timestamp.last_timestamp().unwrap(), 0);
//     }

//     #[test]
//     fn test_data_message_contains_multiple_u32_timestamps() {
//         let message_w_timestamp = DataMessage {
//             local_message_type: 0,
//             fields: vec![DataMessageField {
//                 kind: DataField::Timestamp,
//                 values: vec![BaseDataValue::Uint32(0), BaseDataValue::Uint32(3)],
//             }],
//         };

//         assert!(message_w_timestamp.last_timestamp().is_some());
//         assert_eq!(message_w_timestamp.last_timestamp().unwrap(), 3);
//     }

//     #[test]
//     fn test_data_message_contains_multiple_fields_with_u32_timestamps() {
//         let message_w_timestamp = DataMessage {
//             local_message_type: 0,
//             fields: vec![
//                 DataMessageField {
//                     kind: DataField::Timestamp,
//                     values: vec![BaseDataValue::Uint32(16)],
//                 },
//                 DataMessageField {
//                     kind: DataField::Timestamp,
//                     values: vec![BaseDataValue::Uint32(0), BaseDataValue::Uint32(3)],
//                 },
//             ],
//         };

//         assert!(message_w_timestamp.last_timestamp().is_some());
//         assert_eq!(message_w_timestamp.last_timestamp().unwrap(), 16);
//     }

//     #[test]
//     fn test_data_message_contains_timestamp_but_not_u32() {
//         let message_w_timestamp = DataMessage {
//             local_message_type: 0,
//             fields: vec![DataMessageField {
//                 kind: DataField::Timestamp,
//                 values: vec![BaseDataValue::String("toto".to_string())],
//             }],
//         };

//         assert!(message_w_timestamp.last_timestamp().is_none());
//     }

//     #[test]
//     fn test_data_message_contains_no_timestamp() {
//         let message_w_timestamp = DataMessage {
//             local_message_type: 0,
//             fields: vec![DataMessageField {
//                 kind: DataField::Unknown,
//                 values: vec![BaseDataValue::String("toto".to_string())],
//             }],
//         };

//         assert!(message_w_timestamp.last_timestamp().is_none());
//     }
// }

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
