use thiserror::Error;

use crate::parser::definition::Endianness;

#[derive(Debug)]
pub struct Reader {
    bytes_left_to_read: u32,
    crc: u16,
    content: std::vec::IntoIter<u8>,
}

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("Content has been exhausted of expected number of bytes")]
    ContentExhausted,
    #[error("Content is empty while more bytes were expected")]
    ContentPrematurelyEmpty,
}

impl Reader {
    pub fn is_empty(&self) -> bool {
        self.bytes_left_to_read == 0
    }

    pub fn new(bytes_to_read: u32, content: std::vec::IntoIter<u8>) -> Self {
        Self {
            bytes_left_to_read: bytes_to_read,
            crc: 0,
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
        if let Ok(byte) = res {
            self.crc = compute_crc(&self.crc, byte);
        }
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

    pub fn current_crc(&self) -> u16 {
        self.crc
    }

    pub fn check_crc(&self, expected_crc: u16) -> bool {
        self.crc == expected_crc
    }

    pub fn remaining_content(self) -> std::vec::IntoIter<u8> {
        self.content
    }
}

const CRC_TABLE: [u16; 16] = [
    0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800, 0xB401,
    0x5000, 0x9C01, 0x8801, 0x4400,
];

fn compute_crc(crc: &u16, byte: u8) -> u16 {
    // Process lower 4 bits of byte
    let tmp = CRC_TABLE[(crc & 0xF) as usize];
    let mut crc = (*crc >> 4) & 0x0FFF;
    crc ^= tmp ^ CRC_TABLE[(byte & 0xF) as usize];

    // Process upper 4 bits of byte
    let tmp = CRC_TABLE[(crc & 0xF) as usize];
    crc = (crc >> 4) & 0x0FFF;
    crc ^= tmp ^ CRC_TABLE[((byte >> 4) & 0xF) as usize];

    crc
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

        let mut reader = Reader::new(bytes_to_read, content);

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

        let mut reader = Reader::new(bytes_to_read, content);

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

        let mut reader = Reader::new(bytes_to_read, content);

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

        let mut reader = Reader::new(bytes_to_read, content);

        let res = reader.next_u8();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1);
    }

    #[test]
    fn test_parse_u16_big_endian() {
        let content = vec![12, 6, 0].into_iter();
        let bytes_to_read = 2;

        let mut reader = Reader::new(bytes_to_read, content);

        let res = reader.next_u16(&Endianness::Big);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u16::from_be_bytes([12, 6]));
    }

    #[test]
    fn test_parse_u16_little_endian() {
        let content = vec![17, 5, 0].into_iter();
        let bytes_to_read = 2;

        let mut reader = Reader::new(bytes_to_read, content);

        let res = reader.next_u16(&Endianness::Little);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u16::from_le_bytes([17, 5]));
    }

    #[test]
    fn test_parse_u32_little_endian() {
        let content = vec![12, 6, 0, 0].into_iter();
        let bytes_to_read = 4;

        let mut reader = Reader::new(bytes_to_read, content);

        let res = reader.next_u32(&Endianness::Little);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u32::from_le_bytes([12, 6, 0, 0]));
    }

    #[test]
    fn test_parse_u32_big_endian() {
        let content = vec![12, 6, 0, 0].into_iter();
        let bytes_to_read = 4;

        let mut reader = Reader::new(bytes_to_read, content);

        let res = reader.next_u32(&Endianness::Big);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u32::from_be_bytes([12, 6, 0, 0]));
    }

    #[test]
    fn test_parse_u64_big_endian() {
        let content = vec![17, 5, 0, 0, 17, 5, 0, 0].into_iter();
        let bytes_to_read = 8;

        let mut reader = Reader::new(bytes_to_read, content);

        let res = reader.next_u64(&Endianness::Big);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u64::from_be_bytes([17, 5, 0, 0, 17, 5, 0, 0]));
    }

    #[test]
    fn test_parse_u64_little_endian() {
        let content = vec![17, 5, 0, 0, 17, 5, 0, 0].into_iter();
        let bytes_to_read = 8;

        let mut reader = Reader::new(bytes_to_read, content);

        let res = reader.next_u64(&Endianness::Little);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), u64::from_le_bytes([17, 5, 0, 0, 17, 5, 0, 0]));
    }

    #[test]
    fn test_reaader_is_empty() {
        let content = vec![0].into_iter();
        let bytes_to_read = 1;

        let mut reader = Reader::new(bytes_to_read, content);

        assert!(!reader.is_empty());
        let _ = reader.next_u8();
        assert!(reader.is_empty());
    }
}
