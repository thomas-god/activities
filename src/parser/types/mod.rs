use thiserror::Error;

use crate::parser::{
    definition::Endianness,
    reader::{Reader, ReaderError},
    types::generated::DataValue,
};

pub mod generated;

#[derive(Debug, Clone, PartialEq)]
pub enum BaseDataValue {
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
    #[error("Error while trying to read bytes from content")]
    ReaderError(#[from] ReaderError),
}

fn number_of_values(variant_size: u8, bytes: u8) -> Result<u8, DataTypeError> {
    let type_size = variant_size;
    if bytes % type_size != 0 {
        return Err(DataTypeError::InsufficientData);
    }
    Ok(bytes / type_size)
}

pub fn parse_uint8(
    content: &mut Reader,
    _endianness: &Endianness,
    bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    let number_of_values = number_of_values(1, bytes)?;
    let mut values = Vec::new();

    for _ in 0..number_of_values {
        values.push(DataValue::Base(BaseDataValue::Uint8(content.next_u8()?)));
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
        values.push(DataValue::Base(BaseDataValue::Uint16(
            content.next_u16(endianness)?,
        )));
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
        values.push(DataValue::Base(BaseDataValue::Uint32(
            content.next_u32(endianness)?,
        )));
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
        values.push(DataValue::Base(BaseDataValue::Uint64(
            content.next_u64(endianness)?,
        )));
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
        values.push(DataValue::Base(BaseDataValue::Uint8z(content.next_u8()?)));
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
        values.push(DataValue::Base(BaseDataValue::Uint16z(
            content.next_u16(endianness)?,
        )));
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
        values.push(DataValue::Base(BaseDataValue::Uint32z(
            content.next_u32(endianness)?,
        )));
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
        values.push(DataValue::Base(BaseDataValue::Uint64z(
            content.next_u64(endianness)?,
        )));
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
        values.push(DataValue::Base(BaseDataValue::Sint8(
            content.next_u8()? as i8
        )));
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
        values.push(DataValue::Base(BaseDataValue::Sint16(match endianness {
            Endianness::Little => i16::from_le_bytes([content.next_u8()?, content.next_u8()?]),
            Endianness::Big => i16::from_be_bytes([content.next_u8()?, content.next_u8()?]),
        })));
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
        values.push(DataValue::Base(BaseDataValue::Sint32(match endianness {
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
        })));
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
        values.push(DataValue::Base(BaseDataValue::Sint64(match endianness {
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
        })));
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
        values.push(DataValue::Base(BaseDataValue::Float32(f32::from_bits(
            content.next_u32(endianness)?,
        ))));
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
        values.push(DataValue::Base(BaseDataValue::Float64(f64::from_bits(
            content.next_u64(endianness)?,
        ))));
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

    Ok(vec![DataValue::Base(BaseDataValue::String(
        String::from_utf8(bytes).map_err(|_| DataTypeError::InvalidUtf8)?,
    ))])
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

    Ok(vec![DataValue::Base(BaseDataValue::Byte(bytes))])
}

pub fn parse_unknown(
    content: &mut Reader,
    _endianness: &Endianness,
    number_of_bytes: u8,
) -> Result<Vec<DataValue>, DataTypeError> {
    for _ in 0..number_of_bytes {
        let _ = content.next_u8()?;
    }

    Ok(Vec::new())
}

impl BaseDataValue {
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
    use crate::BaseDataValue;

    #[test]
    fn test_data_value_enum_invalid() {
        assert!(!BaseDataValue::Enum(0).is_invalid());
        assert!(BaseDataValue::Enum(0xFF).is_invalid());
    }

    #[test]
    fn test_data_value_sint_invalid() {
        assert!(!BaseDataValue::Sint8(0).is_invalid());
        assert!(BaseDataValue::Sint8(0x7F).is_invalid());

        assert!(!BaseDataValue::Sint16(0).is_invalid());
        assert!(BaseDataValue::Sint16(0x7FFF).is_invalid());

        assert!(!BaseDataValue::Sint32(0).is_invalid());
        assert!(BaseDataValue::Sint32(0x7FFFFFFF).is_invalid());

        assert!(!BaseDataValue::Sint64(0).is_invalid());
        assert!(BaseDataValue::Sint64(0x7FFFFFFFFFFFFFFF).is_invalid());
    }

    #[test]
    fn test_data_value_uint_invalid() {
        assert!(!BaseDataValue::Uint8(0).is_invalid());
        assert!(BaseDataValue::Uint8(0xFF).is_invalid());

        assert!(!BaseDataValue::Uint16(0).is_invalid());
        assert!(BaseDataValue::Uint16(0xFFFF).is_invalid());

        assert!(!BaseDataValue::Uint32(0).is_invalid());
        assert!(BaseDataValue::Uint32(0xFFFFFFFF).is_invalid());

        assert!(!BaseDataValue::Uint64(0).is_invalid());
        assert!(BaseDataValue::Uint64(0xFFFFFFFFFFFFFFFF).is_invalid());
    }

    #[test]
    fn test_data_value_uintz_invalid() {
        assert!(!BaseDataValue::Uint8z(0xFF).is_invalid());
        assert!(BaseDataValue::Uint8z(0).is_invalid());

        assert!(!BaseDataValue::Uint16z(0xFFFF).is_invalid());
        assert!(BaseDataValue::Uint16z(0).is_invalid());

        assert!(!BaseDataValue::Uint32z(0xFFFFFFFF).is_invalid());
        assert!(BaseDataValue::Uint32z(0).is_invalid());

        assert!(!BaseDataValue::Uint64z(0xFFFFFFFFFFFFFFFF).is_invalid());
        assert!(BaseDataValue::Uint64z(0).is_invalid());
    }

    #[test]
    fn test_data_value_float_invalid() {
        assert!(!BaseDataValue::Float32(f32::from_le_bytes([0x00, 0x00, 0x00, 0x00])).is_invalid());
        assert!(BaseDataValue::Float32(f32::from_le_bytes([0xFF, 0xFF, 0xFF, 0xFF])).is_invalid());

        assert!(
            !BaseDataValue::Float64(f64::from_le_bytes([
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ]))
            .is_invalid()
        );
        assert!(
            BaseDataValue::Float64(f64::from_le_bytes([
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
            ]))
            .is_invalid()
        );
    }

    #[test]
    fn test_data_value_byte_invalid() {
        assert!(!BaseDataValue::Byte(vec![0x00, 0x00]).is_invalid());
        assert!(BaseDataValue::Byte(vec![0xFF, 0xFF]).is_invalid());
    }

    #[test]
    fn test_data_value_string_always_valid() {
        assert!(!BaseDataValue::String(String::from_utf8(vec![]).unwrap()).is_invalid());
        assert!(!BaseDataValue::String(String::from_utf8(vec![0x00]).unwrap()).is_invalid());
        assert!(!BaseDataValue::String(String::from_utf8(vec![0x01, 0x00]).unwrap()).is_invalid());
    }

    #[test]
    fn test_data_value_unknown_always_invalid() {
        assert!(BaseDataValue::Unknown.is_invalid());
    }
}
