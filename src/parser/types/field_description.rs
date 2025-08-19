#[derive(Debug, Clone, Copy)]
pub enum FieldDescriptionField {
    DeveloperDataIndex,
    FieldDefinitionNumber,
    FitBaseTypeId,
    FieldName,
    Array,
    Components,
    Scale,
    Offset,
    Units,
    Bits,
    Accumulate,
    FitBaseUnitId,
    NativeMesgNum,
    NativeFieldNum,

    Unknown(u8),
}

impl From<u8> for FieldDescriptionField {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::DeveloperDataIndex,
            1 => Self::FieldDefinitionNumber,
            2 => Self::FitBaseTypeId,
            3 => Self::FieldName,
            4 => Self::Array,
            5 => Self::Components,
            6 => Self::Scale,
            7 => Self::Offset,
            8 => Self::Units,
            9 => Self::Bits,
            10 => Self::Accumulate,
            13 => Self::FitBaseUnitId,
            14 => Self::NativeMesgNum,
            15 => Self::NativeFieldNum,
            val => Self::Unknown(val),
        }
    }
}
