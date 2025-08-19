#[derive(Debug, Clone, Copy)]
pub enum FileIdField {
    Type,
    Manufacturer,
    Product,
    SerialNumber,
    TimeCreated,
    Number,
    ProductName,
    Unknown(u8),
}

impl From<u8> for FileIdField {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Type,
            1 => Self::Manufacturer,
            2 => Self::Product,
            3 => Self::ProductName,
            4 => Self::SerialNumber,
            5 => Self::TimeCreated,
            6 => Self::Number,
            7 => Self::ProductName,
            val => Self::Unknown(val),
        }
    }
}
