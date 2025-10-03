use thiserror::Error;

use crate::parser::{Endianness, reader::Reader};

#[derive(Error, Debug)]
pub enum FileHeaderError {
    #[error("File header {0} expected 14 bytes")]
    InvalidHeaderSize(u8),
    #[error("File header is not of type .FIT")]
    InvalidHeaderType,
    #[error("File header is malformed")]
    HeaderMalformed,
    #[error("Invalid header CRC value: expected {0} but got {1}")]
    InvalidCRC(u16, u16),
}

pub const HEADER_SIZE_WITH_CRC: u8 = 14;
pub const HEADER_SIZE_WITHOUT_CRC: u8 = 12;

#[derive(Debug)]
pub struct FileHeader {
    pub _protocol: u8,
    pub _profile_version: u16,
    pub data_size: u32,
}

impl FileHeader {
    pub fn from_bytes(reader: &mut Reader) -> Result<FileHeader, FileHeaderError> {
        let header_size = reader
            .next_u8()
            .map_err(|_| FileHeaderError::HeaderMalformed)?;

        let crc_present = match header_size {
            size if size == HEADER_SIZE_WITH_CRC => true,
            size if size == HEADER_SIZE_WITHOUT_CRC => false,
            _ => return Err(FileHeaderError::InvalidHeaderSize(header_size)),
        };

        let protocol = reader
            .next_u8()
            .map_err(|_| FileHeaderError::HeaderMalformed)?; // byte 1
        let profile_version = reader
            .next_u16(&Endianness::Little)
            .map_err(|_| FileHeaderError::HeaderMalformed)?; // bytes 2 and 3

        let data_size = reader
            .next_u32(&Endianness::Little)
            .map_err(|_| FileHeaderError::HeaderMalformed)?; // bytes 4 to 7

        let data_type = String::from_utf8(
            [
                reader
                    .next_u8()
                    .map_err(|_| FileHeaderError::HeaderMalformed)?, // byte 8
                reader
                    .next_u8()
                    .map_err(|_| FileHeaderError::HeaderMalformed)?, // byte 9
                reader
                    .next_u8()
                    .map_err(|_| FileHeaderError::HeaderMalformed)?, // byte 10
                reader
                    .next_u8()
                    .map_err(|_| FileHeaderError::HeaderMalformed)?, // byte 11
            ]
            .to_vec(),
        )
        .map_err(|_| FileHeaderError::InvalidHeaderType)?;
        if data_type != ".FIT" {
            return Err(FileHeaderError::InvalidHeaderType);
        }
        if crc_present {
            let crc = reader.current_crc();

            let expected_crc = reader
                .next_u16(&Endianness::Little) // bytes 12 and 13
                .map_err(|_| FileHeaderError::HeaderMalformed)?;

            if crc != expected_crc {
                return Err(FileHeaderError::InvalidCRC(expected_crc, crc));
            }
        }

        Ok(Self {
            _protocol: protocol,
            _profile_version: profile_version,
            data_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_returns_err_if_header_size_less_than_14_bytes() {
        let mut reader = Reader::new(14, vec![13].into_iter());

        let Err(FileHeaderError::InvalidHeaderSize(size)) = FileHeader::from_bytes(&mut reader)
        else {
            unreachable!("Should have return an Err(FileHeaderError::InvalidHeaderSize)")
        };
        assert_eq!(size, 13);
    }

    #[test]
    fn test_parse_returns_err_if_header_size_more_than_14_bytes() {
        let mut reader = Reader::new(14, vec![15].into_iter());

        let Err(FileHeaderError::InvalidHeaderSize(size)) = FileHeader::from_bytes(&mut reader)
        else {
            unreachable!("Should have return an Err(FileHeaderError::InvalidHeaderSize)")
        };
        assert_eq!(size, 15);
    }

    #[test]
    fn test_parse_returns_err_if_no_protocol_byte_in_position_1() {
        let mut reader = Reader::new(14, vec![HEADER_SIZE_WITH_CRC].into_iter());

        let Err(FileHeaderError::HeaderMalformed) = FileHeader::from_bytes(&mut reader) else {
            unreachable!("Should have return an Err(FileHeaderError::HeaderMalformed)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_no_protocol_version_in_positions_2_3() {
        let mut reader = Reader::new(14, vec![HEADER_SIZE_WITH_CRC, 0].into_iter());

        let Err(FileHeaderError::HeaderMalformed) = FileHeader::from_bytes(&mut reader) else {
            unreachable!("Should have return an Err(FileHeaderError::HeaderMalformed)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_no_data_size_in_positions_4_7() {
        let mut reader = Reader::new(14, vec![HEADER_SIZE_WITH_CRC, 0, 0, 12].into_iter());

        let Err(FileHeaderError::HeaderMalformed) = FileHeader::from_bytes(&mut reader) else {
            unreachable!("Should have return an Err(FileHeaderError::HeaderMalformed)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_data_type_is_not_dot_fit() {
        let mut content = vec![HEADER_SIZE_WITH_CRC, 0, 0, 12, 0, 0, 0, 1];
        let mut data_type = String::from("tFIT").as_bytes().to_vec();
        content.append(&mut data_type);
        let mut reader = Reader::new(14, content.into_iter());

        let Err(FileHeaderError::InvalidHeaderType) = FileHeader::from_bytes(&mut reader) else {
            unreachable!("Should have return an Err(FileHeaderError::InvalidHeaderType)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_crc_not_2_bytes() {
        let mut content = vec![HEADER_SIZE_WITH_CRC, 0, 0, 12, 0, 0, 0, 1];
        let mut data_type = String::from(".FIT").as_bytes().to_vec();
        content.append(&mut data_type);
        content.append(&mut vec![0]);
        let mut reader = Reader::new(14, content.into_iter());

        let Err(FileHeaderError::HeaderMalformed) = FileHeader::from_bytes(&mut reader) else {
            unreachable!("Should have return an Err(FileHeaderError::HeaderMalformed)")
        };
    }

    #[test]
    fn test_parse_header_crc_is_invalid() {
        let mut content = vec![HEADER_SIZE_WITH_CRC, 0, 13, 0, 1, 0, 0, 0];
        let mut data_type = String::from(".FIT").as_bytes().to_vec();
        content.append(&mut data_type);
        content.append(&mut vec![103, 114]); // CRC value for bytes 0-11
        let mut reader = Reader::new(14, content.into_iter());

        let Err(FileHeaderError::InvalidCRC(_expected_crc, _crc)) =
            FileHeader::from_bytes(&mut reader)
        else {
            unreachable!("Should have return an Err(FileHeaderError::HeaderMalformed)")
        };
    }

    #[test]
    fn test_parse_returns_header() {
        let mut content = vec![HEADER_SIZE_WITH_CRC, 0, 13, 0, 1, 0, 0, 0];
        let mut data_type = String::from(".FIT").as_bytes().to_vec();
        content.append(&mut data_type);
        content.append(&mut vec![103, 115]); // CRC value for bytes 0-11
        let mut reader = Reader::new(14, content.into_iter());

        let Ok(header) = FileHeader::from_bytes(&mut reader) else {
            unreachable!("Should have return an Ok(FileHeader)")
        };

        assert_eq!(header._protocol, 0);
        assert_eq!(header._profile_version, 13);
        assert_eq!(header.data_size, 1);
    }

    #[test]
    fn test_parse_12bytes_header() {
        let mut content = vec![12, 0, 13, 0, 1, 0, 0, 0];
        let mut data_type = String::from(".FIT").as_bytes().to_vec();
        content.append(&mut data_type);
        // No header CRC for 12 bytes headers

        let mut reader = Reader::new(12, content.into_iter());

        let Ok(header) = FileHeader::from_bytes(&mut reader) else {
            unreachable!("Should have return an Ok(FileHeader)")
        };

        assert_eq!(header._protocol, 0);
        assert_eq!(header._profile_version, 13);
        assert_eq!(header.data_size, 1);
    }
}
