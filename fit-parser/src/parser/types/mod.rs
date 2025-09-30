use thiserror::Error;

use crate::parser::{
    definition::Endianness,
    reader::{Reader, ReaderError},
    records::RecordError,
    types::generated::FitEnum,
};

pub mod generated;

#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    Enum(FitEnum),
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
    DateTime(u32),
    Unknown(Vec<u8>),
}

#[derive(Debug, Error)]
pub enum DataTypeError {
    #[error(
        "Expected number of bytes ({0}) to read is not a multiple of the underlying type size ({1} bytes)"
    )]
    DataNotAligned(u8, u8),
    #[error("Unable to parse Utf-8 String from bytes")]
    InvalidUtf8,
    #[error("Error while trying to read bytes from content")]
    ReaderError(#[from] ReaderError),
}

fn number_of_values(type_size: u8, bytes_to_read: u8) -> Result<u8, DataTypeError> {
    if bytes_to_read % type_size != 0 {
        return Err(DataTypeError::DataNotAligned(bytes_to_read, type_size));
    }
    Ok(bytes_to_read / type_size)
}

pub fn parse_uint8(
    content: &mut Reader,
    _endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(1, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Uint8(content.next_u8()?));
    }

    Ok(values)
}

pub fn parse_uint16(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(2, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Uint16(content.next_u16(endianness)?));
    }

    Ok(values)
}

pub fn parse_uint32(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(4, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Uint32(content.next_u32(endianness)?));
    }

    Ok(values)
}

pub fn parse_uint64(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(8, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Uint64(content.next_u64(endianness)?));
    }

    Ok(values)
}

pub fn parse_uint8z(
    content: &mut Reader,
    _endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(1, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Uint8z(content.next_u8()?));
    }

    Ok(values)
}

pub fn parse_uint16z(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(2, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Uint16z(content.next_u16(endianness)?));
    }

    Ok(values)
}

pub fn parse_uint32z(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(4, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Uint32z(content.next_u32(endianness)?));
    }

    Ok(values)
}

pub fn parse_uint64z(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(8, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Uint64z(content.next_u64(endianness)?));
    }

    Ok(values)
}

pub fn parse_sint8(
    content: &mut Reader,
    _endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(1, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Sint8(content.next_u8()? as i8));
    }

    Ok(values)
}

pub fn parse_sint16(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(2, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Sint16(match endianness {
            Endianness::Little => i16::from_le_bytes([content.next_u8()?, content.next_u8()?]),
            Endianness::Big => i16::from_be_bytes([content.next_u8()?, content.next_u8()?]),
        }));
    }

    Ok(values)
}

pub fn parse_sint32(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(4, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Sint32(match endianness {
            Endianness::Little => i32::from_le_bytes([
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
            ]),
            Endianness::Big => i32::from_be_bytes([
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
            ]),
        }));
    }

    Ok(values)
}

pub fn parse_sint64(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(8, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Sint64(match endianness {
            Endianness::Little => i64::from_le_bytes([
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
            ]),
            Endianness::Big => i64::from_be_bytes([
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
                content.next_u8()?,
            ]),
        }));
    }

    Ok(values)
}

pub fn parse_float32(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(4, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Float32(f32::from_bits(
            content.next_u32(endianness)?,
        )));
    }

    Ok(values)
}

pub fn parse_float64(
    content: &mut Reader,
    endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(8, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Float64(f64::from_bits(
            content.next_u64(endianness)?,
        )));
    }

    Ok(values)
}

pub fn parse_string(
    content: &mut Reader,
    _endianness: &Endianness,
    number_of_bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let mut bytes = Vec::new();
    for _ in 0..number_of_bytes {
        bytes.push(content.next_u8()?)
    }

    let string = String::from_utf8_lossy(&bytes);
    let string = string.trim_matches(char::from(0));

    Ok(vec![DataValue::String(string.to_string())])
}

pub fn parse_byte_array(
    content: &mut Reader,
    _endianness: &Endianness,
    number_of_bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let mut bytes = Vec::new();
    for _ in 0..number_of_bytes {
        bytes.push(content.next_u8()?);
    }

    Ok(vec![DataValue::Byte(bytes)])
}

pub fn parse_unknown(
    content: &mut Reader,
    _endianness: &Endianness,
    number_of_bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let mut bytes = Vec::new();
    for _ in 0..number_of_bytes {
        bytes.push(content.next_u8()?);
    }

    Ok(vec![DataValue::Unknown(bytes)])
}

impl DataValue {
    /// Check if a value should be considered invalid as per the .FIT protocol. Notable exceptions
    /// are:
    ///
    /// - [DataValue::String] are always considered valid, event if empty,
    /// - [DataValue::Unknown] are always considerred invalid.
    pub fn is_invalid(&self) -> bool {
        match self {
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
            Self::DateTime(_) => false,
            Self::Enum(_) => false,
            Self::Unknown(_) => true,
        }
    }

    pub fn apply_scale_offset(
        &self,
        scale_offset: &Option<ScaleOffset>,
    ) -> Result<DataValue, RecordError> {
        if self.is_invalid() {
            return Ok(self.clone());
        }

        let Some(ScaleOffset { scale, offset }) = scale_offset else {
            return Ok(self.clone());
        };

        if *scale == 0. {
            return Err(RecordError::ScaleByZeroError);
        }

        match self {
            Self::Sint8(val) => Ok(Self::Float32((*val as f32) / *scale - *offset)),
            Self::Sint16(val) => Ok(Self::Float32((*val as f32) / *scale - *offset)),
            Self::Sint32(val) => Ok(Self::Float32((*val as f32) / *scale - *offset)),
            Self::Sint64(val) => {
                let scale = *scale as f64;
                let offset = *offset as f64;
                Ok(Self::Float64((*val as f64) / scale - offset))
            }
            Self::Uint8(val) => Ok(Self::Float32((*val as f32) / scale - offset)),
            Self::Uint16(val) => Ok(Self::Float32((*val as f32) / *scale - *offset)),
            Self::Uint32(val) => Ok(Self::Float32((*val as f32) / *scale - *offset)),
            Self::Uint64(val) => {
                let scale = *scale as f64;
                let offset = *offset as f64;
                Ok(Self::Float64((*val as f64) / scale - offset))
            }
            Self::Uint8z(val) => Ok(Self::Float32((*val as f32) / *scale - *offset)),
            Self::Uint16z(val) => Ok(Self::Float32((*val as f32) / *scale - *offset)),
            Self::Uint32z(val) => Ok(Self::Float32((*val as f32) / *scale - *offset)),
            Self::Uint64z(val) => {
                let scale = *scale as f64;
                let offset = *offset as f64;

                Ok(Self::Float64((*val as f64) / scale - offset))
            }
            Self::Float32(val) => Ok(Self::Float32(*val / scale - offset)),
            Self::Float64(val) => {
                let scale = *scale as f64;
                let offset = *offset as f64;

                Ok(Self::Float64(*val / scale - offset))
            }
            Self::DateTime(val) => Ok(Self::DateTime(((*val as f32) / scale - offset) as u32)),
            val => Ok(val.clone()),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct ScaleOffset {
    pub scale: f32,
    pub offset: f32,
}

#[cfg(test)]
mod tests {
    use crate::parser::types::generated::Activity;

    use super::*;

    #[test]
    fn test_data_value_enum_invalid() {
        assert!(!DataValue::Enum(FitEnum::Activity(Activity::AutoMultiSport)).is_invalid());
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
        assert!(DataValue::Unknown(vec![]).is_invalid());
    }

    #[test]
    fn test_apply_scale_offset_is_none() {
        let value = DataValue::Sint32(100);
        let result = value.apply_scale_offset(&None);

        assert_eq!(result.unwrap(), DataValue::Sint32(100));
    }

    #[test]
    fn test_apply_scale_offset_as_f32() {
        let value = DataValue::Uint8(135);
        let scale = Some(ScaleOffset {
            scale: 2.,
            offset: 50.,
        });
        let result = value.apply_scale_offset(&scale);

        assert_eq!(result.unwrap(), DataValue::Float32(17.5));
    }

    #[test]
    fn test_apply_scale_offset_as_f64() {
        let value = DataValue::Uint64(135);
        let scale = Some(ScaleOffset {
            scale: 2.,
            offset: 50.,
        });
        let result = value.apply_scale_offset(&scale);

        assert_eq!(result.unwrap(), DataValue::Float64(17.5));
    }

    #[test]
    fn test_apply_scale_offset_datetime() {
        let value = DataValue::DateTime(135);
        let scale = Some(ScaleOffset {
            scale: 2.,
            offset: 50.,
        });
        let result = value.apply_scale_offset(&scale);

        assert_eq!(result.unwrap(), DataValue::DateTime(17));
    }

    #[test]
    fn test_apply_scale_offset_no_effect_on_string() {
        let value = DataValue::String("toto".to_string());
        let scale = Some(ScaleOffset {
            scale: 2.,
            offset: 50.,
        });
        let result = value.apply_scale_offset(&scale);

        assert_eq!(result.unwrap(), DataValue::String("toto".to_string()));
    }

    #[test]
    fn test_scale_is_equals_to_zero() {
        let value = DataValue::String("toto".to_string());
        let scale = Some(ScaleOffset {
            scale: 0.,
            offset: 50.,
        });
        let result = value.apply_scale_offset(&scale);

        assert!(result.is_err());
    }

    #[test]
    fn test_scale_offset_keeps_invalid_values() {
        let test_values = vec![
            DataValue::Uint8(u8::MAX),
            DataValue::Uint16(u16::MAX),
            DataValue::Uint32(u32::MAX),
            DataValue::Uint64(u64::MAX),
            DataValue::Uint8z(0),
            DataValue::Uint16z(0),
            DataValue::Uint32z(0),
            DataValue::Uint64z(0),
            DataValue::Sint8(i8::MAX),
            DataValue::Sint16(i16::MAX),
            DataValue::Sint32(i32::MAX),
            DataValue::Sint64(i64::MAX),
        ];
        let scale = Some(ScaleOffset {
            scale: 0.5,
            offset: 1.,
        });

        for invalid_value in test_values {
            assert_eq!(
                invalid_value.apply_scale_offset(&scale).unwrap(),
                invalid_value,
                "Invalid value {:?} shoud not be applied a scale/offset",
                invalid_value
            );
        }
    }

    #[test]
    fn test_scale_offset_keeps_nan_values() {
        let scale = Some(ScaleOffset {
            scale: 0.5,
            offset: 1.,
        });

        let invalid_f32 = DataValue::Float32(f32::from_le_bytes([0xFF, 0xFF, 0xFF, 0xFF]));

        match invalid_f32.apply_scale_offset(&scale) {
            Ok(DataValue::Float32(val)) => assert!(val.is_nan()),
            _ => unreachable!("Should have return an Ok(DataValue::Float32()"),
        }
        let invalid_f64 = DataValue::Float64(f64::from_le_bytes([
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ]));

        match invalid_f64.apply_scale_offset(&scale) {
            Ok(DataValue::Float64(val)) => assert!(val.is_nan()),
            _ => unreachable!("Should have return an Ok(DataValue::Float32()"),
        }
    }
}
