#[cfg(test)]
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(EnumIter))]
pub enum GlobalMessage {
    FileId,
    Record,
    FieldDescription,
    DeveloperDataId,
    Unsupported(u16),
}

impl From<u16> for GlobalMessage {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::FileId,
            20 => Self::Record,
            206 => Self::FieldDescription,
            207 => Self::DeveloperDataId,
            val => Self::Unsupported(val),
        }
    }
}

impl GlobalMessage {
    pub fn parse_field(&self, definition_number: u8) -> DataField {
        match self {
            GlobalMessage::Record => DataField::Record(RecordField::from(definition_number)),
            GlobalMessage::FileId => DataField::FileId(FileIdField::from(definition_number)),
            GlobalMessage::FieldDescription => {
                DataField::FieldDescription(FieldDescriptionField::from(definition_number))
            }
            GlobalMessage::DeveloperDataId => {
                DataField::DeveloperDataId(DeveloperDataIdField::from(definition_number))
            }
            GlobalMessage::Unsupported(_) => DataField::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DataField {
    FileId(FileIdField),
    Record(RecordField),
    FieldDescription(FieldDescriptionField),
    DeveloperDataId(DeveloperDataIdField),
    Custom(CustomField),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CustomField {
    pub name: Option<String>,
    pub units: Option<String>,
}

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

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use super::*;

    #[test]
    fn test_global_message_from_u16_is_explicit() {
        let mut missing_variants = Vec::new();
        for variant in GlobalMessage::iter() {
            let mut found = false;
            for value in 0..u16::MAX {
                if discriminant(&GlobalMessage::from(value)) == discriminant(&variant) {
                    found = true;
                    break;
                }
            }
            if found == false {
                missing_variants.push(variant);
            }
        }

        assert!(
            missing_variants.is_empty(),
            "Variants missing in GlobalMessage::from<u16>: {:?}",
            missing_variants
        );
    }
}
