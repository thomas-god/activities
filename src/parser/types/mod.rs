use thiserror::Error;

use crate::parser::types::{file_id::FileIdField, record::RecordField};

pub mod file_id;
pub mod record;

#[derive(Debug, Clone, Copy)]
pub enum DataField {
    FileId(FileIdField),
    Record(RecordField),
    Unknown,
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
    #[error("Base type {0} is unknown")]
    UnknownBaseType(u8),
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
                    values.push(DataValue::Sint16(i16::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            DataType::Uint16 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint16(u16::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            DataType::Uint16z => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint16z(u16::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            DataType::Sint32 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Sint32(i32::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            DataType::Uint32 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint32(u32::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            DataType::Uint32z => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint32z(u32::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            DataType::Sint64 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Sint64(i64::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            DataType::Uint64 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint64(u64::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            DataType::Uint64z => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Uint64z(u64::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            DataType::Float32 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Float32(f32::from_bits(u32::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ]))));
                }
            }

            DataType::Float64 => {
                for _ in 0..number_of_values {
                    values.push(DataValue::Float64(f64::from_bits(u64::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ]))));
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
