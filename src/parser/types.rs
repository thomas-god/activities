#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordField {
    PositionLat,
    PositionLong,
    Altitude,
    HeartRate,
    Cadence,
    Distance,
    Speed,
    Power,
    CompressedSpeedDistance,
    Grade,
    Resistance,
    TimeFromCourse,
    CycleLength,
    Temperature,
    Speed1s,
    Cycles,
    TotalCycles,
    CompressedAccumulatedPower,
    AccumulatedPower,
    LeftRightBalance,
    GpsAccuracy,
    VerticalSpeed,
    Calories,
    VerticalOscillation,
    StanceTimePercent,
    StanceTime,
    ActivityType,
    LeftTorqueEffectiveness,
    RightTorqueEffectiveness,
    LeftPedalSmoothness,
    RightPedalSmoothness,
    CombinedPedalSmoothness,
    Time128,
    StrokeType,
    Zone,
    BallSpeed,
    Cadence256,
    FractionalCadence,
    TotalHemoglobinConc,
    TotalHemoglobinConcMin,
    TotalHemoglobinConcMax,
    SaturatedHemoglobinPercent,
    SaturatedHemoglobinPercentMin,
    SaturatedHemoglobinPercentMax,
    DeviceIndex,
    LeftPco,
    RightPco,
    LeftPowerPhase,
    LeftPowerPhasePeak,
    RightPowerPhase,
    RightPowerPhasePeak,
    EnhancedSpeed,
    EnhancedAltitude,
    BatterySoc,
    MotorPower,
    VerticalRatio,
    StanceTimeBalance,
    StepLength,
    CycleLength16,
    AbsolutePressure,
    Depth,
    NextStopDepth,
    NextStopTime,
    TimeToSurface,
    NdlTime,
    CnsLoad,
    N2Load,
    RespirationRate,
    EnhancedRespirationRate,
    Grit,
    Flow,
    CurrentStress,
    EbikeTraverRange,
    EbikeBatteryLevel,
    EbikeAssistMode,
    EbikeAssistLevelPercent,
    AirTimeRemaining,
    PressureSac,
    VolumeSac,
    Rmv,
    AscentRate,
    Po2,
    CoreTemperature,
    Timestamp,
    Unknown(u8),
}

impl From<u8> for RecordField {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::PositionLat,
            1 => Self::PositionLong,
            2 => Self::Altitude,
            3 => Self::HeartRate,
            4 => Self::Cadence,
            5 => Self::Distance,
            6 => Self::Speed,
            7 => Self::Power,
            8 => Self::CompressedSpeedDistance,
            9 => Self::Grade,
            10 => Self::Resistance,
            11 => Self::TimeFromCourse,
            12 => Self::CycleLength,
            13 => Self::Temperature,
            17 => Self::Speed1s,
            18 => Self::Cycles,
            19 => Self::TotalCycles,
            28 => Self::CompressedAccumulatedPower,
            29 => Self::AccumulatedPower,
            30 => Self::LeftRightBalance,
            31 => Self::GpsAccuracy,
            32 => Self::VerticalSpeed,
            33 => Self::Calories,
            39 => Self::VerticalOscillation,
            40 => Self::StanceTimePercent,
            41 => Self::StanceTime,
            42 => Self::ActivityType,
            43 => Self::LeftTorqueEffectiveness,
            44 => Self::RightTorqueEffectiveness,
            45 => Self::LeftPedalSmoothness,
            46 => Self::RightPedalSmoothness,
            47 => Self::CombinedPedalSmoothness,
            48 => Self::Time128,
            49 => Self::StrokeType,
            50 => Self::Zone,
            51 => Self::BallSpeed,
            52 => Self::Cadence256,
            53 => Self::FractionalCadence,
            54 => Self::TotalHemoglobinConc,
            55 => Self::TotalHemoglobinConcMin,
            56 => Self::TotalHemoglobinConcMax,
            57 => Self::SaturatedHemoglobinPercent,
            58 => Self::SaturatedHemoglobinPercentMin,
            59 => Self::SaturatedHemoglobinPercentMax,
            62 => Self::DeviceIndex,
            67 => Self::LeftPco,
            68 => Self::RightPco,
            69 => Self::LeftPowerPhase,
            70 => Self::LeftPowerPhasePeak,
            71 => Self::RightPowerPhase,
            72 => Self::RightPowerPhasePeak,
            73 => Self::EnhancedSpeed,
            78 => Self::EnhancedAltitude,
            81 => Self::BatterySoc,
            82 => Self::MotorPower,
            83 => Self::VerticalRatio,
            84 => Self::StanceTimeBalance,
            85 => Self::StepLength,
            87 => Self::CycleLength16,
            91 => Self::AbsolutePressure,
            92 => Self::Depth,
            93 => Self::NextStopDepth,
            94 => Self::NextStopTime,
            95 => Self::TimeToSurface,
            96 => Self::NdlTime,
            97 => Self::CnsLoad,
            98 => Self::N2Load,
            99 => Self::RespirationRate,
            108 => Self::EnhancedRespirationRate,
            114 => Self::Grit,
            115 => Self::Flow,
            116 => Self::CurrentStress,
            117 => Self::EbikeTraverRange,
            118 => Self::EbikeBatteryLevel,
            119 => Self::EbikeAssistMode,
            120 => Self::EbikeAssistLevelPercent,
            123 => Self::AirTimeRemaining,
            124 => Self::PressureSac,
            125 => Self::VolumeSac,
            126 => Self::Rmv,
            127 => Self::AscentRate,
            129 => Self::Po2,
            139 => Self::CoreTemperature,
            253 => Self::Timestamp,
            val => Self::Unknown(val),
        }
    }
}

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum RecordDataType {
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
pub enum RecordDataValue {
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

impl RecordDataType {
    /// Parse the enum variant from the base type field value
    pub fn from_base_type_field(base_type_field: u8) -> Result<Self, DataTypeError> {
        match base_type_field {
            0x00 => Ok(RecordDataType::Enum),
            0x01 => Ok(RecordDataType::Sint8),
            0x02 => Ok(RecordDataType::Uint8),
            0x83 => Ok(RecordDataType::Sint16),
            0x84 => Ok(RecordDataType::Uint16),
            0x85 => Ok(RecordDataType::Sint32),
            0x86 => Ok(RecordDataType::Uint32),
            0x07 => Ok(RecordDataType::String),
            0x88 => Ok(RecordDataType::Float32),
            0x89 => Ok(RecordDataType::Float64),
            0x0A => Ok(RecordDataType::Uint8z),
            0x8B => Ok(RecordDataType::Uint16z),
            0x8C => Ok(RecordDataType::Uint32z),
            0x0D => Ok(RecordDataType::Byte),
            0x8E => Ok(RecordDataType::Sint64),
            0x8F => Ok(RecordDataType::Uint64),
            0x90 => Ok(RecordDataType::Uint64z),
            _ => Ok(RecordDataType::Unknown),
        }
    }

    /// Get the size in bytes for this data type
    fn size_bytes(&self) -> u8 {
        match self {
            RecordDataType::Enum => 1,
            RecordDataType::Sint8 => 1,
            RecordDataType::Uint8 => 1,
            RecordDataType::Sint16 => 2,
            RecordDataType::Uint16 => 2,
            RecordDataType::Sint32 => 4,
            RecordDataType::Uint32 => 4,
            RecordDataType::String => 1, // Minimum size, actual size depends on content
            RecordDataType::Float32 => 4,
            RecordDataType::Float64 => 8,
            RecordDataType::Uint8z => 1,
            RecordDataType::Uint16z => 2,
            RecordDataType::Uint32z => 4,
            RecordDataType::Byte => 1, // Minimum size, actual size depends on content
            RecordDataType::Sint64 => 8,
            RecordDataType::Uint64 => 8,
            RecordDataType::Uint64z => 8,
            RecordDataType::Unknown => 1, // 1 to allways parse the number of bytes requested
        }
    }

    pub fn parse_values<I>(
        &self,
        content: &mut I,
        number_of_bytes: u8,
    ) -> Result<Vec<RecordDataValue>, DataTypeError>
    where
        I: Iterator<Item = u8>,
    {
        if number_of_bytes % self.size_bytes() != 0 {
            return Err(DataTypeError::InsufficientData);
        }
        let number_of_values = number_of_bytes / self.size_bytes();
        let mut values = Vec::new();

        match self {
            RecordDataType::Enum => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Enum(
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ));
                }
            }

            RecordDataType::Sint8 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Sint8(
                        content.next().ok_or(DataTypeError::InsufficientData)? as i8,
                    ));
                }
            }

            RecordDataType::Uint8 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Uint8(
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ));
                }
            }

            RecordDataType::Uint8z => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Uint8z(
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ));
                }
            }

            RecordDataType::Sint16 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Sint16(i16::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            RecordDataType::Uint16 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Uint16(u16::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            RecordDataType::Uint16z => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Uint16z(u16::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            RecordDataType::Sint32 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Sint32(i32::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            RecordDataType::Uint32 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Uint32(u32::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            RecordDataType::Uint32z => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Uint32z(u32::from_le_bytes([
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                        content.next().ok_or(DataTypeError::InsufficientData)?,
                    ])));
                }
            }

            RecordDataType::Sint64 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Sint64(i64::from_le_bytes([
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

            RecordDataType::Uint64 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Uint64(u64::from_le_bytes([
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

            RecordDataType::Uint64z => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Uint64z(u64::from_le_bytes([
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

            RecordDataType::Float32 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Float32(f32::from_bits(
                        u32::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    )));
                }
            }

            RecordDataType::Float64 => {
                for _ in 0..number_of_values {
                    values.push(RecordDataValue::Float64(f64::from_bits(
                        u64::from_le_bytes([
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                            content.next().ok_or(DataTypeError::InsufficientData)?,
                        ]),
                    )));
                }
            }

            RecordDataType::String => {
                let mut bytes = Vec::new();
                for _ in 0..number_of_bytes {
                    bytes.push(content.next().ok_or(DataTypeError::InsufficientData)?)
                }
                values.push(RecordDataValue::String(
                    String::from_utf8(bytes).map_err(|_| DataTypeError::InvalidUtf8)?,
                ));
            }

            RecordDataType::Byte => {
                let mut bytes = Vec::new();
                for _ in 0..number_of_values {
                    bytes.push(content.next().ok_or(DataTypeError::InsufficientData)?);
                }
                values.push(RecordDataValue::Byte(bytes));
            }

            RecordDataType::Unknown => {
                // Just consume the number of bytes from the iterator
                for _ in 0..number_of_values {
                    let _ = content.next();
                }
            }
        };
        Ok(values)
    }
}
