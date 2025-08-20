use thiserror::Error;

use crate::parser::{
    definition::Endianness,
    types::{
        developer::DeveloperDataIdField, field_description::FieldDescriptionField,
        file_id::FileIdField, record::RecordField,
    },
};

pub mod developer;
pub mod field_description;
pub mod file_id;
pub mod record;

#[derive(Debug, Clone)]
pub enum DataField {
    FileId(FileIdField),
    Record(RecordField),
    FieldDescription(FieldDescriptionField),
    DeveloperDataId(DeveloperDataIdField),
    Custom(CustomField),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CustomField {
    pub name: Option<String>,
    pub units: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataType {
    Enum,
    Sint8,
    Uint8,
    Sint16,
    Uint16,
    Sint32,
    Uint32,
    String,
    Float32,
    Float64,
    Uint8z,
    Uint16z,
    Uint32z,
    Byte,
    Sint64,
    Uint64,
    Uint64z,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    Enum(u8),
    Sint8(i8),
    Uint8(u8),
    Sint16(i16),
    Uint16(u16),
    Sint32(i32),
    Uint32(u32),
    String(String),
    Float32(f32),
    Float64(f64),
    Uint8z(u8),
    Uint16z(u16),
    Uint32z(u32),
    Byte(Vec<u8>),
    Sint64(i64),
    Uint64(u64),
    Uint64z(u64),
    Unknown,
}

#[derive(Debug, Error)]
pub enum DataTypeError {
    #[error("Not enough bytes to parse data")]
    InsufficientData,
    #[error("Unable to parse Utf-8 String from bytes")]
    InvalidUtf8,
}

impl DataType {
    /// Parse the enum variant from the base type field value
    pub fn from_base_type_field(base_type_field: u8) -> Result<Self, DataTypeError> {
        match base_type_field {
            0x00 => Ok(DataType::Enum),
            0x01 => Ok(DataType::Sint8),
            0x02 => Ok(DataType::Uint8),
            0x83 => Ok(DataType::Sint16),
            0x84 => Ok(DataType::Uint16),
            0x85 => Ok(DataType::Sint32),
            0x86 => Ok(DataType::Uint32),
            0x07 => Ok(DataType::String),
            0x88 => Ok(DataType::Float32),
            0x89 => Ok(DataType::Float64),
            0x0A => Ok(DataType::Uint8z),
            0x8B => Ok(DataType::Uint16z),
            0x8C => Ok(DataType::Uint32z),
            0x0D => Ok(DataType::Byte),
            0x8E => Ok(DataType::Sint64),
            0x8F => Ok(DataType::Uint64),
            0x90 => Ok(DataType::Uint64z),
            _ => Ok(DataType::Unknown),
        }
    }

    /// Get the size in bytes for this data type
    fn size_bytes(&self) -> u8 {
        match self {
            DataType::Enum => 1,
            DataType::Sint8 => 1,
            DataType::Uint8 => 1,
            DataType::Sint16 => 2,
            DataType::Uint16 => 2,
            DataType::Sint32 => 4,
            DataType::Uint32 => 4,
            DataType::String => 1, // Minimum size, actual size depends on content
            DataType::Float32 => 4,
            DataType::Float64 => 8,
            DataType::Uint8z => 1,
            DataType::Uint16z => 2,
            DataType::Uint32z => 4,
            DataType::Byte => 1, // Minimum size, actual size depends on content
            DataType::Sint64 => 8,
            DataType::Uint64 => 8,
            DataType::Uint64z => 8,
            DataType::Unknown => 1, // 1 to allways parse the number of bytes requested
        }
    }

    pub fn parse_values<I>(
        &self,
        content: &mut I,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError>
    where
        I: Iterator<Item = u8>,
    {
        if number_of_bytes % self.size_bytes() != 0 {
            return Err(DataTypeError::InsufficientData);
        }
        let number_of_values = number_of_bytes / self.size_bytes();
        let mut values = Vec::new();

        match self {
            DataType::Enum => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Enum(
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ));
                }
            }

            DataType::Sint8 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Sint8(
                        content.next().ok_or(DataTypeError::InsufficientData)? as i8,
                    ));
                }
            }

            DataType::Uint8 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint8(
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ));
                }
            }

            DataType::Uint8z => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint8z(
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ));
                }
            }

            DataType::Sint16 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Sint16(match endianness {
                        Endianness::Little => i16::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => i16::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    }));
                }
            }

            DataType::Uint16 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint16(match endianness {
                        Endianness::Little => u16::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => u16::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    }));
                }
            }

            DataType::Uint16z => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint16z(match endianness {
                        Endianness::Little => u16::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => u16::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    }));
                }
            }

            DataType::Sint32 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Sint32(match endianness {
                        Endianness::Little => i32::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => i32::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    }));
                }
            }

            DataType::Uint32 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint32(match endianness {
                        Endianness::Little => u32::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => u32::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    }));
                }
            }

            DataType::Uint32z => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint32z(match endianness {
                        Endianness::Little => u32::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => u32::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    }));
                }
            }

            DataType::Sint64 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Sint64(match endianness {
                        Endianness::Little => i64::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => i64::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    }));
                }
            }

            DataType::Uint64 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint64(match endianness {
                        Endianness::Little => u64::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => u64::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    }));
                }
            }

            DataType::Uint64z => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint64z(match endianness {
                        Endianness::Little => u64::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => u64::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    }));
                }
            }

            DataType::Float32 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Float32(f32::from_bits(match endianness {
                        Endianness::Little => u32::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => u32::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    })));
                }
            }

            DataType::Float64 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Float64(f64::from_bits(match endianness {
                        Endianness::Little => u64::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                        Endianness::Big => u64::from_be_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    })));
                }
            }

            DataType::String => {
                let mut bytes = Vec::new();
                for _ in 0..number_of_bytes {
                    bytes.push(content.next().ok_or(DataTypeError::InsufficientData)?)
                }
                values.push(DataValue::String(
                    String::from_utf8(bytes).map_err(|_| DataTypeError::InvalidUtf8)?,
                ));
            }

            DataType::Byte => {
                let mut bytes = Vec::new();
                for _ in 0..number_of_values {
                    bytes.push(content.next().ok_or(DataTypeError::InsufficientData)?);
                }
                values.push(DataValue::Byte(bytes));
            }

            DataType::Unknown => {
                // Just consume the number of bytes from the iterator
                for _ in 0..number_of_values {
                    let _ = content.next();
                }
            }
        };
        Ok(values)
    }
}

impl DataValue {
    /// Check if a value should be considered invalid as per the .FIT protocol. Notable exceptions
    /// are:
    ///
    /// - [DataValue::String] are always considered valid, event if empty,
    /// - [DataValue::Unknown] are always considerred invalid.
    pub fn is_invalid(&self) -> bool {
        match self {
            Self::Enum(val) => *val == 0xFF,
            Self::Sint8(val) => *val == 0x7F,
            Self::Sint16(val) => *val == 0x7FFF,
            Self::Sint32(val) => *val == 0x7FFFFFFF,
            Self::Sint64(val) => *val == 0x7FFFFFFFFFFFFFFF,
            Self::Uint8(val) => *val == 0xFF,
            Self::Uint16(val) => *val == 0xFFFF,
            Self::Uint32(val) => *val == 0xFFFFFFFF,
            Self::Uint64(val) => *val == 0xFFFFFFFFFFFFFFFF,
            Self::Uint8z(val) => *val == 0x00,
            Self::Uint16z(val) => *val == 0x0000,
            Self::Uint32z(val) => *val == 0x00000000,
            Self::Uint64z(val) => *val == 0x0000000000000000,
            Self::Float32(val) => val.to_le_bytes() == [0xFF, 0xFF, 0xFF, 0xFF],
            Self::Float64(val) => {
                val.to_le_bytes() == [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
            }
            Self::Byte(val) => val.iter().all(|b| *b == 0xFF),
            Self::String(_) => false,
            Self::Unknown => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::DataValue;

    #[test]
    fn test_data_value_enum_invalid() {
        assert!(!DataValue::Enum(0).is_invalid());
        assert!(DataValue::Enum(0xFF).is_invalid());
    }

    #[test]
    fn test_data_value_sint_invalid() {
        assert!(!DataValue::Sint8(0).is_invalid());
        assert!(DataValue::Sint8(0x7F).is_invalid());

        assert!(!DataValue::Sint16(0).is_invalid());
        assert!(DataValue::Sint16(0x7FFF).is_invalid());

        assert!(!DataValue::Sint32(0).is_invalid());
        assert!(DataValue::Sint32(0x7FFFFFFF).is_invalid());

        assert!(!DataValue::Sint64(0).is_invalid());
        assert!(DataValue::Sint64(0x7FFFFFFFFFFFFFFF).is_invalid());
    }

    #[test]
    fn test_data_value_uint_invalid() {
        assert!(!DataValue::Uint8(0).is_invalid());
        assert!(DataValue::Uint8(0xFF).is_invalid());

        assert!(!DataValue::Uint16(0).is_invalid());
        assert!(DataValue::Uint16(0xFFFF).is_invalid());

        assert!(!DataValue::Uint32(0).is_invalid());
        assert!(DataValue::Uint32(0xFFFFFFFF).is_invalid());

        assert!(!DataValue::Uint64(0).is_invalid());
        assert!(DataValue::Uint64(0xFFFFFFFFFFFFFFFF).is_invalid());
    }

    #[test]
    fn test_data_value_uintz_invalid() {
        assert!(!DataValue::Uint8z(0xFF).is_invalid());
        assert!(DataValue::Uint8z(0).is_invalid());

        assert!(!DataValue::Uint16z(0xFFFF).is_invalid());
        assert!(DataValue::Uint16z(0).is_invalid());

        assert!(!DataValue::Uint32z(0xFFFFFFFF).is_invalid());
        assert!(DataValue::Uint32z(0).is_invalid());

        assert!(!DataValue::Uint64z(0xFFFFFFFFFFFFFFFF).is_invalid());
        assert!(DataValue::Uint64z(0).is_invalid());
    }

    #[test]
    fn test_data_value_float_invalid() {
        assert!(!DataValue::Float32(f32::from_le_bytes([0x00, 0x00, 0x00, 0x00])).is_invalid());
        assert!(DataValue::Float32(f32::from_le_bytes([0xFF, 0xFF, 0xFF, 0xFF])).is_invalid());

        assert!(
            !DataValue::Float64(f64::from_le_bytes([
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ]))
            .is_invalid()
        );
        assert!(
            DataValue::Float64(f64::from_le_bytes([
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
            ]))
            .is_invalid()
        );
    }

    #[test]
    fn test_data_value_byte_invalid() {
        assert!(!DataValue::Byte(vec![0x00, 0x00]).is_invalid());
        assert!(DataValue::Byte(vec![0xFF, 0xFF]).is_invalid());
    }

    #[test]
    fn test_data_value_string_always_valid() {
        assert!(!DataValue::String(String::from_utf8(vec![]).unwrap()).is_invalid());
        assert!(!DataValue::String(String::from_utf8(vec![0x00]).unwrap()).is_invalid());
        assert!(!DataValue::String(String::from_utf8(vec![0x01, 0x00]).unwrap()).is_invalid());
    }

    #[test]
    fn test_data_value_unknown_always_invalid() {
        assert!(DataValue::Unknown.is_invalid());
    }
}
