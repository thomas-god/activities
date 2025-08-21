use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileHeaderError {
    #[error("File header is not exactly 14 bytes long")]
    InvalidHeaderSize,
    #[error("File header is not of type .FIT")]
    InvalidHeaderType,
    #[error("File header is malformed")]
    HeaderMalformed,
}

#[derive(Debug)]
pub struct FileHeader {
    pub protocol: u8,
    pub profile_version: u16,
    pub data_size: u32,
    pub crc: u16,
}

impl FileHeader {
    pub fn from_bytes<I>(content: &mut I) -> Result<FileHeader, FileHeaderError>
    where
        I: Iterator<Item = u8>,
    {
        let header_size = content.next().ok_or(FileHeaderError::InvalidHeaderSize)?; // byte 0
        if header_size != 14 {
            return Err(FileHeaderError::InvalidHeaderSize);
        }

        let protocol = content.next().ok_or(FileHeaderError::HeaderMalformed)?; // byte 1
        let profile_version = u16::from_le_bytes([
            content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 2
            content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 3
        ]);
        let data_size = u32::from_le_bytes([
            content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 4
            content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 5
            content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 6
            content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 7
        ]);

        let data_type = String::from_utf8(
            [
                content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 8
                content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 9
                content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 10
                content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 11
            ]
            .to_vec(),
        )
        .map_err(|_| FileHeaderError::InvalidHeaderType)?;
        if data_type != ".FIT" {
            return Err(FileHeaderError::InvalidHeaderType);
        }
        let crc = u16::from_le_bytes([
            content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 12
            content.next().ok_or(FileHeaderError::HeaderMalformed)?, // byte 13
        ]);

        Ok(Self {
            protocol,
            profile_version,
            data_size,
            crc,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::header::{FileHeader, FileHeaderError};

    #[test]
    fn test_parse_returns_err_if_header_size_less_than_14_bytes() {
        let mut content = vec![13].into_iter();

        let Err(FileHeaderError::InvalidHeaderSize) = FileHeader::from_bytes(&mut content) else {
            unreachable!("Should have return an Err(FileHeaderError::InvalidHeaderSize)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_header_size_more_than_14_bytes() {
        let mut content = vec![15].into_iter();

        let Err(FileHeaderError::InvalidHeaderSize) = FileHeader::from_bytes(&mut content) else {
            unreachable!("Should have return an Err(FileHeaderError::InvalidHeaderSize)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_no_protocol_byte_in_position_1() {
        let mut content = vec![14].into_iter();

        let Err(FileHeaderError::HeaderMalformed) = FileHeader::from_bytes(&mut content) else {
            unreachable!("Should have return an Err(FileHeaderError::HeaderMalformed)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_no_protocol_version_in_positions_2_3() {
        let mut content = vec![14, 0].into_iter();

        let Err(FileHeaderError::HeaderMalformed) = FileHeader::from_bytes(&mut content) else {
            unreachable!("Should have return an Err(FileHeaderError::HeaderMalformed)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_no_data_size_in_positions_4_7() {
        let mut content = vec![14, 0, 0, 12].into_iter();

        let Err(FileHeaderError::HeaderMalformed) = FileHeader::from_bytes(&mut content) else {
            unreachable!("Should have return an Err(FileHeaderError::HeaderMalformed)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_data_type_is_not_dot_fit() {
        let mut content = vec![14, 0, 0, 12, 0, 0, 0, 1];
        let mut data_type = String::from("tFIT").as_bytes().to_vec();
        content.append(&mut data_type);
        let mut content = content.into_iter();

        let Err(FileHeaderError::InvalidHeaderType) = FileHeader::from_bytes(&mut content) else {
            unreachable!("Should have return an Err(FileHeaderError::InvalidHeaderType)")
        };
    }

    #[test]
    fn test_parse_returns_err_if_crc_not_2_bytes() {
        let mut content = vec![14, 0, 0, 12, 0, 0, 0, 1];
        let mut data_type = String::from(".FIT").as_bytes().to_vec();
        content.append(&mut data_type);
        content.append(&mut vec![0]);
        let mut content = content.into_iter();

        let Err(FileHeaderError::HeaderMalformed) = FileHeader::from_bytes(&mut content) else {
            unreachable!("Should have return an Err(FileHeaderError::HeaderMalformed)")
        };
    }

    #[test]
    fn test_parse_returns_header() {
        let mut content = vec![14, 0, 13, 0, 1, 0, 0, 0];
        let mut data_type = String::from(".FIT").as_bytes().to_vec();
        content.append(&mut data_type);
        content.append(&mut vec![12, 0]);
        let mut content = content.into_iter();

        let Ok(header) = FileHeader::from_bytes(&mut content) else {
            unreachable!("Should have return an Ok(FileHeader)")
        };

        assert_eq!(header.protocol, 0);
        assert_eq!(header.profile_version, 13);
        assert_eq!(header.data_size, 1);
        assert_eq!(header.crc, 12);
    }
}
