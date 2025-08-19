#[derive(Debug, Clone, Copy)]
pub enum DeveloperDataIdField {
    DeveloperId,
    ApplicationId,
    ManufacturerId,
    DeveloperDataIndex,
    ApplicationVersion,
    Unknown(u8),
}

impl From<u8> for DeveloperDataIdField {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::DeveloperId,
            1 => Self::ApplicationId,
            2 => Self::ManufacturerId,
            3 => Self::DeveloperDataIndex,
            4 => Self::ApplicationVersion,
            val => Self::Unknown(val),
        }
    }
}
