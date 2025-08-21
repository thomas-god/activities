use thiserror::Error;

use crate::parser::definition::Endianness;

#[derive(Debug)]
pub struct Reader<I> {
    bytes_left_to_read: u32,
    crc: u16,
    content: I,
}

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("Content has been exhausted of expected number of bytes")]
    ContentExhausted,
    #[error("Content is empty while more bytes were expected")]
    ContentPrematurelyEmpty,
}

impl<I> Reader<I> {
    pub fn is_empty(&self) -> bool {
        self.bytes_left_to_read == 0
    }
}

impl<I> Reader<I>
where
    I: Iterator<Item = u8>,
{
    pub fn new(bytes_to_read: u32, crc: u16, content: I) -> Self {
        Self {
            bytes_left_to_read: bytes_to_read,
            crc,
            content,
        }
    }

    pub fn next_u8(&mut self) -> Result<u8, ReaderError> {
        if self.bytes_left_to_read == 0 {
            return Err(ReaderError::ContentExhausted);
        }

        let res = self
            .content
            .next()
            .ok_or(ReaderError::ContentPrematurelyEmpty);
        self.bytes_left_to_read = self.bytes_left_to_read.saturating_sub(1);
        res
    }

    pub fn next_u16(&mut self, endianness: &Endianness) -> Result<u16, ReaderError> {
        let bytes = [self.next_u8()?, self.next_u8()?];
        match endianness {
            Endianness::Big => Ok(u16::from_be_bytes(bytes)),
            Endianness::Little => Ok(u16::from_le_bytes(bytes)),
        }
    }

    pub fn next_u32(&mut self, endianness: &Endianness) -> Result<u32, ReaderError> {
        let bytes = [
            self.next_u8()?,
            self.next_u8()?,
            self.next_u8()?,
            self.next_u8()?,
        ];
        match endianness {
            Endianness::Big => Ok(u32::from_be_bytes(bytes)),
            Endianness::Little => Ok(u32::from_le_bytes(bytes)),
        }
    }

    pub fn next_u64(&mut self, endianness: &Endianness) -> Result<u64, ReaderError> {
        let bytes = [
            self.next_u8()?,
            self.next_u8()?,
            self.next_u8()?,
            self.next_u8()?,
            self.next_u8()?,
            self.next_u8()?,
            self.next_u8()?,
            self.next_u8()?,
        ];
        match endianness {
            Endianness::Big => Ok(u64::from_be_bytes(bytes)),
            Endianness::Little => Ok(u64::from_le_bytes(bytes)),
        }
    }
}

#[cfg(test)]
mod tests {

    use std::mem::discriminant;

    use crate::parser::definition::Endianness;

    use super::*;

    #[test]
    fn test_next_returns_err_if_no_bytes_expected_left_to_read_initial() {
        let content = vec![0, 0].into_iter();
        let bytes_to_read = 0;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u8();
        assert!(res.is_err());
        assert_eq!(
            discriminant(&res.unwrap_err()),
            discriminant(&ReaderError::ContentExhausted)
        );
    }

    #[test]
    fn test_next_returns_err_if_no_bytes_expected_left_to_read_after_reads() {
        let content = vec![0, 0].into_iter();
        let bytes_to_read = 1;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u8();
        assert!(res.is_ok());

        let res = reader.next_u8();
        assert!(res.is_err());
        assert_eq!(
            discriminant(&res.unwrap_err()),
            discriminant(&ReaderError::ContentExhausted)
        );
    }

    #[test]
    fn test_next_returns_err_if_underlying_content_is_empty() {
        let content = vec![].into_iter();
        let bytes_to_read = 1;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u8();
        assert!(res.is_err());
        assert_eq!(
            discriminant(&res.unwrap_err()),
            discriminant(&ReaderError::ContentPrematurelyEmpty)
        );
    }

    #[test]
    fn test_next_returns_next_byte() {
        let content = vec![1, 0, 0].into_iter();
        let bytes_to_read = 1;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u8();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1);
    }

    #[test]
    fn test_parse_u16_big_endian() {
        let content = vec![12, 6, 0].into_iter();
        let bytes_to_read = 2;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u16(&Endianness::Big);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u16::from_be_bytes([12, 6]));
    }

    #[test]
    fn test_parse_u16_little_endian() {
        let content = vec![17, 5, 0].into_iter();
        let bytes_to_read = 2;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u16(&Endianness::Little);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u16::from_le_bytes([17, 5]));
    }

    #[test]
    fn test_parse_u32_little_endian() {
        let content = vec![12, 6, 0, 0].into_iter();
        let bytes_to_read = 4;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u32(&Endianness::Little);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u32::from_le_bytes([12, 6, 0, 0]));
    }

    #[test]
    fn test_parse_u32_big_endian() {
        let content = vec![12, 6, 0, 0].into_iter();
        let bytes_to_read = 4;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u32(&Endianness::Big);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u32::from_be_bytes([12, 6, 0, 0]));
    }

    #[test]
    fn test_parse_u64_big_endian() {
        let content = vec![17, 5, 0, 0, 17, 5, 0, 0].into_iter();
        let bytes_to_read = 8;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u64(&Endianness::Big);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u64::from_be_bytes([17, 5, 0, 0, 17, 5, 0, 0]));
    }

    #[test]
    fn test_parse_u64_little_endian() {
        let content = vec![17, 5, 0, 0, 17, 5, 0, 0].into_iter();
        let bytes_to_read = 8;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        let res = reader.next_u64(&Endianness::Little);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u64::from_le_bytes([17, 5, 0, 0, 17, 5, 0, 0]));
    }

    #[test]
    fn test_reaader_is_empty() {
        let content = vec![0].into_iter();
        let bytes_to_read = 1;
        let crc = 0;

        let mut reader = Reader::new(bytes_to_read, crc, content);

        assert!(!reader.is_empty());
        let _ = reader.next_u8();
        assert!(reader.is_empty());
    }
}
