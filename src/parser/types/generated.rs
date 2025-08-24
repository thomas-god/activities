#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::upper_case_acronyms)]

use crate::parser::definition::Endianness;
use crate::parser::types::{
    parse_byte_array as parse_byte, parse_float32, parse_float64, parse_sint16, parse_sint32,
    parse_sint64, parse_sint8, parse_string, parse_uint16, parse_uint16z, parse_uint32,
    parse_uint32z, parse_uint64, parse_uint64z, parse_uint8, parse_uint8z, parse_unknown,
    DataTypeError,
};
use crate::{parser::reader::Reader, BaseDataType, BaseDataValue};

#[derive(Debug, PartialEq)]
pub enum DataValue {
    Base(BaseDataValue),
    Enum(FitEnum),
}

#[derive(Debug, PartialEq)]
pub enum FitEnum {
    File(File),
    MesgNum(MesgNum),
    FileFlags(FileFlags),
    MesgCount(MesgCount),
    DateTime(DateTime),
    LocalDateTime(LocalDateTime),
    MessageIndex(MessageIndex),
    DeviceIndex(DeviceIndex),
    Gender(Gender),
    Language(Language),
    DisplayMeasure(DisplayMeasure),
    DisplayHeart(DisplayHeart),
    DisplayPower(DisplayPower),
    DisplayPosition(DisplayPosition),
    Switch(Switch),
    Sport(Sport),
    SportBits0(SportBits0),
    SubSport(SubSport),
    SportEvent(SportEvent),
    Activity(Activity),
    Intensity(Intensity),
    SessionTrigger(SessionTrigger),
    LapTrigger(LapTrigger),
    TimeMode(TimeMode),
    BacklightMode(BacklightMode),
    DateMode(DateMode),
    BacklightTimeout(BacklightTimeout),
    Event(Event),
    EventType(EventType),
    Tone(Tone),
    ActivityClass(ActivityClass),
    HrZoneCalc(HrZoneCalc),
    PwrZoneCalc(PwrZoneCalc),
    WktStepDuration(WktStepDuration),
    WktStepTarget(WktStepTarget),
    Goal(Goal),
    GoalRecurrence(GoalRecurrence),
    GoalSource(GoalSource),
    Schedule(Schedule),
    CoursePoint(CoursePoint),
    Manufacturer(Manufacturer),
    AntNetwork(AntNetwork),
    WorkoutCapabilities(WorkoutCapabilities),
    BatteryStatus(BatteryStatus),
    HrType(HrType),
    CourseCapabilities(CourseCapabilities),
    Weight(Weight),
    BpStatus(BpStatus),
    UserLocalId(UserLocalId),
    SwimStroke(SwimStroke),
    ActivityType(ActivityType),
    ActivitySubtype(ActivitySubtype),
    ActivityLevel(ActivityLevel),
    Side(Side),
    LeftRightBalance(LeftRightBalance),
    LeftRightBalance100(LeftRightBalance100),
    LengthType(LengthType),
    DayOfWeek(DayOfWeek),
    ConnectivityCapabilities(ConnectivityCapabilities),
    WeatherReport(WeatherReport),
    WeatherStatus(WeatherStatus),
    WeatherSeverity(WeatherSeverity),
    WeatherSevereType(WeatherSevereType),
    LocaltimeIntoDay(LocaltimeIntoDay),
    StrokeType(StrokeType),
    BodyLocation(BodyLocation),
    SegmentLapStatus(SegmentLapStatus),
    SegmentLeaderboardType(SegmentLeaderboardType),
    SegmentDeleteStatus(SegmentDeleteStatus),
    SegmentSelectionType(SegmentSelectionType),
    SourceType(SourceType),
    AntChannelId(AntChannelId),
    DisplayOrientation(DisplayOrientation),
    WorkoutEquipment(WorkoutEquipment),
    WatchfaceMode(WatchfaceMode),
    CameraEventType(CameraEventType),
    SensorType(SensorType),
    CameraOrientationType(CameraOrientationType),
    AttitudeStage(AttitudeStage),
    AttitudeValidity(AttitudeValidity),
    AutoSyncFrequency(AutoSyncFrequency),
    ExdLayout(ExdLayout),
    ExdDisplayType(ExdDisplayType),
    ExdDataUnits(ExdDataUnits),
    ExdQualifiers(ExdQualifiers),
    ExdDescriptors(ExdDescriptors),
    AutoActivityDetect(AutoActivityDetect),
    FitBaseType(FitBaseType),
    FitBaseUnit(FitBaseUnit),
    SetType(SetType),
    MaxMetCategory(MaxMetCategory),
    ExerciseCategory(ExerciseCategory),
    WaterType(WaterType),
    TissueModelType(TissueModelType),
    DiveGasStatus(DiveGasStatus),
    DiveAlarmType(DiveAlarmType),
    DiveBacklightMode(DiveBacklightMode),
    SleepLevel(SleepLevel),
    Spo2MeasurementType(Spo2MeasurementType),
    CcrSetpointSwitchMode(CcrSetpointSwitchMode),
    DiveGasMode(DiveGasMode),
    ProjectileType(ProjectileType),
    SplitType(SplitType),
    ClimbProEvent(ClimbProEvent),
    GasConsumptionRateType(GasConsumptionRateType),
    TapSensitivity(TapSensitivity),
    RadarThreatLevelType(RadarThreatLevelType),
    MaxMetSpeedSource(MaxMetSpeedSource),
    MaxMetHeartRateSource(MaxMetHeartRateSource),
    HrvStatus(HrvStatus),
    NoFlyTimeMode(NoFlyTimeMode),
}

#[derive(Debug, PartialEq)]
pub enum File {
    ExdConfiguration,
    SegmentList,
    MonitoringB,
    Sport,
    Settings,
    Goals,
    MfgRangeMin,
    MonitoringA,
    Activity,
    MfgRangeMax,
    Workout,
    Totals,
    Segment,
    ActivitySummary,
    Course,
    BloodPressure,
    Device,
    Schedules,
    MonitoringDaily,
    Weight,
    UnknownVariant,
}
impl File {
    pub fn from(content: u8) -> File {
        match content {
            40 => File::ExdConfiguration,
            35 => File::SegmentList,
            32 => File::MonitoringB,
            3 => File::Sport,
            2 => File::Settings,
            11 => File::Goals,
            247 => File::MfgRangeMin,
            15 => File::MonitoringA,
            4 => File::Activity,
            254 => File::MfgRangeMax,
            5 => File::Workout,
            10 => File::Totals,
            34 => File::Segment,
            20 => File::ActivitySummary,
            6 => File::Course,
            14 => File::BloodPressure,
            1 => File::Device,
            7 => File::Schedules,
            28 => File::MonitoringDaily,
            9 => File::Weight,
            _ => File::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::File(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum MesgNum {
    FileCreator,
    Monitoring,
    CadenceZone,
    WorkoutSession,
    ThreeDSensorCalibration,
    ObdiiData,
    AntTx,
    AntChannelId,
    HsaSpo2Data,
    TrainingFile,
    HsaGyroscopeData,
    DeviceSettings,
    Totals,
    DiveGas,
    SegmentPoint,
    BarometerData,
    PowerZone,
    DeveloperDataId,
    Lap,
    Length,
    WatchfaceSettings,
    NmeaSentence,
    AadAccelFeatures,
    SlaveDevice,
    WeatherAlert,
    Course,
    Record,
    Workout,
    SegmentFile,
    VideoDescription,
    DiveSettings,
    Split,
    Session,
    MagnetometerData,
    CameraEvent,
    TankSummary,
    ChronoShotData,
    ZonesTarget,
    SegmentLap,
    TimeInZone,
    Pad,
    DiveSummary,
    RawBbi,
    FileId,
    Capabilities,
    ExdScreenConfiguration,
    HrvValue,
    Activity,
    DeviceInfo,
    ExdDataFieldConfiguration,
    MfgRangeMin,
    DeviceAuxBatteryInfo,
    AccelerometerData,
    TankUpdate,
    OneDSensorCalibration,
    OhrSettings,
    MfgRangeMax,
    AntRx,
    TimestampCorrelation,
    BikeProfile,
    FieldCapabilities,
    VideoTitle,
    Software,
    BeatIntervals,
    Hr,
    AviationAttitude,
    MonitoringInfo,
    HrmProfile,
    UserProfile,
    Schedule,
    HsaStressData,
    HsaEvent,
    RespirationRate,
    Sport,
    MesgCapabilities,
    HsaConfigurationData,
    Event,
    StressLevel,
    BloodPressure,
    GpsMetadata,
    FieldDescription,
    CoursePoint,
    Connectivity,
    GyroscopeData,
    WeatherConditions,
    ExerciseTitle,
    HsaStepData,
    SplitSummary,
    SpeedZone,
    HsaRespirationData,
    MemoGlob,
    WorkoutStep,
    HrvStatusSummary,
    FileCapabilities,
    TrainingSettings,
    VideoFrame,
    Hrv,
    HsaHeartRateData,
    HrZone,
    DiveAlarm,
    Jump,
    HsaWristTemperatureData,
    MaxMetData,
    SleepLevel,
    ChronoShotSession,
    ClimbPro,
    SkinTempOvernight,
    HsaBodyBatteryData,
    VideoClip,
    MetZone,
    Goal,
    MonitoringHrData,
    SegmentLeaderboardEntry,
    WeightScale,
    SleepAssessment,
    SdmProfile,
    ExdDataConceptConfiguration,
    Set,
    SegmentId,
    Video,
    Spo2Data,
    HsaAccelerometerData,
    DiveApneaAlarm,
    UnknownVariant,
}
impl MesgNum {
    pub fn from(content: u16) -> MesgNum {
        match content {
            49 => MesgNum::FileCreator,
            55 => MesgNum::Monitoring,
            131 => MesgNum::CadenceZone,
            158 => MesgNum::WorkoutSession,
            167 => MesgNum::ThreeDSensorCalibration,
            174 => MesgNum::ObdiiData,
            81 => MesgNum::AntTx,
            82 => MesgNum::AntChannelId,
            305 => MesgNum::HsaSpo2Data,
            72 => MesgNum::TrainingFile,
            376 => MesgNum::HsaGyroscopeData,
            2 => MesgNum::DeviceSettings,
            33 => MesgNum::Totals,
            259 => MesgNum::DiveGas,
            150 => MesgNum::SegmentPoint,
            209 => MesgNum::BarometerData,
            9 => MesgNum::PowerZone,
            207 => MesgNum::DeveloperDataId,
            19 => MesgNum::Lap,
            101 => MesgNum::Length,
            159 => MesgNum::WatchfaceSettings,
            177 => MesgNum::NmeaSentence,
            289 => MesgNum::AadAccelFeatures,
            106 => MesgNum::SlaveDevice,
            129 => MesgNum::WeatherAlert,
            31 => MesgNum::Course,
            20 => MesgNum::Record,
            26 => MesgNum::Workout,
            151 => MesgNum::SegmentFile,
            186 => MesgNum::VideoDescription,
            258 => MesgNum::DiveSettings,
            312 => MesgNum::Split,
            18 => MesgNum::Session,
            208 => MesgNum::MagnetometerData,
            161 => MesgNum::CameraEvent,
            323 => MesgNum::TankSummary,
            388 => MesgNum::ChronoShotData,
            7 => MesgNum::ZonesTarget,
            142 => MesgNum::SegmentLap,
            216 => MesgNum::TimeInZone,
            105 => MesgNum::Pad,
            268 => MesgNum::DiveSummary,
            372 => MesgNum::RawBbi,
            0 => MesgNum::FileId,
            1 => MesgNum::Capabilities,
            200 => MesgNum::ExdScreenConfiguration,
            371 => MesgNum::HrvValue,
            34 => MesgNum::Activity,
            23 => MesgNum::DeviceInfo,
            201 => MesgNum::ExdDataFieldConfiguration,
            65280 => MesgNum::MfgRangeMin,
            375 => MesgNum::DeviceAuxBatteryInfo,
            165 => MesgNum::AccelerometerData,
            319 => MesgNum::TankUpdate,
            210 => MesgNum::OneDSensorCalibration,
            188 => MesgNum::OhrSettings,
            65534 => MesgNum::MfgRangeMax,
            80 => MesgNum::AntRx,
            162 => MesgNum::TimestampCorrelation,
            6 => MesgNum::BikeProfile,
            39 => MesgNum::FieldCapabilities,
            185 => MesgNum::VideoTitle,
            35 => MesgNum::Software,
            290 => MesgNum::BeatIntervals,
            132 => MesgNum::Hr,
            178 => MesgNum::AviationAttitude,
            103 => MesgNum::MonitoringInfo,
            4 => MesgNum::HrmProfile,
            3 => MesgNum::UserProfile,
            28 => MesgNum::Schedule,
            306 => MesgNum::HsaStressData,
            315 => MesgNum::HsaEvent,
            297 => MesgNum::RespirationRate,
            12 => MesgNum::Sport,
            38 => MesgNum::MesgCapabilities,
            389 => MesgNum::HsaConfigurationData,
            21 => MesgNum::Event,
            227 => MesgNum::StressLevel,
            51 => MesgNum::BloodPressure,
            160 => MesgNum::GpsMetadata,
            206 => MesgNum::FieldDescription,
            32 => MesgNum::CoursePoint,
            127 => MesgNum::Connectivity,
            164 => MesgNum::GyroscopeData,
            128 => MesgNum::WeatherConditions,
            264 => MesgNum::ExerciseTitle,
            304 => MesgNum::HsaStepData,
            313 => MesgNum::SplitSummary,
            53 => MesgNum::SpeedZone,
            307 => MesgNum::HsaRespirationData,
            145 => MesgNum::MemoGlob,
            27 => MesgNum::WorkoutStep,
            370 => MesgNum::HrvStatusSummary,
            37 => MesgNum::FileCapabilities,
            13 => MesgNum::TrainingSettings,
            169 => MesgNum::VideoFrame,
            78 => MesgNum::Hrv,
            308 => MesgNum::HsaHeartRateData,
            8 => MesgNum::HrZone,
            262 => MesgNum::DiveAlarm,
            285 => MesgNum::Jump,
            409 => MesgNum::HsaWristTemperatureData,
            229 => MesgNum::MaxMetData,
            275 => MesgNum::SleepLevel,
            387 => MesgNum::ChronoShotSession,
            317 => MesgNum::ClimbPro,
            398 => MesgNum::SkinTempOvernight,
            314 => MesgNum::HsaBodyBatteryData,
            187 => MesgNum::VideoClip,
            10 => MesgNum::MetZone,
            15 => MesgNum::Goal,
            211 => MesgNum::MonitoringHrData,
            149 => MesgNum::SegmentLeaderboardEntry,
            30 => MesgNum::WeightScale,
            346 => MesgNum::SleepAssessment,
            5 => MesgNum::SdmProfile,
            202 => MesgNum::ExdDataConceptConfiguration,
            225 => MesgNum::Set,
            148 => MesgNum::SegmentId,
            184 => MesgNum::Video,
            269 => MesgNum::Spo2Data,
            302 => MesgNum::HsaAccelerometerData,
            393 => MesgNum::DiveApneaAlarm,
            _ => MesgNum::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::MesgNum(Self::from(
                reader.next_u16(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum FileFlags {
    Write,
    Erase,
    Read,
    UnknownVariant,
}
impl FileFlags {
    pub fn from(content: u8) -> FileFlags {
        match content {
            4 => FileFlags::Write,
            8 => FileFlags::Erase,
            2 => FileFlags::Read,
            _ => FileFlags::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::FileFlags(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum MesgCount {
    MaxPerFile,
    MaxPerFileType,
    NumPerFile,
    UnknownVariant,
}
impl MesgCount {
    pub fn from(content: u8) -> MesgCount {
        match content {
            1 => MesgCount::MaxPerFile,
            2 => MesgCount::MaxPerFileType,
            0 => MesgCount::NumPerFile,
            _ => MesgCount::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::MesgCount(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DateTime {
    Min,
    UnknownVariant,
}
impl DateTime {
    pub fn from(content: u32) -> DateTime {
        match content {
            268435456 => DateTime::Min,
            _ => DateTime::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DateTime(Self::from(
                reader.next_u32(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum LocalDateTime {
    Min,
    UnknownVariant,
}
impl LocalDateTime {
    pub fn from(content: u32) -> LocalDateTime {
        match content {
            268435456 => LocalDateTime::Min,
            _ => LocalDateTime::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::LocalDateTime(Self::from(
                reader.next_u32(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum MessageIndex {
    Selected,
    Reserved,
    Mask,
    UnknownVariant,
}
impl MessageIndex {
    pub fn from(content: u16) -> MessageIndex {
        match content {
            32768 => MessageIndex::Selected,
            28672 => MessageIndex::Reserved,
            4095 => MessageIndex::Mask,
            _ => MessageIndex::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::MessageIndex(Self::from(
                reader.next_u16(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DeviceIndex {
    Creator,
    UnknownVariant,
}
impl DeviceIndex {
    pub fn from(content: u8) -> DeviceIndex {
        match content {
            0 => DeviceIndex::Creator,
            _ => DeviceIndex::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DeviceIndex(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Gender {
    Male,
    Female,
    UnknownVariant,
}
impl Gender {
    pub fn from(content: u8) -> Gender {
        match content {
            1 => Gender::Male,
            0 => Gender::Female,
            _ => Gender::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Gender(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Language {
    Korean,
    German,
    Chinese,
    Bulgarian,
    Croatian,
    Hungarian,
    Turkish,
    Danish,
    Japanese,
    Romanian,
    Farsi,
    Mongolian,
    Swedish,
    Italian,
    Norwegian,
    Dutch,
    Portuguese,
    Slovenian,
    English,
    Slovakian,
    Russian,
    Arabic,
    Polish,
    Ukrainian,
    Taiwanese,
    Thai,
    French,
    Spanish,
    Czech,
    Latvian,
    BrazilianPortuguese,
    Indonesian,
    Malaysian,
    Hebrew,
    Vietnamese,
    Burmese,
    Greek,
    Custom,
    Finnish,
    UnknownVariant,
}
impl Language {
    pub fn from(content: u8) -> Language {
        match content {
            28 => Language::Korean,
            3 => Language::German,
            26 => Language::Chinese,
            24 => Language::Bulgarian,
            5 => Language::Croatian,
            11 => Language::Hungarian,
            19 => Language::Turkish,
            7 => Language::Danish,
            27 => Language::Japanese,
            25 => Language::Romanian,
            23 => Language::Farsi,
            37 => Language::Mongolian,
            17 => Language::Swedish,
            2 => Language::Italian,
            12 => Language::Norwegian,
            8 => Language::Dutch,
            14 => Language::Portuguese,
            16 => Language::Slovenian,
            0 => Language::English,
            15 => Language::Slovakian,
            18 => Language::Russian,
            22 => Language::Arabic,
            13 => Language::Polish,
            21 => Language::Ukrainian,
            29 => Language::Taiwanese,
            30 => Language::Thai,
            1 => Language::French,
            4 => Language::Spanish,
            6 => Language::Czech,
            20 => Language::Latvian,
            32 => Language::BrazilianPortuguese,
            33 => Language::Indonesian,
            34 => Language::Malaysian,
            31 => Language::Hebrew,
            35 => Language::Vietnamese,
            36 => Language::Burmese,
            10 => Language::Greek,
            254 => Language::Custom,
            9 => Language::Finnish,
            _ => Language::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Language(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DisplayMeasure {
    Nautical,
    Statute,
    Metric,
    UnknownVariant,
}
impl DisplayMeasure {
    pub fn from(content: u8) -> DisplayMeasure {
        match content {
            2 => DisplayMeasure::Nautical,
            1 => DisplayMeasure::Statute,
            0 => DisplayMeasure::Metric,
            _ => DisplayMeasure::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DisplayMeasure(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DisplayHeart {
    Max,
    Bpm,
    Reserve,
    UnknownVariant,
}
impl DisplayHeart {
    pub fn from(content: u8) -> DisplayHeart {
        match content {
            1 => DisplayHeart::Max,
            0 => DisplayHeart::Bpm,
            2 => DisplayHeart::Reserve,
            _ => DisplayHeart::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DisplayHeart(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DisplayPower {
    Watts,
    PercentFtp,
    UnknownVariant,
}
impl DisplayPower {
    pub fn from(content: u8) -> DisplayPower {
        match content {
            0 => DisplayPower::Watts,
            1 => DisplayPower::PercentFtp,
            _ => DisplayPower::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DisplayPower(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DisplayPosition {
    MgrsGrid,
    IndonesianSouthern,
    IrishTransverse,
    BritishGrid,
    NewZealandTransverse,
    IndiaZoneIA,
    QatarGrid,
    SwedishGrid,
    GermanGrid,
    FinnishGrid,
    SouthAfricanGrid,
    TaiwanGrid,
    SwissGrid,
    Degree,
    IndiaZoneIB,
    IndiaZoneIVB,
    LatvianGrid,
    IndiaZoneIIIB,
    AustrianGrid,
    IcelandicGrid,
    IndiaZoneIIB,
    Loran,
    IndiaZoneIVA,
    IndonesianIrian,
    MaidenheadGrid,
    SwedishRef99Grid,
    UtmUpsGrid,
    NewZealandGrid,
    DutchGrid,
    HungarianGrid,
    UnitedStatesGrid,
    DegreeMinute,
    BorneoRso,
    IrishGrid,
    IndiaZoneIIA,
    IndiaZoneIIIA,
    DegreeMinuteSecond,
    IndiaZone0,
    IndonesianEquatorial,
    ModifiedSwedishGrid,
    WestMalayan,
    EstonianGrid,
    UnknownVariant,
}
impl DisplayPosition {
    pub fn from(content: u8) -> DisplayPosition {
        match content {
            26 => DisplayPosition::MgrsGrid,
            12 => DisplayPosition::IndonesianSouthern,
            22 => DisplayPosition::IrishTransverse,
            4 => DisplayPosition::BritishGrid,
            28 => DisplayPosition::NewZealandTransverse,
            14 => DisplayPosition::IndiaZoneIA,
            29 => DisplayPosition::QatarGrid,
            31 => DisplayPosition::SwedishGrid,
            8 => DisplayPosition::GermanGrid,
            7 => DisplayPosition::FinnishGrid,
            32 => DisplayPosition::SouthAfricanGrid,
            34 => DisplayPosition::TaiwanGrid,
            33 => DisplayPosition::SwissGrid,
            0 => DisplayPosition::Degree,
            15 => DisplayPosition::IndiaZoneIB,
            21 => DisplayPosition::IndiaZoneIVB,
            40 => DisplayPosition::LatvianGrid,
            19 => DisplayPosition::IndiaZoneIIIB,
            3 => DisplayPosition::AustrianGrid,
            9 => DisplayPosition::IcelandicGrid,
            17 => DisplayPosition::IndiaZoneIIB,
            24 => DisplayPosition::Loran,
            20 => DisplayPosition::IndiaZoneIVA,
            11 => DisplayPosition::IndonesianIrian,
            25 => DisplayPosition::MaidenheadGrid,
            41 => DisplayPosition::SwedishRef99Grid,
            36 => DisplayPosition::UtmUpsGrid,
            27 => DisplayPosition::NewZealandGrid,
            5 => DisplayPosition::DutchGrid,
            6 => DisplayPosition::HungarianGrid,
            35 => DisplayPosition::UnitedStatesGrid,
            1 => DisplayPosition::DegreeMinute,
            38 => DisplayPosition::BorneoRso,
            23 => DisplayPosition::IrishGrid,
            16 => DisplayPosition::IndiaZoneIIA,
            18 => DisplayPosition::IndiaZoneIIIA,
            2 => DisplayPosition::DegreeMinuteSecond,
            13 => DisplayPosition::IndiaZone0,
            10 => DisplayPosition::IndonesianEquatorial,
            30 => DisplayPosition::ModifiedSwedishGrid,
            37 => DisplayPosition::WestMalayan,
            39 => DisplayPosition::EstonianGrid,
            _ => DisplayPosition::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DisplayPosition(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Switch {
    Auto,
    Off,
    On,
    UnknownVariant,
}
impl Switch {
    pub fn from(content: u8) -> Switch {
        match content {
            2 => Switch::Auto,
            0 => Switch::Off,
            1 => Switch::On,
            _ => Switch::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Switch(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Sport {
    Hiking,
    Tennis,
    Driving,
    Tactical,
    Kitesurfing,
    Baseball,
    JumpRope,
    CrossCountrySkiing,
    Sailing,
    Rugby,
    Volleyball,
    Surfing,
    DiscGolf,
    Motorcycling,
    Snorkeling,
    Multisport,
    Running,
    Training,
    EBiking,
    Flying,
    Hiit,
    Hunting,
    Snowboarding,
    Paddling,
    IceSkating,
    Racket,
    Transition,
    AlpineSkiing,
    Hockey,
    Swimming,
    Rowing,
    WaterTubing,
    FitnessEquipment,
    Fishing,
    Snowshoeing,
    Wakeboarding,
    InlineSkating,
    Cycling,
    Rafting,
    AmericanFootball,
    StandUpPaddleboarding,
    Mountaineering,
    WheelchairPushRun,
    Meditation,
    Lacrosse,
    Windsurfing,
    HorsebackRiding,
    Generic,
    All,
    Diving,
    Walking,
    RockClimbing,
    Boating,
    SkyDiving,
    WaterSkiing,
    HangGliding,
    Cricket,
    FloorClimbing,
    Jumpmaster,
    Snowmobiling,
    Golf,
    Kayaking,
    Boxing,
    Wakesurfing,
    MixedMartialArts,
    Basketball,
    Soccer,
    Dance,
    WheelchairPushWalk,
    UnknownVariant,
}
impl Sport {
    pub fn from(content: u8) -> Sport {
        match content {
            17 => Sport::Hiking,
            8 => Sport::Tennis,
            24 => Sport::Driving,
            45 => Sport::Tactical,
            44 => Sport::Kitesurfing,
            49 => Sport::Baseball,
            84 => Sport::JumpRope,
            12 => Sport::CrossCountrySkiing,
            32 => Sport::Sailing,
            72 => Sport::Rugby,
            75 => Sport::Volleyball,
            38 => Sport::Surfing,
            69 => Sport::DiscGolf,
            22 => Sport::Motorcycling,
            82 => Sport::Snorkeling,
            18 => Sport::Multisport,
            1 => Sport::Running,
            10 => Sport::Training,
            21 => Sport::EBiking,
            20 => Sport::Flying,
            62 => Sport::Hiit,
            28 => Sport::Hunting,
            14 => Sport::Snowboarding,
            19 => Sport::Paddling,
            33 => Sport::IceSkating,
            64 => Sport::Racket,
            3 => Sport::Transition,
            13 => Sport::AlpineSkiing,
            73 => Sport::Hockey,
            5 => Sport::Swimming,
            15 => Sport::Rowing,
            76 => Sport::WaterTubing,
            4 => Sport::FitnessEquipment,
            29 => Sport::Fishing,
            35 => Sport::Snowshoeing,
            39 => Sport::Wakeboarding,
            30 => Sport::InlineSkating,
            2 => Sport::Cycling,
            42 => Sport::Rafting,
            9 => Sport::AmericanFootball,
            37 => Sport::StandUpPaddleboarding,
            16 => Sport::Mountaineering,
            66 => Sport::WheelchairPushRun,
            67 => Sport::Meditation,
            74 => Sport::Lacrosse,
            43 => Sport::Windsurfing,
            27 => Sport::HorsebackRiding,
            0 => Sport::Generic,
            254 => Sport::All,
            53 => Sport::Diving,
            11 => Sport::Walking,
            31 => Sport::RockClimbing,
            23 => Sport::Boating,
            34 => Sport::SkyDiving,
            40 => Sport::WaterSkiing,
            26 => Sport::HangGliding,
            71 => Sport::Cricket,
            48 => Sport::FloorClimbing,
            46 => Sport::Jumpmaster,
            36 => Sport::Snowmobiling,
            25 => Sport::Golf,
            41 => Sport::Kayaking,
            47 => Sport::Boxing,
            77 => Sport::Wakesurfing,
            80 => Sport::MixedMartialArts,
            6 => Sport::Basketball,
            7 => Sport::Soccer,
            83 => Sport::Dance,
            65 => Sport::WheelchairPushWalk,
            _ => Sport::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Sport(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SportBits0 {
    Cycling,
    Soccer,
    Running,
    Basketball,
    Generic,
    Transition,
    FitnessEquipment,
    Swimming,
    UnknownVariant,
}
impl SportBits0 {
    pub fn from(content: u8) -> SportBits0 {
        match content {
            4 => SportBits0::Cycling,
            128 => SportBits0::Soccer,
            2 => SportBits0::Running,
            64 => SportBits0::Basketball,
            1 => SportBits0::Generic,
            8 => SportBits0::Transition,
            16 => SportBits0::FitnessEquipment,
            32 => SportBits0::Swimming,
            _ => SportBits0::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SportBits0(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SubSport {
    Treadmill,
    GravelCycling,
    Atv,
    Obstacle,
    Track,
    TrackCycling,
    RunToBikeTransition,
    Trail,
    Commuting,
    VirtualActivity,
    Breathing,
    FlyCanopy,
    FlyParamotor,
    IndoorRowing,
    WarmUp,
    Motocross,
    Squash,
    Racquetball,
    FlyPressurized,
    HandCycling,
    IndoorRunning,
    SailRace,
    Generic,
    Map,
    Padel,
    IndoorClimbing,
    Bouldering,
    IndoorCycling,
    BikeToRunTransition,
    OpenWater,
    SingleGasDiving,
    FlexibilityTraining,
    Challenge,
    GaugeDiving,
    TrackMe,
    IndoorWalking,
    CasualWalking,
    All,
    IndoorHandCycling,
    IndoorWheelchairRun,
    Wingsuit,
    Yoga,
    Whitewater,
    Pickleball,
    Spin,
    Road,
    SkateSkiing,
    Elliptical,
    Navigate,
    Pilates,
    Emom,
    FlyWx,
    Badminton,
    Resort,
    EBikeFitness,
    FlyIfr,
    Downhill,
    Backcountry,
    FlyTimer,
    ApneaHunting,
    StrengthTraining,
    IndoorSkiing,
    Bmx,
    Mountain,
    Hiit,
    TableTennis,
    MixedSurface,
    FlyParaglide,
    FlyNavigate,
    Street,
    SwimToBikeTransition,
    CardioTraining,
    Cyclocross,
    Recumbent,
    Match,
    SpeedWalking,
    Tabata,
    Amrap,
    IndoorWheelchairWalk,
    FlyAltimeter,
    FlyVfr,
    RcDrone,
    StairClimbing,
    Exercise,
    EBikeMountain,
    LapSwimming,
    MultiGasDiving,
    Ultra,
    ApneaDiving,
    UnknownVariant,
}
impl SubSport {
    pub fn from(content: u8) -> SubSport {
        match content {
            1 => SubSport::Treadmill,
            46 => SubSport::GravelCycling,
            35 => SubSport::Atv,
            59 => SubSport::Obstacle,
            4 => SubSport::Track,
            13 => SubSport::TrackCycling,
            33 => SubSport::RunToBikeTransition,
            3 => SubSport::Trail,
            48 => SubSport::Commuting,
            58 => SubSport::VirtualActivity,
            62 => SubSport::Breathing,
            110 => SubSport::FlyCanopy,
            112 => SubSport::FlyParamotor,
            14 => SubSport::IndoorRowing,
            21 => SubSport::WarmUp,
            36 => SubSport::Motocross,
            94 => SubSport::Squash,
            96 => SubSport::Racquetball,
            113 => SubSport::FlyPressurized,
            12 => SubSport::HandCycling,
            45 => SubSport::IndoorRunning,
            65 => SubSport::SailRace,
            0 => SubSport::Generic,
            52 => SubSport::Map,
            85 => SubSport::Padel,
            68 => SubSport::IndoorClimbing,
            69 => SubSport::Bouldering,
            6 => SubSport::IndoorCycling,
            32 => SubSport::BikeToRunTransition,
            18 => SubSport::OpenWater,
            53 => SubSport::SingleGasDiving,
            19 => SubSport::FlexibilityTraining,
            24 => SubSport::Challenge,
            55 => SubSport::GaugeDiving,
            51 => SubSport::TrackMe,
            27 => SubSport::IndoorWalking,
            30 => SubSport::CasualWalking,
            254 => SubSport::All,
            88 => SubSport::IndoorHandCycling,
            87 => SubSport::IndoorWheelchairRun,
            40 => SubSport::Wingsuit,
            43 => SubSport::Yoga,
            41 => SubSport::Whitewater,
            84 => SubSport::Pickleball,
            5 => SubSport::Spin,
            7 => SubSport::Road,
            42 => SubSport::SkateSkiing,
            15 => SubSport::Elliptical,
            50 => SubSport::Navigate,
            44 => SubSport::Pilates,
            74 => SubSport::Emom,
            117 => SubSport::FlyWx,
            95 => SubSport::Badminton,
            38 => SubSport::Resort,
            28 => SubSport::EBikeFitness,
            119 => SubSport::FlyIfr,
            9 => SubSport::Downhill,
            37 => SubSport::Backcountry,
            115 => SubSport::FlyTimer,
            57 => SubSport::ApneaHunting,
            20 => SubSport::StrengthTraining,
            25 => SubSport::IndoorSkiing,
            29 => SubSport::Bmx,
            8 => SubSport::Mountain,
            70 => SubSport::Hiit,
            97 => SubSport::TableTennis,
            49 => SubSport::MixedSurface,
            111 => SubSport::FlyParaglide,
            114 => SubSport::FlyNavigate,
            2 => SubSport::Street,
            34 => SubSport::SwimToBikeTransition,
            26 => SubSport::CardioTraining,
            11 => SubSport::Cyclocross,
            10 => SubSport::Recumbent,
            22 => SubSport::Match,
            31 => SubSport::SpeedWalking,
            75 => SubSport::Tabata,
            73 => SubSport::Amrap,
            86 => SubSport::IndoorWheelchairWalk,
            116 => SubSport::FlyAltimeter,
            118 => SubSport::FlyVfr,
            39 => SubSport::RcDrone,
            16 => SubSport::StairClimbing,
            23 => SubSport::Exercise,
            47 => SubSport::EBikeMountain,
            17 => SubSport::LapSwimming,
            54 => SubSport::MultiGasDiving,
            67 => SubSport::Ultra,
            56 => SubSport::ApneaDiving,
            _ => SubSport::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SubSport(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SportEvent {
    Touring,
    Geocaching,
    SpecialEvent,
    Fitness,
    Transportation,
    Uncategorized,
    Race,
    Recreation,
    Training,
    UnknownVariant,
}
impl SportEvent {
    pub fn from(content: u8) -> SportEvent {
        match content {
            8 => SportEvent::Touring,
            1 => SportEvent::Geocaching,
            5 => SportEvent::SpecialEvent,
            2 => SportEvent::Fitness,
            7 => SportEvent::Transportation,
            0 => SportEvent::Uncategorized,
            4 => SportEvent::Race,
            3 => SportEvent::Recreation,
            6 => SportEvent::Training,
            _ => SportEvent::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SportEvent(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Activity {
    Manual,
    AutoMultiSport,
    UnknownVariant,
}
impl Activity {
    pub fn from(content: u8) -> Activity {
        match content {
            0 => Activity::Manual,
            1 => Activity::AutoMultiSport,
            _ => Activity::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Activity(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Intensity {
    Other,
    Active,
    Rest,
    Recovery,
    Interval,
    Cooldown,
    Warmup,
    UnknownVariant,
}
impl Intensity {
    pub fn from(content: u8) -> Intensity {
        match content {
            6 => Intensity::Other,
            0 => Intensity::Active,
            1 => Intensity::Rest,
            4 => Intensity::Recovery,
            5 => Intensity::Interval,
            3 => Intensity::Cooldown,
            2 => Intensity::Warmup,
            _ => Intensity::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Intensity(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SessionTrigger {
    FitnessEquipment,
    AutoMultiSport,
    Manual,
    ActivityEnd,
    UnknownVariant,
}
impl SessionTrigger {
    pub fn from(content: u8) -> SessionTrigger {
        match content {
            3 => SessionTrigger::FitnessEquipment,
            2 => SessionTrigger::AutoMultiSport,
            1 => SessionTrigger::Manual,
            0 => SessionTrigger::ActivityEnd,
            _ => SessionTrigger::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SessionTrigger(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum LapTrigger {
    PositionMarked,
    PositionLap,
    PositionWaypoint,
    PositionStart,
    Time,
    Distance,
    FitnessEquipment,
    SessionEnd,
    Manual,
    UnknownVariant,
}
impl LapTrigger {
    pub fn from(content: u8) -> LapTrigger {
        match content {
            6 => LapTrigger::PositionMarked,
            4 => LapTrigger::PositionLap,
            5 => LapTrigger::PositionWaypoint,
            3 => LapTrigger::PositionStart,
            1 => LapTrigger::Time,
            2 => LapTrigger::Distance,
            8 => LapTrigger::FitnessEquipment,
            7 => LapTrigger::SessionEnd,
            0 => LapTrigger::Manual,
            _ => LapTrigger::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::LapTrigger(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum TimeMode {
    Hour24WithSeconds,
    Military,
    Hour24,
    Hour12,
    Hour12WithSeconds,
    Utc,
    UnknownVariant,
}
impl TimeMode {
    pub fn from(content: u8) -> TimeMode {
        match content {
            4 => TimeMode::Hour24WithSeconds,
            2 => TimeMode::Military,
            1 => TimeMode::Hour24,
            0 => TimeMode::Hour12,
            3 => TimeMode::Hour12WithSeconds,
            5 => TimeMode::Utc,
            _ => TimeMode::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::TimeMode(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum BacklightMode {
    Off,
    Manual,
    SmartNotifications,
    AutoBrightness,
    KeyAndMessagesAndSmartNotifications,
    KeyAndMessages,
    KeyAndMessagesNight,
    UnknownVariant,
}
impl BacklightMode {
    pub fn from(content: u8) -> BacklightMode {
        match content {
            0 => BacklightMode::Off,
            1 => BacklightMode::Manual,
            4 => BacklightMode::SmartNotifications,
            3 => BacklightMode::AutoBrightness,
            6 => BacklightMode::KeyAndMessagesAndSmartNotifications,
            2 => BacklightMode::KeyAndMessages,
            5 => BacklightMode::KeyAndMessagesNight,
            _ => BacklightMode::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::BacklightMode(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DateMode {
    DayMonth,
    MonthDay,
    UnknownVariant,
}
impl DateMode {
    pub fn from(content: u8) -> DateMode {
        match content {
            0 => DateMode::DayMonth,
            1 => DateMode::MonthDay,
            _ => DateMode::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DateMode(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum BacklightTimeout {
    Infinite,
    UnknownVariant,
}
impl BacklightTimeout {
    pub fn from(content: u8) -> BacklightTimeout {
        match content {
            0 => BacklightTimeout::Infinite,
            _ => BacklightTimeout::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::BacklightTimeout(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Event {
    TankLost,
    Timer,
    BatteryLow,
    DiveGasSwitched,
    VirtualPartnerPace,
    CommTimeout,
    DistanceDurationAlert,
    UserMarker,
    RecoveryHr,
    RiderPositionChange,
    PowerUp,
    Lap,
    Length,
    FrontGearChange,
    TankBatteryLow,
    TankPodDisconnected,
    HrHighAlert,
    Activity,
    CadHighAlert,
    SpeedHighAlert,
    TankPodConnected,
    OffCourse,
    Battery,
    CalorieDurationAlert,
    Calibration,
    ElevLowAlert,
    DiveAlert,
    CoursePoint,
    RearGearChange,
    SpeedLowAlert,
    TimeDurationAlert,
    PowerLowAlert,
    FitnessEquipment,
    TankPressureReserve,
    PowerDown,
    PowerHighAlert,
    TankPressureCritical,
    CadLowAlert,
    RadarThreatAlert,
    Session,
    ElevHighAlert,
    WorkoutStep,
    HrLowAlert,
    SportPoint,
    Workout,
    AutoActivityDetect,
    UnknownVariant,
}
impl Event {
    pub fn from(content: u8) -> Event {
        match content {
            73 => Event::TankLost,
            0 => Event::Timer,
            22 => Event::BatteryLow,
            57 => Event::DiveGasSwitched,
            12 => Event::VirtualPartnerPace,
            47 => Event::CommTimeout,
            24 => Event::DistanceDurationAlert,
            32 => Event::UserMarker,
            21 => Event::RecoveryHr,
            44 => Event::RiderPositionChange,
            6 => Event::PowerUp,
            9 => Event::Lap,
            28 => Event::Length,
            42 => Event::FrontGearChange,
            76 => Event::TankBatteryLow,
            82 => Event::TankPodDisconnected,
            13 => Event::HrHighAlert,
            26 => Event::Activity,
            17 => Event::CadHighAlert,
            15 => Event::SpeedHighAlert,
            81 => Event::TankPodConnected,
            7 => Event::OffCourse,
            11 => Event::Battery,
            25 => Event::CalorieDurationAlert,
            36 => Event::Calibration,
            46 => Event::ElevLowAlert,
            56 => Event::DiveAlert,
            10 => Event::CoursePoint,
            43 => Event::RearGearChange,
            16 => Event::SpeedLowAlert,
            23 => Event::TimeDurationAlert,
            20 => Event::PowerLowAlert,
            27 => Event::FitnessEquipment,
            71 => Event::TankPressureReserve,
            5 => Event::PowerDown,
            19 => Event::PowerHighAlert,
            72 => Event::TankPressureCritical,
            18 => Event::CadLowAlert,
            75 => Event::RadarThreatAlert,
            8 => Event::Session,
            45 => Event::ElevHighAlert,
            4 => Event::WorkoutStep,
            14 => Event::HrLowAlert,
            33 => Event::SportPoint,
            3 => Event::Workout,
            54 => Event::AutoActivityDetect,
            _ => Event::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Event(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum EventType {
    ConsecutiveDepreciated,
    StopAll,
    BeginDepreciated,
    EndAllDepreciated,
    EndDepreciated,
    Start,
    Stop,
    StopDisableAll,
    StopDisable,
    Marker,
    UnknownVariant,
}
impl EventType {
    pub fn from(content: u8) -> EventType {
        match content {
            2 => EventType::ConsecutiveDepreciated,
            4 => EventType::StopAll,
            5 => EventType::BeginDepreciated,
            7 => EventType::EndAllDepreciated,
            6 => EventType::EndDepreciated,
            0 => EventType::Start,
            1 => EventType::Stop,
            9 => EventType::StopDisableAll,
            8 => EventType::StopDisable,
            3 => EventType::Marker,
            _ => EventType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::EventType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Tone {
    Off,
    Vibrate,
    Tone,
    ToneAndVibrate,
    UnknownVariant,
}
impl Tone {
    pub fn from(content: u8) -> Tone {
        match content {
            0 => Tone::Off,
            2 => Tone::Vibrate,
            1 => Tone::Tone,
            3 => Tone::ToneAndVibrate,
            _ => Tone::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Tone(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ActivityClass {
    LevelMax,
    Athlete,
    Level,
    UnknownVariant,
}
impl ActivityClass {
    pub fn from(content: u8) -> ActivityClass {
        match content {
            100 => ActivityClass::LevelMax,
            128 => ActivityClass::Athlete,
            127 => ActivityClass::Level,
            _ => ActivityClass::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ActivityClass(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum HrZoneCalc {
    Custom,
    PercentMaxHr,
    PercentLthr,
    PercentHrr,
    UnknownVariant,
}
impl HrZoneCalc {
    pub fn from(content: u8) -> HrZoneCalc {
        match content {
            0 => HrZoneCalc::Custom,
            1 => HrZoneCalc::PercentMaxHr,
            3 => HrZoneCalc::PercentLthr,
            2 => HrZoneCalc::PercentHrr,
            _ => HrZoneCalc::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::HrZoneCalc(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum PwrZoneCalc {
    Custom,
    PercentFtp,
    UnknownVariant,
}
impl PwrZoneCalc {
    pub fn from(content: u8) -> PwrZoneCalc {
        match content {
            0 => PwrZoneCalc::Custom,
            1 => PwrZoneCalc::PercentFtp,
            _ => PwrZoneCalc::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::PwrZoneCalc(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WktStepDuration {
    Distance,
    TimeOnly,
    RepeatUntilDistance,
    Power3sLessThan,
    RepeatUntilTime,
    Time,
    HrGreaterThan,
    RepeatUntilMaxPowerLastLapLessThan,
    PowerLapGreaterThan,
    TrainingPeaksTss,
    PowerGreaterThan,
    Power30sLessThan,
    Power3sGreaterThan,
    RepeatUntilPowerLastLapLessThan,
    RepeatUntilStepsCmplt,
    RepeatUntilHrLessThan,
    RepeatUntilTrainingPeaksTss,
    Calories,
    Power10sLessThan,
    Power10sGreaterThan,
    RepeatUntilCalories,
    PowerLessThan,
    Power30sGreaterThan,
    PowerLapLessThan,
    RepeatUntilHrGreaterThan,
    RepetitionTime,
    RepeatUntilPowerGreaterThan,
    Open,
    Reps,
    HrLessThan,
    RepeatUntilPowerLessThan,
    UnknownVariant,
}
impl WktStepDuration {
    pub fn from(content: u8) -> WktStepDuration {
        match content {
            1 => WktStepDuration::Distance,
            31 => WktStepDuration::TimeOnly,
            8 => WktStepDuration::RepeatUntilDistance,
            19 => WktStepDuration::Power3sLessThan,
            7 => WktStepDuration::RepeatUntilTime,
            0 => WktStepDuration::Time,
            3 => WktStepDuration::HrGreaterThan,
            18 => WktStepDuration::RepeatUntilMaxPowerLastLapLessThan,
            26 => WktStepDuration::PowerLapGreaterThan,
            16 => WktStepDuration::TrainingPeaksTss,
            15 => WktStepDuration::PowerGreaterThan,
            21 => WktStepDuration::Power30sLessThan,
            22 => WktStepDuration::Power3sGreaterThan,
            17 => WktStepDuration::RepeatUntilPowerLastLapLessThan,
            6 => WktStepDuration::RepeatUntilStepsCmplt,
            10 => WktStepDuration::RepeatUntilHrLessThan,
            27 => WktStepDuration::RepeatUntilTrainingPeaksTss,
            4 => WktStepDuration::Calories,
            20 => WktStepDuration::Power10sLessThan,
            23 => WktStepDuration::Power10sGreaterThan,
            9 => WktStepDuration::RepeatUntilCalories,
            14 => WktStepDuration::PowerLessThan,
            24 => WktStepDuration::Power30sGreaterThan,
            25 => WktStepDuration::PowerLapLessThan,
            11 => WktStepDuration::RepeatUntilHrGreaterThan,
            28 => WktStepDuration::RepetitionTime,
            13 => WktStepDuration::RepeatUntilPowerGreaterThan,
            5 => WktStepDuration::Open,
            29 => WktStepDuration::Reps,
            2 => WktStepDuration::HrLessThan,
            12 => WktStepDuration::RepeatUntilPowerLessThan,
            _ => WktStepDuration::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WktStepDuration(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WktStepTarget {
    Power3s,
    Power,
    Open,
    Cadence,
    Grade,
    Resistance,
    Power30s,
    SwimStroke,
    HeartRate,
    Speed,
    SpeedLap,
    PowerLap,
    Power10s,
    HeartRateLap,
    UnknownVariant,
}
impl WktStepTarget {
    pub fn from(content: u8) -> WktStepTarget {
        match content {
            7 => WktStepTarget::Power3s,
            4 => WktStepTarget::Power,
            2 => WktStepTarget::Open,
            3 => WktStepTarget::Cadence,
            5 => WktStepTarget::Grade,
            6 => WktStepTarget::Resistance,
            9 => WktStepTarget::Power30s,
            11 => WktStepTarget::SwimStroke,
            1 => WktStepTarget::HeartRate,
            0 => WktStepTarget::Speed,
            12 => WktStepTarget::SpeedLap,
            10 => WktStepTarget::PowerLap,
            8 => WktStepTarget::Power10s,
            13 => WktStepTarget::HeartRateLap,
            _ => WktStepTarget::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WktStepTarget(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Goal {
    Time,
    Ascent,
    ActiveMinutes,
    Steps,
    Distance,
    Frequency,
    Calories,
    UnknownVariant,
}
impl Goal {
    pub fn from(content: u8) -> Goal {
        match content {
            0 => Goal::Time,
            5 => Goal::Ascent,
            6 => Goal::ActiveMinutes,
            4 => Goal::Steps,
            1 => Goal::Distance,
            3 => Goal::Frequency,
            2 => Goal::Calories,
            _ => Goal::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Goal(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum GoalRecurrence {
    Daily,
    Off,
    Monthly,
    Weekly,
    Yearly,
    Custom,
    UnknownVariant,
}
impl GoalRecurrence {
    pub fn from(content: u8) -> GoalRecurrence {
        match content {
            1 => GoalRecurrence::Daily,
            0 => GoalRecurrence::Off,
            3 => GoalRecurrence::Monthly,
            2 => GoalRecurrence::Weekly,
            4 => GoalRecurrence::Yearly,
            5 => GoalRecurrence::Custom,
            _ => GoalRecurrence::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::GoalRecurrence(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum GoalSource {
    Auto,
    Community,
    User,
    UnknownVariant,
}
impl GoalSource {
    pub fn from(content: u8) -> GoalSource {
        match content {
            0 => GoalSource::Auto,
            1 => GoalSource::Community,
            2 => GoalSource::User,
            _ => GoalSource::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::GoalSource(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Schedule {
    Course,
    Workout,
    UnknownVariant,
}
impl Schedule {
    pub fn from(content: u8) -> Schedule {
        match content {
            1 => Schedule::Course,
            0 => Schedule::Workout,
            _ => Schedule::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Schedule(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum CoursePoint {
    Crossing,
    Generic,
    MileMarker,
    MeetingSpot,
    Shelter,
    FirstCategory,
    Left,
    Right,
    Toilet,
    Gear,
    Store,
    Valley,
    MiddleFork,
    SharpRight,
    Summit,
    FourthCategory,
    SlightRight,
    UTurn,
    RestArea,
    Obstacle,
    EnergyGel,
    Navaid,
    Water,
    LeftFork,
    HorsCategory,
    Sprint,
    SportsDrink,
    Campsite,
    Tunnel,
    Service,
    SteepIncline,
    Transition,
    Transport,
    Shower,
    ThirdCategory,
    Danger,
    Food,
    AidStation,
    SecondCategory,
    FirstAid,
    RightFork,
    Bridge,
    Alert,
    Overlook,
    SharpCurve,
    Info,
    SegmentStart,
    Straight,
    SlightLeft,
    GeneralDistance,
    Checkpoint,
    SharpLeft,
    SegmentEnd,
    UnknownVariant,
}
impl CoursePoint {
    pub fn from(content: u8) -> CoursePoint {
        match content {
            47 => CoursePoint::Crossing,
            0 => CoursePoint::Generic,
            34 => CoursePoint::MileMarker,
            37 => CoursePoint::MeetingSpot,
            36 => CoursePoint::Shelter,
            13 => CoursePoint::FirstCategory,
            6 => CoursePoint::Left,
            7 => CoursePoint::Right,
            39 => CoursePoint::Toilet,
            41 => CoursePoint::Gear,
            48 => CoursePoint::Store,
            2 => CoursePoint::Valley,
            18 => CoursePoint::MiddleFork,
            22 => CoursePoint::SharpRight,
            1 => CoursePoint::Summit,
            10 => CoursePoint::FourthCategory,
            21 => CoursePoint::SlightRight,
            23 => CoursePoint::UTurn,
            29 => CoursePoint::RestArea,
            46 => CoursePoint::Obstacle,
            32 => CoursePoint::EnergyGel,
            50 => CoursePoint::Navaid,
            3 => CoursePoint::Water,
            16 => CoursePoint::LeftFork,
            14 => CoursePoint::HorsCategory,
            15 => CoursePoint::Sprint,
            33 => CoursePoint::SportsDrink,
            27 => CoursePoint::Campsite,
            44 => CoursePoint::Tunnel,
            31 => CoursePoint::Service,
            43 => CoursePoint::SteepIncline,
            49 => CoursePoint::Transition,
            51 => CoursePoint::Transport,
            40 => CoursePoint::Shower,
            11 => CoursePoint::ThirdCategory,
            5 => CoursePoint::Danger,
            4 => CoursePoint::Food,
            28 => CoursePoint::AidStation,
            12 => CoursePoint::SecondCategory,
            9 => CoursePoint::FirstAid,
            17 => CoursePoint::RightFork,
            45 => CoursePoint::Bridge,
            52 => CoursePoint::Alert,
            38 => CoursePoint::Overlook,
            42 => CoursePoint::SharpCurve,
            53 => CoursePoint::Info,
            24 => CoursePoint::SegmentStart,
            8 => CoursePoint::Straight,
            19 => CoursePoint::SlightLeft,
            30 => CoursePoint::GeneralDistance,
            35 => CoursePoint::Checkpoint,
            20 => CoursePoint::SharpLeft,
            25 => CoursePoint::SegmentEnd,
            _ => CoursePoint::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::CoursePoint(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Manufacturer {
    MaxwellGuider,
    Magura,
    Shapelog,
    Iiiis,
    PhysicalEnterprises,
    Virtugo,
    Form,
    PolarElectro,
    Cycloptim,
    MonarkExercise,
    SpinningMda,
    Magellan,
    BsxAthletics,
    GiantManufacturingCo,
    Vdo,
    YamahaMotors,
    Icg,
    Cardiosport,
    DkCity,
    AeroSensor,
    Ciclosport,
    FaveroElectronics,
    Greenteg,
    Concept2,
    Vasa,
    Coros,
    BodyBikeSmart,
    IfitCom,
    Shimano,
    Omata,
    AlatechTechnologyLtd,
    Watteam,
    LifeTimeFitness,
    Geonaute,
    Laisi,
    Cateye,
    Tanita,
    Coospo,
    Archinoetics,
    Virtualtraining,
    Onelap,
    EoSwimbetter,
    Fitcare,
    Healthandlife,
    CorosByte,
    Ibike,
    TheSufferfest,
    OrekaTraining,
    OctaneFitness,
    Quarq,
    Lezyne,
    Nurvv,
    Beurer,
    TektroRacingProducts,
    MioMagellan,
    RaceRepublic,
    Precor,
    Dabuziduo,
    CleanMobile,
    Blackbird,
    RGTCycling,
    Suunto,
    MiPulse,
    Abawo,
    Cobi,
    ZwiftByte,
    Mywhoosh,
    Bkool,
    Podoon,
    PerceptionDigital,
    DirectionTechnology,
    Pioneer,
    Bryton,
    GarminFr405Antfs,
    Scosche,
    Inpeak,
    Bontrager,
    Cycplus,
    Ezon,
    Metrigear,
    Orangetheory,
    SensitivusGauge,
    GopherSport,
    Osynce,
    KineticSports,
    BrytonSensors,
    VersaDesign,
    KineticByKurt,
    Feedbacksports,
    Peripedal,
    TrueFitness,
    DaradInnovationCorporation,
    Actigraphcorp,
    LatitudeLimited,
    Strava,
    Magneticdays,
    MahleEbikemotion,
    Whoop,
    Heatup,
    Xplova,
    AbsoluteCycling,
    Lifebeam,
    Wellgo,
    Shanyue,
    PorscheEp,
    Decathlon,
    Moxy,
    Look,
    Idt,
    TqSystems,
    Sigeyi,
    Peaksware,
    Fullspeedahead,
    Zephyr,
    Technogym,
    TopactionTechnology,
    Powerbahn,
    AcornProjectsAps,
    FalcoEMotors,
    Hmm,
    Jetblack,
    Chileaf,
    Spantec,
    NciTechnology,
    Igpsport,
    Myzone,
    Wattbike,
    Kyto,
    Kinetic,
    Geoid,
    Dexcom,
    ThitaElektronik,
    Praxisworks,
    Bosch,
    AceSensor,
    Fazua,
    Hilldating,
    Elite,
    Tigrasport,
    Metalogics,
    JohnsonHealthTech,
    Magicshine,
    WahooFitness,
    CampagnoloSrl,
    IdBike,
    LululemonStudio,
    InsideRideTechnologies,
    OneGiantLeap,
    Dynastream,
    Cosinuss,
    Cycliq,
    StarTrac,
    GravaaByte,
    Trailforks,
    TagHeuer,
    Minoura,
    NielsenKellerman,
    Cycligentinc,
    Development,
    Magene,
    TheHurtBox,
    Breakaway,
    MioTechnologyEurope,
    Tomtom,
    Partcarbon,
    Velosense,
    Microprogram,
    BrimBrothers,
    Waterrower,
    Garmin,
    Luxottica,
    Wtek,
    PedalBrain,
    Dynovelo,
    Ictrainer,
    NorthPoleEngineering,
    SeikoEpsonOem,
    Zwift,
    SeikoEpson,
    SoaringTechnology,
    Stryd,
    Bafang,
    Recon,
    LuhongTechnology,
    DynastreamOem,
    ScribeLabs,
    Rotor,
    Syncros,
    Gpulse,
    Ravemen,
    Lsec,
    Specialized,
    Srm,
    Seesense,
    MeilanByte,
    Holux,
    Cannondale,
    Tacx,
    Hammerhead,
    Saris,
    Echowell,
    Magtonic,
    Navman,
    Bf1systems,
    Iqsquare,
    Dayton,
    KeiserFitness,
    Salutron,
    StagesCycling,
    DecathlonByte,
    Zone5cloud,
    IforPowell,
    Timex,
    Sram,
    SoundOfMotion,
    Woodway,
    Gravaa,
    Xelic,
    Leomo,
    Thinkrider,
    TrainerRoad,
    AAndD,
    Nike,
    Evesports,
    LimitsTechnology,
    CitizenSystems,
    SparkHk,
    Saxonar,
    Nautilus,
    Sigmasport,
    LemondFitness,
    Spivi,
    UnknownVariant,
}
impl Manufacturer {
    pub fn from(content: u16) -> Manufacturer {
        match content {
            55 => Manufacturer::MaxwellGuider,
            84 => Manufacturer::Magura,
            291 => Manufacturer::Shapelog,
            51 => Manufacturer::Iiiis,
            65 => Manufacturer::PhysicalEnterprises,
            295 => Manufacturer::Virtugo,
            309 => Manufacturer::Form,
            123 => Manufacturer::PolarElectro,
            335 => Manufacturer::Cycloptim,
            308 => Manufacturer::MonarkExercise,
            323 => Manufacturer::SpinningMda,
            37 => Manufacturer::Magellan,
            98 => Manufacturer::BsxAthletics,
            108 => Manufacturer::GiantManufacturingCo,
            287 => Manufacturer::Vdo,
            304 => Manufacturer::YamahaMotors,
            96 => Manufacturer::Icg,
            20 => Manufacturer::Cardiosport,
            88 => Manufacturer::DkCity,
            325 => Manufacturer::AeroSensor,
            77 => Manufacturer::Ciclosport,
            263 => Manufacturer::FaveroElectronics,
            303 => Manufacturer::Greenteg,
            40 => Manufacturer::Concept2,
            316 => Manufacturer::Vasa,
            294 => Manufacturer::Coros,
            101 => Manufacturer::BodyBikeSmart,
            128 => Manufacturer::IfitCom,
            41 => Manufacturer::Shimano,
            286 => Manufacturer::Omata,
            58 => Manufacturer::AlatechTechnologyLtd,
            261 => Manufacturer::Watteam,
            276 => Manufacturer::LifeTimeFitness,
            61 => Manufacturer::Geonaute,
            149 => Manufacturer::Laisi,
            68 => Manufacturer::Cateye,
            11 => Manufacturer::Tanita,
            135 => Manufacturer::Coospo,
            34 => Manufacturer::Archinoetics,
            284 => Manufacturer::Virtualtraining,
            307 => Manufacturer::Onelap,
            330 => Manufacturer::EoSwimbetter,
            106 => Manufacturer::Fitcare,
            257 => Manufacturer::Healthandlife,
            129 => Manufacturer::CorosByte,
            8 => Manufacturer::Ibike,
            282 => Manufacturer::TheSufferfest,
            319 => Manufacturer::OrekaTraining,
            33 => Manufacturer::OctaneFitness,
            7 => Manufacturer::Quarq,
            258 => Manufacturer::Lezyne,
            300 => Manufacturer::Nurvv,
            19 => Manufacturer::Beurer,
            333 => Manufacturer::TektroRacingProducts,
            272 => Manufacturer::MioMagellan,
            317 => Manufacturer::RaceRepublic,
            266 => Manufacturer::Precor,
            292 => Manufacturer::Dabuziduo,
            26 => Manufacturer::CleanMobile,
            146 => Manufacturer::Blackbird,
            315 => Manufacturer::RGTCycling,
            23 => Manufacturer::Suunto,
            97 => Manufacturer::MiPulse,
            151 => Manufacturer::Abawo,
            270 => Manufacturer::Cobi,
            144 => Manufacturer::ZwiftByte,
            331 => Manufacturer::Mywhoosh,
            67 => Manufacturer::Bkool,
            275 => Manufacturer::Podoon,
            46 => Manufacturer::PerceptionDigital,
            90 => Manufacturer::DirectionTechnology,
            48 => Manufacturer::Pioneer,
            267 => Manufacturer::Bryton,
            2 => Manufacturer::GarminFr405Antfs,
            83 => Manufacturer::Scosche,
            120 => Manufacturer::Inpeak,
            81 => Manufacturer::Bontrager,
            132 => Manufacturer::Cycplus,
            148 => Manufacturer::Ezon,
            17 => Manufacturer::Metrigear,
            119 => Manufacturer::Orangetheory,
            274 => Manufacturer::SensitivusGauge,
            117 => Manufacturer::GopherSport,
            38 => Manufacturer::Osynce,
            139 => Manufacturer::KineticSports,
            112 => Manufacturer::BrytonSensors,
            130 => Manufacturer::VersaDesign,
            290 => Manufacturer::KineticByKurt,
            285 => Manufacturer::Feedbacksports,
            72 => Manufacturer::Peripedal,
            314 => Manufacturer::TrueFitness,
            334 => Manufacturer::DaradInnovationCorporation,
            5759 => Manufacturer::Actigraphcorp,
            113 => Manufacturer::LatitudeLimited,
            265 => Manufacturer::Strava,
            288 => Manufacturer::Magneticdays,
            299 => Manufacturer::MahleEbikemotion,
            305 => Manufacturer::Whoop,
            312 => Manufacturer::Heatup,
            45 => Manufacturer::Xplova,
            329 => Manufacturer::AbsoluteCycling,
            80 => Manufacturer::Lifebeam,
            82 => Manufacturer::Wellgo,
            322 => Manufacturer::Shanyue,
            145 => Manufacturer::PorscheEp,
            310 => Manufacturer::Decathlon,
            76 => Manufacturer::Moxy,
            99 => Manufacturer::Look,
            5 => Manufacturer::Idt,
            141 => Manufacturer::TqSystems,
            134 => Manufacturer::Sigeyi,
            28 => Manufacturer::Peaksware,
            283 => Manufacturer::Fullspeedahead,
            3 => Manufacturer::Zephyr,
            111 => Manufacturer::Technogym,
            104 => Manufacturer::TopactionTechnology,
            78 => Manufacturer::Powerbahn,
            79 => Manufacturer::AcornProjectsAps,
            277 => Manufacturer::FalcoEMotors,
            22 => Manufacturer::Hmm,
            293 => Manufacturer::Jetblack,
            131 => Manufacturer::Chileaf,
            49 => Manufacturer::Spantec,
            125 => Manufacturer::NciTechnology,
            115 => Manufacturer::Igpsport,
            150 => Manufacturer::Myzone,
            73 => Manufacturer::Wattbike,
            138 => Manufacturer::Kyto,
            121 => Manufacturer::Kinetic,
            136 => Manufacturer::Geoid,
            31 => Manufacturer::Dexcom,
            24 => Manufacturer::ThitaElektronik,
            102 => Manufacturer::Praxisworks,
            137 => Manufacturer::Bosch,
            43 => Manufacturer::AceSensor,
            318 => Manufacturer::Fazua,
            324 => Manufacturer::Hilldating,
            86 => Manufacturer::Elite,
            109 => Manufacturer::Tigrasport,
            50 => Manufacturer::Metalogics,
            122 => Manufacturer::JohnsonHealthTech,
            327 => Manufacturer::Magicshine,
            32 => Manufacturer::WahooFitness,
            100 => Manufacturer::CampagnoloSrl,
            62 => Manufacturer::IdBike,
            321 => Manufacturer::LululemonStudio,
            93 => Manufacturer::InsideRideTechnologies,
            42 => Manufacturer::OneGiantLeap,
            15 => Manufacturer::Dynastream,
            105 => Manufacturer::Cosinuss,
            279 => Manufacturer::Cycliq,
            56 => Manufacturer::StarTrac,
            133 => Manufacturer::GravaaByte,
            298 => Manufacturer::Trailforks,
            142 => Manufacturer::TagHeuer,
            278 => Manufacturer::Minoura,
            87 => Manufacturer::NielsenKellerman,
            297 => Manufacturer::Cycligentinc,
            255 => Manufacturer::Development,
            107 => Manufacturer::Magene,
            35 => Manufacturer::TheHurtBox,
            57 => Manufacturer::Breakaway,
            59 => Manufacturer::MioTechnologyEurope,
            71 => Manufacturer::Tomtom,
            92 => Manufacturer::Partcarbon,
            296 => Manufacturer::Velosense,
            301 => Manufacturer::Microprogram,
            44 => Manufacturer::BrimBrothers,
            118 => Manufacturer::Waterrower,
            1 => Manufacturer::Garmin,
            280 => Manufacturer::Luxottica,
            64 => Manufacturer::Wtek,
            27 => Manufacturer::PedalBrain,
            264 => Manufacturer::Dynovelo,
            328 => Manufacturer::Ictrainer,
            66 => Manufacturer::NorthPoleEngineering,
            53 => Manufacturer::SeikoEpsonOem,
            260 => Manufacturer::Zwift,
            52 => Manufacturer::SeikoEpson,
            114 => Manufacturer::SoaringTechnology,
            95 => Manufacturer::Stryd,
            152 => Manufacturer::Bafang,
            262 => Manufacturer::Recon,
            153 => Manufacturer::LuhongTechnology,
            13 => Manufacturer::DynastreamOem,
            259 => Manufacturer::ScribeLabs,
            60 => Manufacturer::Rotor,
            311 => Manufacturer::Syncros,
            25 => Manufacturer::Gpulse,
            332 => Manufacturer::Ravemen,
            320 => Manufacturer::Lsec,
            63 => Manufacturer::Specialized,
            6 => Manufacturer::Srm,
            124 => Manufacturer::Seesense,
            147 => Manufacturer::MeilanByte,
            39 => Manufacturer::Holux,
            313 => Manufacturer::Cannondale,
            89 => Manufacturer::Tacx,
            289 => Manufacturer::Hammerhead,
            9 => Manufacturer::Saris,
            12 => Manufacturer::Echowell,
            91 => Manufacturer::Magtonic,
            269 => Manufacturer::Navman,
            47 => Manufacturer::Bf1systems,
            126 => Manufacturer::Iqsquare,
            4 => Manufacturer::Dayton,
            143 => Manufacturer::KeiserFitness,
            110 => Manufacturer::Salutron,
            69 => Manufacturer::StagesCycling,
            140 => Manufacturer::DecathlonByte,
            302 => Manufacturer::Zone5cloud,
            54 => Manufacturer::IforPowell,
            16 => Manufacturer::Timex,
            268 => Manufacturer::Sram,
            94 => Manufacturer::SoundOfMotion,
            85 => Manufacturer::Woodway,
            306 => Manufacturer::Gravaa,
            18 => Manufacturer::Xelic,
            127 => Manufacturer::Leomo,
            116 => Manufacturer::Thinkrider,
            281 => Manufacturer::TrainerRoad,
            21 => Manufacturer::AAndD,
            326 => Manufacturer::Nike,
            273 => Manufacturer::Evesports,
            103 => Manufacturer::LimitsTechnology,
            36 => Manufacturer::CitizenSystems,
            10 => Manufacturer::SparkHk,
            29 => Manufacturer::Saxonar,
            14 => Manufacturer::Nautilus,
            70 => Manufacturer::Sigmasport,
            30 => Manufacturer::LemondFitness,
            271 => Manufacturer::Spivi,
            _ => Manufacturer::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Manufacturer(Self::from(
                reader.next_u16(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum AntNetwork {
    Antplus,
    Public,
    Antfs,
    Private,
    UnknownVariant,
}
impl AntNetwork {
    pub fn from(content: u8) -> AntNetwork {
        match content {
            1 => AntNetwork::Antplus,
            0 => AntNetwork::Public,
            2 => AntNetwork::Antfs,
            3 => AntNetwork::Private,
            _ => AntNetwork::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::AntNetwork(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WorkoutCapabilities {
    Protected,
    Power,
    Tcx,
    Custom,
    Interval,
    Speed,
    NewLeaf,
    Firstbeat,
    Distance,
    Cadence,
    FitnessEquipment,
    Resistance,
    Grade,
    HeartRate,
    UnknownVariant,
}
impl WorkoutCapabilities {
    pub fn from(content: u32) -> WorkoutCapabilities {
        match content {
            16384 => WorkoutCapabilities::Protected,
            2048 => WorkoutCapabilities::Power,
            32 => WorkoutCapabilities::Tcx,
            2 => WorkoutCapabilities::Custom,
            1 => WorkoutCapabilities::Interval,
            128 => WorkoutCapabilities::Speed,
            16 => WorkoutCapabilities::NewLeaf,
            8 => WorkoutCapabilities::Firstbeat,
            512 => WorkoutCapabilities::Distance,
            1024 => WorkoutCapabilities::Cadence,
            4 => WorkoutCapabilities::FitnessEquipment,
            8192 => WorkoutCapabilities::Resistance,
            4096 => WorkoutCapabilities::Grade,
            256 => WorkoutCapabilities::HeartRate,
            _ => WorkoutCapabilities::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WorkoutCapabilities(Self::from(
                reader.next_u32(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum BatteryStatus {
    Charging,
    Unknown,
    Ok,
    Good,
    New,
    Low,
    Critical,
    UnknownVariant,
}
impl BatteryStatus {
    pub fn from(content: u8) -> BatteryStatus {
        match content {
            6 => BatteryStatus::Charging,
            7 => BatteryStatus::Unknown,
            3 => BatteryStatus::Ok,
            2 => BatteryStatus::Good,
            1 => BatteryStatus::New,
            4 => BatteryStatus::Low,
            5 => BatteryStatus::Critical,
            _ => BatteryStatus::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::BatteryStatus(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum HrType {
    Normal,
    Irregular,
    UnknownVariant,
}
impl HrType {
    pub fn from(content: u8) -> HrType {
        match content {
            0 => HrType::Normal,
            1 => HrType::Irregular,
            _ => HrType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::HrType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum CourseCapabilities {
    Power,
    Training,
    Navigation,
    Processed,
    Bikeway,
    Distance,
    Cadence,
    Aviation,
    Position,
    Time,
    HeartRate,
    Valid,
    UnknownVariant,
}
impl CourseCapabilities {
    pub fn from(content: u32) -> CourseCapabilities {
        match content {
            64 => CourseCapabilities::Power,
            256 => CourseCapabilities::Training,
            512 => CourseCapabilities::Navigation,
            1 => CourseCapabilities::Processed,
            1024 => CourseCapabilities::Bikeway,
            8 => CourseCapabilities::Distance,
            128 => CourseCapabilities::Cadence,
            4096 => CourseCapabilities::Aviation,
            16 => CourseCapabilities::Position,
            4 => CourseCapabilities::Time,
            32 => CourseCapabilities::HeartRate,
            2 => CourseCapabilities::Valid,
            _ => CourseCapabilities::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::CourseCapabilities(Self::from(
                reader.next_u32(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Weight {
    Calculating,
    UnknownVariant,
}
impl Weight {
    pub fn from(content: u16) -> Weight {
        match content {
            65534 => Weight::Calculating,
            _ => Weight::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Weight(Self::from(
                reader.next_u16(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum BpStatus {
    ErrorIrregularHeartRate,
    NoError,
    ErrorNoMeasurement,
    ErrorIncompleteData,
    ErrorDataOutOfRange,
    UnknownVariant,
}
impl BpStatus {
    pub fn from(content: u8) -> BpStatus {
        match content {
            4 => BpStatus::ErrorIrregularHeartRate,
            0 => BpStatus::NoError,
            2 => BpStatus::ErrorNoMeasurement,
            1 => BpStatus::ErrorIncompleteData,
            3 => BpStatus::ErrorDataOutOfRange,
            _ => BpStatus::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::BpStatus(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum UserLocalId {
    LocalMax,
    LocalMin,
    StationaryMax,
    PortableMin,
    PortableMax,
    StationaryMin,
    UnknownVariant,
}
impl UserLocalId {
    pub fn from(content: u16) -> UserLocalId {
        match content {
            15 => UserLocalId::LocalMax,
            0 => UserLocalId::LocalMin,
            255 => UserLocalId::StationaryMax,
            256 => UserLocalId::PortableMin,
            65534 => UserLocalId::PortableMax,
            16 => UserLocalId::StationaryMin,
            _ => UserLocalId::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::UserLocalId(Self::from(
                reader.next_u16(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SwimStroke {
    Freestyle,
    Mixed,
    Drill,
    Im,
    Butterfly,
    Backstroke,
    ImByRound,
    Breaststroke,
    Rimo,
    UnknownVariant,
}
impl SwimStroke {
    pub fn from(content: u8) -> SwimStroke {
        match content {
            0 => SwimStroke::Freestyle,
            5 => SwimStroke::Mixed,
            4 => SwimStroke::Drill,
            6 => SwimStroke::Im,
            3 => SwimStroke::Butterfly,
            1 => SwimStroke::Backstroke,
            7 => SwimStroke::ImByRound,
            2 => SwimStroke::Breaststroke,
            8 => SwimStroke::Rimo,
            _ => SwimStroke::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SwimStroke(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ActivityType {
    Cycling,
    Sedentary,
    Transition,
    Walking,
    Generic,
    Swimming,
    All,
    FitnessEquipment,
    Running,
    UnknownVariant,
}
impl ActivityType {
    pub fn from(content: u8) -> ActivityType {
        match content {
            2 => ActivityType::Cycling,
            8 => ActivityType::Sedentary,
            3 => ActivityType::Transition,
            6 => ActivityType::Walking,
            0 => ActivityType::Generic,
            5 => ActivityType::Swimming,
            254 => ActivityType::All,
            4 => ActivityType::FitnessEquipment,
            1 => ActivityType::Running,
            _ => ActivityType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ActivityType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ActivitySubtype {
    Elliptical,
    LapSwimming,
    Trail,
    IndoorRowing,
    StairClimbing,
    Track,
    Road,
    HandCycling,
    Generic,
    Downhill,
    TrackCycling,
    Treadmill,
    OpenWater,
    Street,
    Mountain,
    All,
    Spin,
    Cyclocross,
    Recumbent,
    IndoorCycling,
    UnknownVariant,
}
impl ActivitySubtype {
    pub fn from(content: u8) -> ActivitySubtype {
        match content {
            15 => ActivitySubtype::Elliptical,
            17 => ActivitySubtype::LapSwimming,
            3 => ActivitySubtype::Trail,
            14 => ActivitySubtype::IndoorRowing,
            16 => ActivitySubtype::StairClimbing,
            4 => ActivitySubtype::Track,
            7 => ActivitySubtype::Road,
            12 => ActivitySubtype::HandCycling,
            0 => ActivitySubtype::Generic,
            9 => ActivitySubtype::Downhill,
            13 => ActivitySubtype::TrackCycling,
            1 => ActivitySubtype::Treadmill,
            18 => ActivitySubtype::OpenWater,
            2 => ActivitySubtype::Street,
            8 => ActivitySubtype::Mountain,
            254 => ActivitySubtype::All,
            5 => ActivitySubtype::Spin,
            11 => ActivitySubtype::Cyclocross,
            10 => ActivitySubtype::Recumbent,
            6 => ActivitySubtype::IndoorCycling,
            _ => ActivitySubtype::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ActivitySubtype(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ActivityLevel {
    Medium,
    High,
    Low,
    UnknownVariant,
}
impl ActivityLevel {
    pub fn from(content: u8) -> ActivityLevel {
        match content {
            1 => ActivityLevel::Medium,
            2 => ActivityLevel::High,
            0 => ActivityLevel::Low,
            _ => ActivityLevel::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ActivityLevel(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Side {
    Right,
    Left,
    UnknownVariant,
}
impl Side {
    pub fn from(content: u8) -> Side {
        match content {
            0 => Side::Right,
            1 => Side::Left,
            _ => Side::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Side(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum LeftRightBalance {
    Right,
    Mask,
    UnknownVariant,
}
impl LeftRightBalance {
    pub fn from(content: u8) -> LeftRightBalance {
        match content {
            128 => LeftRightBalance::Right,
            127 => LeftRightBalance::Mask,
            _ => LeftRightBalance::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::LeftRightBalance(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum LeftRightBalance100 {
    Right,
    Mask,
    UnknownVariant,
}
impl LeftRightBalance100 {
    pub fn from(content: u16) -> LeftRightBalance100 {
        match content {
            32768 => LeftRightBalance100::Right,
            16383 => LeftRightBalance100::Mask,
            _ => LeftRightBalance100::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::LeftRightBalance100(Self::from(
                reader.next_u16(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum LengthType {
    Idle,
    Active,
    UnknownVariant,
}
impl LengthType {
    pub fn from(content: u8) -> LengthType {
        match content {
            0 => LengthType::Idle,
            1 => LengthType::Active,
            _ => LengthType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::LengthType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DayOfWeek {
    Monday,
    Thursday,
    Sunday,
    Friday,
    Tuesday,
    Wednesday,
    Saturday,
    UnknownVariant,
}
impl DayOfWeek {
    pub fn from(content: u8) -> DayOfWeek {
        match content {
            1 => DayOfWeek::Monday,
            4 => DayOfWeek::Thursday,
            0 => DayOfWeek::Sunday,
            5 => DayOfWeek::Friday,
            2 => DayOfWeek::Tuesday,
            3 => DayOfWeek::Wednesday,
            6 => DayOfWeek::Saturday,
            _ => DayOfWeek::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DayOfWeek(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ConnectivityCapabilities {
    ActivityUpload,
    GolfCourseDownload,
    WifiVerification,
    InstantInput,
    ConnectIqWidgetDownload,
    CourseDownload,
    ContinueSyncAfterSoftwareUpdate,
    LiveTrack,
    WorkoutDownload,
    ConnectIqAppDownload,
    BluetoothLe,
    ConnectIqDataFieldDownload,
    FindMyWatch,
    RemoteManualSync,
    WeatherAlerts,
    Ant,
    IncidentDetection,
    AudioPrompts,
    ExplicitArchive,
    TrueUp,
    SetupIncomplete,
    ConnectIqWatchAppDownload,
    SwingSensorRemote,
    LiveTrackMessaging,
    WeatherConditions,
    GpsEphemerisDownload,
    ConnectIqWatchFaceDownload,
    DeviceInitiatesSync,
    Bluetooth,
    LiveTrackAutoStart,
    ConnectIqAppManagment,
    SwingSensor,
    UnknownVariant,
}
impl ConnectivityCapabilities {
    pub fn from(content: u32) -> ConnectivityCapabilities {
        match content {
            8 => ConnectivityCapabilities::ActivityUpload,
            16384 => ConnectivityCapabilities::GolfCourseDownload,
            33554432 => ConnectivityCapabilities::WifiVerification,
            2147483648 => ConnectivityCapabilities::InstantInput,
            131072 => ConnectivityCapabilities::ConnectIqWidgetDownload,
            16 => ConnectivityCapabilities::CourseDownload,
            4096 => ConnectivityCapabilities::ContinueSyncAfterSoftwareUpdate,
            64 => ConnectivityCapabilities::LiveTrack,
            32 => ConnectivityCapabilities::WorkoutDownload,
            8192 => ConnectivityCapabilities::ConnectIqAppDownload,
            2 => ConnectivityCapabilities::BluetoothLe,
            524288 => ConnectivityCapabilities::ConnectIqDataFieldDownload,
            134217728 => ConnectivityCapabilities::FindMyWatch,
            268435456 => ConnectivityCapabilities::RemoteManualSync,
            256 => ConnectivityCapabilities::WeatherAlerts,
            4 => ConnectivityCapabilities::Ant,
            8388608 => ConnectivityCapabilities::IncidentDetection,
            16777216 => ConnectivityCapabilities::AudioPrompts,
            1024 => ConnectivityCapabilities::ExplicitArchive,
            67108864 => ConnectivityCapabilities::TrueUp,
            2048 => ConnectivityCapabilities::SetupIncomplete,
            65536 => ConnectivityCapabilities::ConnectIqWatchAppDownload,
            4194304 => ConnectivityCapabilities::SwingSensorRemote,
            1073741824 => ConnectivityCapabilities::LiveTrackMessaging,
            128 => ConnectivityCapabilities::WeatherConditions,
            512 => ConnectivityCapabilities::GpsEphemerisDownload,
            262144 => ConnectivityCapabilities::ConnectIqWatchFaceDownload,
            32768 => ConnectivityCapabilities::DeviceInitiatesSync,
            1 => ConnectivityCapabilities::Bluetooth,
            536870912 => ConnectivityCapabilities::LiveTrackAutoStart,
            1048576 => ConnectivityCapabilities::ConnectIqAppManagment,
            2097152 => ConnectivityCapabilities::SwingSensor,
            _ => ConnectivityCapabilities::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ConnectivityCapabilities(
                Self::from(reader.next_u32(endianness)?),
            )));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WeatherReport {
    HourlyForecast,
    Current,
    DailyForecast,
    UnknownVariant,
}
impl WeatherReport {
    pub fn from(content: u8) -> WeatherReport {
        match content {
            1 => WeatherReport::HourlyForecast,
            0 => WeatherReport::Current,
            2 => WeatherReport::DailyForecast,
            _ => WeatherReport::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WeatherReport(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WeatherStatus {
    Fog,
    HeavySnow,
    HeavyRain,
    Clear,
    Hail,
    HeavyRainSnow,
    LightRainSnow,
    Windy,
    Cloudy,
    ScatteredThunderstorms,
    LightSnow,
    WintryMix,
    MostlyCloudy,
    UnknownPrecipitation,
    PartlyCloudy,
    ScatteredShowers,
    Rain,
    Thunderstorms,
    Snow,
    Hazy,
    LightRain,
    UnknownVariant,
}
impl WeatherStatus {
    pub fn from(content: u8) -> WeatherStatus {
        match content {
            8 => WeatherStatus::Fog,
            19 => WeatherStatus::HeavySnow,
            17 => WeatherStatus::HeavyRain,
            0 => WeatherStatus::Clear,
            12 => WeatherStatus::Hail,
            21 => WeatherStatus::HeavyRainSnow,
            20 => WeatherStatus::LightRainSnow,
            5 => WeatherStatus::Windy,
            22 => WeatherStatus::Cloudy,
            14 => WeatherStatus::ScatteredThunderstorms,
            18 => WeatherStatus::LightSnow,
            7 => WeatherStatus::WintryMix,
            2 => WeatherStatus::MostlyCloudy,
            15 => WeatherStatus::UnknownPrecipitation,
            1 => WeatherStatus::PartlyCloudy,
            13 => WeatherStatus::ScatteredShowers,
            3 => WeatherStatus::Rain,
            6 => WeatherStatus::Thunderstorms,
            4 => WeatherStatus::Snow,
            11 => WeatherStatus::Hazy,
            16 => WeatherStatus::LightRain,
            _ => WeatherStatus::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WeatherStatus(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WeatherSeverity {
    Statement,
    Watch,
    Advisory,
    Warning,
    Unknown,
    UnknownVariant,
}
impl WeatherSeverity {
    pub fn from(content: u8) -> WeatherSeverity {
        match content {
            4 => WeatherSeverity::Statement,
            2 => WeatherSeverity::Watch,
            3 => WeatherSeverity::Advisory,
            1 => WeatherSeverity::Warning,
            0 => WeatherSeverity::Unknown,
            _ => WeatherSeverity::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WeatherSeverity(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WeatherSevereType {
    Hydrological,
    AirQuality,
    SevereThunderstorm,
    InlandTropicalStorm,
    FreezingRain,
    RipTide,
    Tornado,
    HighWind,
    WinterWeather,
    HighWaterLevel,
    HazardousSeas,
    ExtremeWind,
    Snowfall,
    FlashFreeze,
    HeavySnowAlert,
    SnowSquall,
    Unspecified,
    Storm,
    CoastalFlood,
    HighSurf,
    SnowAndBlowingSnow,
    SnowAlert,
    DebrisFlow,
    LakeWind,
    ArcticOutflow,
    Typhoon,
    Hurricane,
    IceStorm,
    Gale,
    LakeEffectBlowingSnow,
    Freeze,
    HardFreeze,
    SpecialMarine,
    Tsunami,
    Humidex,
    StormSurge,
    MarineWeather,
    WreckhouseWinds,
    Frost,
    Smog,
    LowWater,
    SpecialWeather,
    Sleet,
    Weather,
    StrongWind,
    HeavyFreezingSpray,
    SmallCraftHazardousSeas,
    BlowingDust,
    AirStagnation,
    DenseSmoke,
    ExcessiveHeat,
    ArealFlood,
    Rainfall,
    ColdWave,
    DenseFog,
    LesSuetesWind,
    FreezingFog,
    Squall,
    HurricaneForceWind,
    FireWeather,
    LakeshoreFlood,
    Wind,
    Blizzard,
    WindChill,
    BlowingSnow,
    SmallCraft,
    DustStorm,
    SmallCraftWinds,
    WinterStorm,
    LakeEffectSnow,
    FreezingSpray,
    ExtremeCold,
    InlandHurricane,
    Avalanche,
    Heat,
    FreezingDrizzle,
    HighHeatAndHumidity,
    SmallCraftRoughBar,
    HumidexAndHealth,
    Ashfall,
    Flood,
    BriskWind,
    TropicalStorm,
    Waterspout,
    FlashFlood,
    UnknownVariant,
}
impl WeatherSevereType {
    pub fn from(content: u8) -> WeatherSevereType {
        match content {
            83 => WeatherSevereType::Hydrological,
            79 => WeatherSevereType::AirQuality,
            9 => WeatherSevereType::SevereThunderstorm,
            15 => WeatherSevereType::InlandTropicalStorm,
            18 => WeatherSevereType::FreezingRain,
            76 => WeatherSevereType::RipTide,
            1 => WeatherSevereType::Tornado,
            22 => WeatherSevereType::HighWind,
            32 => WeatherSevereType::WinterWeather,
            65 => WeatherSevereType::HighWaterLevel,
            61 => WeatherSevereType::HazardousSeas,
            4 => WeatherSevereType::ExtremeWind,
            34 => WeatherSevereType::Snowfall,
            20 => WeatherSevereType::FlashFreeze,
            28 => WeatherSevereType::HeavySnowAlert,
            30 => WeatherSevereType::SnowSquall,
            0 => WeatherSevereType::Unspecified,
            40 => WeatherSevereType::Storm,
            44 => WeatherSevereType::CoastalFlood,
            77 => WeatherSevereType::HighSurf,
            35 => WeatherSevereType::SnowAndBlowingSnow,
            37 => WeatherSevereType::SnowAlert,
            19 => WeatherSevereType::DebrisFlow,
            57 => WeatherSevereType::LakeWind,
            38 => WeatherSevereType::ArcticOutflow,
            5 => WeatherSevereType::Typhoon,
            3 => WeatherSevereType::Hurricane,
            17 => WeatherSevereType::IceStorm,
            52 => WeatherSevereType::Gale,
            29 => WeatherSevereType::LakeEffectBlowingSnow,
            72 => WeatherSevereType::Freeze,
            71 => WeatherSevereType::HardFreeze,
            54 => WeatherSevereType::SpecialMarine,
            2 => WeatherSevereType::Tsunami,
            51 => WeatherSevereType::Humidex,
            41 => WeatherSevereType::StormSurge,
            58 => WeatherSevereType::MarineWeather,
            10 => WeatherSevereType::WreckhouseWinds,
            73 => WeatherSevereType::Frost,
            78 => WeatherSevereType::Smog,
            82 => WeatherSevereType::LowWater,
            84 => WeatherSevereType::SpecialWeather,
            33 => WeatherSevereType::Sleet,
            48 => WeatherSevereType::Weather,
            56 => WeatherSevereType::StrongWind,
            24 => WeatherSevereType::HeavyFreezingSpray,
            60 => WeatherSevereType::SmallCraftHazardousSeas,
            70 => WeatherSevereType::BlowingDust,
            81 => WeatherSevereType::AirStagnation,
            69 => WeatherSevereType::DenseSmoke,
            46 => WeatherSevereType::ExcessiveHeat,
            43 => WeatherSevereType::ArealFlood,
            42 => WeatherSevereType::Rainfall,
            27 => WeatherSevereType::ColdWave,
            68 => WeatherSevereType::DenseFog,
            11 => WeatherSevereType::LesSuetesWind,
            67 => WeatherSevereType::FreezingFog,
            55 => WeatherSevereType::Squall,
            7 => WeatherSevereType::HurricaneForceWind,
            74 => WeatherSevereType::FireWeather,
            45 => WeatherSevereType::LakeshoreFlood,
            59 => WeatherSevereType::Wind,
            16 => WeatherSevereType::Blizzard,
            26 => WeatherSevereType::WindChill,
            36 => WeatherSevereType::BlowingSnow,
            62 => WeatherSevereType::SmallCraft,
            21 => WeatherSevereType::DustStorm,
            63 => WeatherSevereType::SmallCraftWinds,
            23 => WeatherSevereType::WinterStorm,
            31 => WeatherSevereType::LakeEffectSnow,
            53 => WeatherSevereType::FreezingSpray,
            25 => WeatherSevereType::ExtremeCold,
            6 => WeatherSevereType::InlandHurricane,
            12 => WeatherSevereType::Avalanche,
            47 => WeatherSevereType::Heat,
            39 => WeatherSevereType::FreezingDrizzle,
            49 => WeatherSevereType::HighHeatAndHumidity,
            64 => WeatherSevereType::SmallCraftRoughBar,
            50 => WeatherSevereType::HumidexAndHealth,
            66 => WeatherSevereType::Ashfall,
            75 => WeatherSevereType::Flood,
            80 => WeatherSevereType::BriskWind,
            14 => WeatherSevereType::TropicalStorm,
            8 => WeatherSevereType::Waterspout,
            13 => WeatherSevereType::FlashFlood,
            _ => WeatherSevereType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WeatherSevereType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum LocaltimeIntoDay {
    UnknownVariant,
}
impl LocaltimeIntoDay {
    pub fn from(content: u32) -> LocaltimeIntoDay {
        match content {
            _ => LocaltimeIntoDay::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::LocaltimeIntoDay(Self::from(
                reader.next_u32(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum StrokeType {
    NoEvent,
    Smash,
    Serve,
    Backhand,
    Other,
    Forehand,
    UnknownVariant,
}
impl StrokeType {
    pub fn from(content: u8) -> StrokeType {
        match content {
            0 => StrokeType::NoEvent,
            5 => StrokeType::Smash,
            2 => StrokeType::Serve,
            4 => StrokeType::Backhand,
            1 => StrokeType::Other,
            3 => StrokeType::Forehand,
            _ => StrokeType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::StrokeType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum BodyLocation {
    RightChest,
    LeftForearmExtensors,
    LeftHamstring,
    WaistMidBack,
    WaistLeft,
    LeftLeg,
    WaistRight,
    RightQuad,
    LeftBicep,
    RightArm,
    RightTricep,
    Throat,
    LeftCalf,
    RightGlute,
    LeftTricep,
    RightHamstring,
    RightUpperBack,
    RightBrachioradialis,
    RightShin,
    RightLowerBack,
    WaistFront,
    LeftGlute,
    RightForearmExtensors,
    TorsoBack,
    RightAbdomen,
    Neck,
    LeftChest,
    LeftQuad,
    LeftLowerBack,
    TorsoFront,
    LeftUpperBack,
    RightCalf,
    LeftShin,
    LeftShoulder,
    LeftArm,
    LeftAbdomen,
    RightShoulder,
    RightLeg,
    RightBicep,
    LeftBrachioradialis,
    UnknownVariant,
}
impl BodyLocation {
    pub fn from(content: u8) -> BodyLocation {
        match content {
            21 => BodyLocation::RightChest,
            27 => BodyLocation::LeftForearmExtensors,
            3 => BodyLocation::LeftHamstring,
            36 => BodyLocation::WaistMidBack,
            38 => BodyLocation::WaistLeft,
            0 => BodyLocation::LeftLeg,
            39 => BodyLocation::WaistRight,
            10 => BodyLocation::RightQuad,
            24 => BodyLocation::LeftBicep,
            28 => BodyLocation::RightArm,
            31 => BodyLocation::RightTricep,
            35 => BodyLocation::Throat,
            1 => BodyLocation::LeftCalf,
            11 => BodyLocation::RightGlute,
            25 => BodyLocation::LeftTricep,
            9 => BodyLocation::RightHamstring,
            16 => BodyLocation::RightUpperBack,
            32 => BodyLocation::RightBrachioradialis,
            8 => BodyLocation::RightShin,
            15 => BodyLocation::RightLowerBack,
            37 => BodyLocation::WaistFront,
            5 => BodyLocation::LeftGlute,
            33 => BodyLocation::RightForearmExtensors,
            12 => BodyLocation::TorsoBack,
            20 => BodyLocation::RightAbdomen,
            34 => BodyLocation::Neck,
            19 => BodyLocation::LeftChest,
            4 => BodyLocation::LeftQuad,
            13 => BodyLocation::LeftLowerBack,
            17 => BodyLocation::TorsoFront,
            14 => BodyLocation::LeftUpperBack,
            7 => BodyLocation::RightCalf,
            2 => BodyLocation::LeftShin,
            23 => BodyLocation::LeftShoulder,
            22 => BodyLocation::LeftArm,
            18 => BodyLocation::LeftAbdomen,
            29 => BodyLocation::RightShoulder,
            6 => BodyLocation::RightLeg,
            30 => BodyLocation::RightBicep,
            26 => BodyLocation::LeftBrachioradialis,
            _ => BodyLocation::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::BodyLocation(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SegmentLapStatus {
    End,
    Fail,
    UnknownVariant,
}
impl SegmentLapStatus {
    pub fn from(content: u8) -> SegmentLapStatus {
        match content {
            0 => SegmentLapStatus::End,
            1 => SegmentLapStatus::Fail,
            _ => SegmentLapStatus::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SegmentLapStatus(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SegmentLeaderboardType {
    Goal,
    RecentBest,
    Connections,
    Pr,
    PersonalBest,
    Last,
    Overall,
    Kom,
    Qom,
    Challenger,
    Carrot,
    Rival,
    ClubLeader,
    CourseRecord,
    Group,
    UnknownVariant,
}
impl SegmentLeaderboardType {
    pub fn from(content: u8) -> SegmentLeaderboardType {
        match content {
            8 => SegmentLeaderboardType::Goal,
            13 => SegmentLeaderboardType::RecentBest,
            2 => SegmentLeaderboardType::Connections,
            7 => SegmentLeaderboardType::Pr,
            1 => SegmentLeaderboardType::PersonalBest,
            12 => SegmentLeaderboardType::Last,
            0 => SegmentLeaderboardType::Overall,
            5 => SegmentLeaderboardType::Kom,
            6 => SegmentLeaderboardType::Qom,
            4 => SegmentLeaderboardType::Challenger,
            9 => SegmentLeaderboardType::Carrot,
            11 => SegmentLeaderboardType::Rival,
            10 => SegmentLeaderboardType::ClubLeader,
            14 => SegmentLeaderboardType::CourseRecord,
            3 => SegmentLeaderboardType::Group,
            _ => SegmentLeaderboardType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SegmentLeaderboardType(
                Self::from(reader.next_u8()?),
            )));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SegmentDeleteStatus {
    DoNotDelete,
    DeleteOne,
    DeleteAll,
    UnknownVariant,
}
impl SegmentDeleteStatus {
    pub fn from(content: u8) -> SegmentDeleteStatus {
        match content {
            0 => SegmentDeleteStatus::DoNotDelete,
            1 => SegmentDeleteStatus::DeleteOne,
            2 => SegmentDeleteStatus::DeleteAll,
            _ => SegmentDeleteStatus::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SegmentDeleteStatus(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SegmentSelectionType {
    Suggested,
    Starred,
    UnknownVariant,
}
impl SegmentSelectionType {
    pub fn from(content: u8) -> SegmentSelectionType {
        match content {
            1 => SegmentSelectionType::Suggested,
            0 => SegmentSelectionType::Starred,
            _ => SegmentSelectionType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SegmentSelectionType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SourceType {
    Ant,
    Bluetooth,
    Local,
    Antplus,
    BluetoothLowEnergy,
    Wifi,
    UnknownVariant,
}
impl SourceType {
    pub fn from(content: u8) -> SourceType {
        match content {
            0 => SourceType::Ant,
            2 => SourceType::Bluetooth,
            5 => SourceType::Local,
            1 => SourceType::Antplus,
            3 => SourceType::BluetoothLowEnergy,
            4 => SourceType::Wifi,
            _ => SourceType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SourceType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum AntChannelId {
    AntDeviceType,
    AntTransmissionTypeLowerNibble,
    AntDeviceNumber,
    AntExtendedDeviceNumberUpperNibble,
    UnknownVariant,
}
impl AntChannelId {
    pub fn from(content: u32) -> AntChannelId {
        match content {
            16711680 => AntChannelId::AntDeviceType,
            251658240 => AntChannelId::AntTransmissionTypeLowerNibble,
            65535 => AntChannelId::AntDeviceNumber,
            4026531840 => AntChannelId::AntExtendedDeviceNumberUpperNibble,
            _ => AntChannelId::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::AntChannelId(Self::from(
                reader.next_u32(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DisplayOrientation {
    Auto,
    Landscape,
    LandscapeFlipped,
    PortraitFlipped,
    Portrait,
    UnknownVariant,
}
impl DisplayOrientation {
    pub fn from(content: u8) -> DisplayOrientation {
        match content {
            0 => DisplayOrientation::Auto,
            2 => DisplayOrientation::Landscape,
            4 => DisplayOrientation::LandscapeFlipped,
            3 => DisplayOrientation::PortraitFlipped,
            1 => DisplayOrientation::Portrait,
            _ => DisplayOrientation::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DisplayOrientation(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WorkoutEquipment {
    SwimFins,
    SwimPullBuoy,
    None,
    SwimSnorkel,
    SwimKickboard,
    SwimPaddles,
    UnknownVariant,
}
impl WorkoutEquipment {
    pub fn from(content: u8) -> WorkoutEquipment {
        match content {
            1 => WorkoutEquipment::SwimFins,
            4 => WorkoutEquipment::SwimPullBuoy,
            0 => WorkoutEquipment::None,
            5 => WorkoutEquipment::SwimSnorkel,
            2 => WorkoutEquipment::SwimKickboard,
            3 => WorkoutEquipment::SwimPaddles,
            _ => WorkoutEquipment::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WorkoutEquipment(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WatchfaceMode {
    Analog,
    Digital,
    Disabled,
    ConnectIq,
    UnknownVariant,
}
impl WatchfaceMode {
    pub fn from(content: u8) -> WatchfaceMode {
        match content {
            1 => WatchfaceMode::Analog,
            0 => WatchfaceMode::Digital,
            3 => WatchfaceMode::Disabled,
            2 => WatchfaceMode::ConnectIq,
            _ => WatchfaceMode::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WatchfaceMode(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum CameraEventType {
    VideoStart,
    PhotoTaken,
    VideoSecondStreamEnd,
    VideoPause,
    VideoSecondStreamStart,
    VideoSecondStreamSplit,
    VideoSecondStreamPause,
    VideoEnd,
    VideoSplit,
    VideoSplitStart,
    VideoSecondStreamSplitStart,
    VideoResume,
    VideoSecondStreamResume,
    UnknownVariant,
}
impl CameraEventType {
    pub fn from(content: u8) -> CameraEventType {
        match content {
            0 => CameraEventType::VideoStart,
            3 => CameraEventType::PhotoTaken,
            6 => CameraEventType::VideoSecondStreamEnd,
            11 => CameraEventType::VideoPause,
            4 => CameraEventType::VideoSecondStreamStart,
            5 => CameraEventType::VideoSecondStreamSplit,
            12 => CameraEventType::VideoSecondStreamPause,
            2 => CameraEventType::VideoEnd,
            1 => CameraEventType::VideoSplit,
            7 => CameraEventType::VideoSplitStart,
            8 => CameraEventType::VideoSecondStreamSplitStart,
            13 => CameraEventType::VideoResume,
            14 => CameraEventType::VideoSecondStreamResume,
            _ => CameraEventType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::CameraEventType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SensorType {
    Accelerometer,
    Gyroscope,
    Barometer,
    Compass,
    UnknownVariant,
}
impl SensorType {
    pub fn from(content: u8) -> SensorType {
        match content {
            0 => SensorType::Accelerometer,
            1 => SensorType::Gyroscope,
            3 => SensorType::Barometer,
            2 => SensorType::Compass,
            _ => SensorType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SensorType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum CameraOrientationType {
    CameraOrientation0,
    CameraOrientation180,
    CameraOrientation90,
    CameraOrientation270,
    UnknownVariant,
}
impl CameraOrientationType {
    pub fn from(content: u8) -> CameraOrientationType {
        match content {
            0 => CameraOrientationType::CameraOrientation0,
            2 => CameraOrientationType::CameraOrientation180,
            1 => CameraOrientationType::CameraOrientation90,
            3 => CameraOrientationType::CameraOrientation270,
            _ => CameraOrientationType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::CameraOrientationType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum AttitudeStage {
    Valid,
    Failed,
    Degraded,
    Aligning,
    UnknownVariant,
}
impl AttitudeStage {
    pub fn from(content: u8) -> AttitudeStage {
        match content {
            3 => AttitudeStage::Valid,
            0 => AttitudeStage::Failed,
            2 => AttitudeStage::Degraded,
            1 => AttitudeStage::Aligning,
            _ => AttitudeStage::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::AttitudeStage(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum AttitudeValidity {
    PitchValid,
    NormalBodyAccelValid,
    TurnRateValid,
    TrueTrackAngle,
    LateralBodyAccelValid,
    HwFail,
    TrackAngleHeadingValid,
    MagInvalid,
    NoGps,
    GpsInvalid,
    SolutionCoasting,
    MagneticHeading,
    RollValid,
    UnknownVariant,
}
impl AttitudeValidity {
    pub fn from(content: u16) -> AttitudeValidity {
        match content {
            2 => AttitudeValidity::PitchValid,
            16 => AttitudeValidity::NormalBodyAccelValid,
            32 => AttitudeValidity::TurnRateValid,
            2048 => AttitudeValidity::TrueTrackAngle,
            8 => AttitudeValidity::LateralBodyAccelValid,
            64 => AttitudeValidity::HwFail,
            1 => AttitudeValidity::TrackAngleHeadingValid,
            128 => AttitudeValidity::MagInvalid,
            256 => AttitudeValidity::NoGps,
            512 => AttitudeValidity::GpsInvalid,
            1024 => AttitudeValidity::SolutionCoasting,
            4096 => AttitudeValidity::MagneticHeading,
            4 => AttitudeValidity::RollValid,
            _ => AttitudeValidity::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::AttitudeValidity(Self::from(
                reader.next_u16(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum AutoSyncFrequency {
    Occasionally,
    Never,
    Frequent,
    Remote,
    OnceADay,
    UnknownVariant,
}
impl AutoSyncFrequency {
    pub fn from(content: u8) -> AutoSyncFrequency {
        match content {
            1 => AutoSyncFrequency::Occasionally,
            0 => AutoSyncFrequency::Never,
            2 => AutoSyncFrequency::Frequent,
            4 => AutoSyncFrequency::Remote,
            3 => AutoSyncFrequency::OnceADay,
            _ => AutoSyncFrequency::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::AutoSyncFrequency(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ExdLayout {
    HalfHorizontalTopSplit,
    HalfHorizontal,
    HalfVertical,
    FullScreen,
    HalfVerticalRightSplit,
    FullQuarterSplit,
    Dynamic,
    HalfHorizontalBottomSplit,
    HalfVerticalLeftSplit,
    UnknownVariant,
}
impl ExdLayout {
    pub fn from(content: u8) -> ExdLayout {
        match content {
            7 => ExdLayout::HalfHorizontalTopSplit,
            2 => ExdLayout::HalfHorizontal,
            1 => ExdLayout::HalfVertical,
            0 => ExdLayout::FullScreen,
            3 => ExdLayout::HalfVerticalRightSplit,
            5 => ExdLayout::FullQuarterSplit,
            8 => ExdLayout::Dynamic,
            4 => ExdLayout::HalfHorizontalBottomSplit,
            6 => ExdLayout::HalfVerticalLeftSplit,
            _ => ExdLayout::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ExdLayout(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ExdDisplayType {
    CircleGraph,
    Bar,
    Gauge,
    Simple,
    Graph,
    Numerical,
    Balance,
    SimpleDynamicIcon,
    StringList,
    VirtualPartner,
    String,
    UnknownVariant,
}
impl ExdDisplayType {
    pub fn from(content: u8) -> ExdDisplayType {
        match content {
            4 => ExdDisplayType::CircleGraph,
            3 => ExdDisplayType::Bar,
            10 => ExdDisplayType::Gauge,
            1 => ExdDisplayType::Simple,
            2 => ExdDisplayType::Graph,
            0 => ExdDisplayType::Numerical,
            6 => ExdDisplayType::Balance,
            9 => ExdDisplayType::SimpleDynamicIcon,
            7 => ExdDisplayType::StringList,
            5 => ExdDisplayType::VirtualPartner,
            8 => ExdDisplayType::String,
            _ => ExdDisplayType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ExdDisplayType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ExdDataUnits {
    Kilometers,
    Percent,
    EnumBikeLightBeamAngleMode,
    Kilojoules,
    Yards,
    SecondPerMile,
    EnumCoursePoint,
    MetersPerHour,
    Rpm,
    Gear,
    Laps,
    Watts,
    Degrees,
    Seconds,
    Centimeter,
    EnumSport,
    MmHg,
    HectoPascals,
    MetersPerMin,
    Minutes,
    Milliseconds,
    MilesPerHour,
    DegreesCelsius,
    WattsPerKilogram,
    Zone,
    EnumBatteryStatus,
    EnumBikeLightNetworkConfigType,
    FeetPerHour,
    EnumBikeLightBatteryStatus,
    Mbars,
    Millimeters,
    Calories,
    FeetPerMin,
    NoUnits,
    Kilofeet,
    EnumTurnType,
    KilometersPerHour,
    Miles,
    SecondPerKilometer,
    EightCardinal,
    InchesHg,
    Bpm,
    DegreesFarenheit,
    Bradians,
    Feet,
    Time,
    MetersPerSec,
    Hours,
    Lights,
    Meters,
    UnknownVariant,
}
impl ExdDataUnits {
    pub fn from(content: u8) -> ExdDataUnits {
        match content {
            15 => ExdDataUnits::Kilometers,
            22 => ExdDataUnits::Percent,
            26 => ExdDataUnits::EnumBikeLightBeamAngleMode,
            34 => ExdDataUnits::Kilojoules,
            17 => ExdDataUnits::Yards,
            36 => ExdDataUnits::SecondPerMile,
            39 => ExdDataUnits::EnumCoursePoint,
            5 => ExdDataUnits::MetersPerHour,
            10 => ExdDataUnits::Rpm,
            9 => ExdDataUnits::Gear,
            1 => ExdDataUnits::Laps,
            23 => ExdDataUnits::Watts,
            12 => ExdDataUnits::Degrees,
            30 => ExdDataUnits::Seconds,
            38 => ExdDataUnits::Centimeter,
            41 => ExdDataUnits::EnumSport,
            43 => ExdDataUnits::MmHg,
            45 => ExdDataUnits::HectoPascals,
            47 => ExdDataUnits::MetersPerMin,
            31 => ExdDataUnits::Minutes,
            35 => ExdDataUnits::Milliseconds,
            2 => ExdDataUnits::MilesPerHour,
            6 => ExdDataUnits::DegreesCelsius,
            24 => ExdDataUnits::WattsPerKilogram,
            8 => ExdDataUnits::Zone,
            25 => ExdDataUnits::EnumBatteryStatus,
            28 => ExdDataUnits::EnumBikeLightNetworkConfigType,
            4 => ExdDataUnits::FeetPerHour,
            27 => ExdDataUnits::EnumBikeLightBatteryStatus,
            44 => ExdDataUnits::Mbars,
            13 => ExdDataUnits::Millimeters,
            33 => ExdDataUnits::Calories,
            46 => ExdDataUnits::FeetPerMin,
            0 => ExdDataUnits::NoUnits,
            18 => ExdDataUnits::Kilofeet,
            21 => ExdDataUnits::EnumTurnType,
            3 => ExdDataUnits::KilometersPerHour,
            19 => ExdDataUnits::Miles,
            37 => ExdDataUnits::SecondPerKilometer,
            49 => ExdDataUnits::EightCardinal,
            42 => ExdDataUnits::InchesHg,
            11 => ExdDataUnits::Bpm,
            7 => ExdDataUnits::DegreesFarenheit,
            40 => ExdDataUnits::Bradians,
            16 => ExdDataUnits::Feet,
            20 => ExdDataUnits::Time,
            48 => ExdDataUnits::MetersPerSec,
            32 => ExdDataUnits::Hours,
            29 => ExdDataUnits::Lights,
            14 => ExdDataUnits::Meters,
            _ => ExdDataUnits::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ExdDataUnits(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ExdQualifiers {
    Zone6,
    PercentMaximumAverage,
    MaximumLap,
    Maximum24h,
    EstimatedTotal,
    Zone2,
    ToDestination,
    Zone3,
    AverageLap,
    Total,
    PercentMaximum,
    Elapsed,
    Shifter,
    Zone9,
    Zone1,
    Third,
    ThreeSecondAverage,
    Minimum,
    NoQualifier,
    Average,
    Zone7,
    MaximumAverage,
    NextCoursePoint,
    Zone5,
    Sunset,
    LastLap,
    ToGo,
    Maximum,
    TenSecondAverage,
    ToNext,
    Lap,
    ThirtySecondAverage,
    First,
    LapPercentMaximum,
    LastSport,
    Stopped,
    Instantaneous,
    Zone4,
    Zone8,
    Minimum24h,
    Second,
    ComparedToVirtualPartner,
    Moving,
    Sunrise,
    UnknownVariant,
}
impl ExdQualifiers {
    pub fn from(content: u8) -> ExdQualifiers {
        match content {
            245 => ExdQualifiers::Zone6,
            18 => ExdQualifiers::PercentMaximumAverage,
            6 => ExdQualifiers::MaximumLap,
            24 => ExdQualifiers::Maximum24h,
            34 => ExdQualifiers::EstimatedTotal,
            249 => ExdQualifiers::Zone2,
            9 => ExdQualifiers::ToDestination,
            248 => ExdQualifiers::Zone3,
            8 => ExdQualifiers::AverageLap,
            13 => ExdQualifiers::Total,
            17 => ExdQualifiers::PercentMaximum,
            20 => ExdQualifiers::Elapsed,
            30 => ExdQualifiers::Shifter,
            242 => ExdQualifiers::Zone9,
            250 => ExdQualifiers::Zone1,
            29 => ExdQualifiers::Third,
            14 => ExdQualifiers::ThreeSecondAverage,
            26 => ExdQualifiers::Minimum,
            0 => ExdQualifiers::NoQualifier,
            2 => ExdQualifiers::Average,
            244 => ExdQualifiers::Zone7,
            5 => ExdQualifiers::MaximumAverage,
            12 => ExdQualifiers::NextCoursePoint,
            246 => ExdQualifiers::Zone5,
            22 => ExdQualifiers::Sunset,
            7 => ExdQualifiers::LastLap,
            10 => ExdQualifiers::ToGo,
            4 => ExdQualifiers::Maximum,
            15 => ExdQualifiers::TenSecondAverage,
            11 => ExdQualifiers::ToNext,
            3 => ExdQualifiers::Lap,
            16 => ExdQualifiers::ThirtySecondAverage,
            27 => ExdQualifiers::First,
            19 => ExdQualifiers::LapPercentMaximum,
            31 => ExdQualifiers::LastSport,
            33 => ExdQualifiers::Stopped,
            1 => ExdQualifiers::Instantaneous,
            247 => ExdQualifiers::Zone4,
            243 => ExdQualifiers::Zone8,
            25 => ExdQualifiers::Minimum24h,
            28 => ExdQualifiers::Second,
            23 => ExdQualifiers::ComparedToVirtualPartner,
            32 => ExdQualifiers::Moving,
            21 => ExdQualifiers::Sunrise,
            _ => ExdQualifiers::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ExdQualifiers(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ExdDescriptors {
    Calories,
    NavigationDistance,
    NavigationEstimatedTimeOfArrival,
    PowerRatio,
    Ascent,
    BikeLightBatteryStatus,
    FrontGear,
    Power,
    Gears,
    NavigationTurn,
    GpsAccuracy,
    AnaerobicTrainingEffect,
    CourseLocation,
    LeftPowerPhaseStartAngle,
    BatteryLevel,
    TimeStanding,
    Temperature,
    LeftPlatformCenterOffset,
    HeartRateZone,
    Vam,
    TimeOfDay,
    LeftGroundContactTimeBalance,
    TimeInPowerZone,
    Time,
    NavigationHeading,
    NormalizedPower,
    GearRatio,
    PowerWeightRatio,
    HeartRate,
    Grade,
    BeamAngleStatus,
    WorkoutStep,
    TorqueEffectiveness,
    FunctionalThresholdPower,
    Laps,
    TimeOnZone,
    Work,
    Balance,
    IntensityFactor,
    TrainingStressScore,
    RightPowerPhaseStartAngle,
    TrainerResistance,
    TimerTime,
    StrideLength,
    NavigationLocation,
    Pace,
    RunningCadence,
    RightPowerPhaseFinishAngle,
    CourseType,
    VerticalOscillation,
    EstimatedTimeOfArrival,
    GearCombo,
    GpsElevation,
    Course,
    Vmg,
    GlideRatio,
    VerticalRatio,
    Icon,
    VerticalDistance,
    RightGroundContactTimeBalance,
    NumberLightsConnected,
    Distance,
    GroundContactTime,
    RearGear,
    AmbientPressure,
    Speed,
    HeartRateReserve,
    TimeInHeartRateZone,
    PedalSmoothness,
    NavigationTime,
    Elevation,
    LeftPowerPhaseFinishAngle,
    RightPlatformCenterOffset,
    OffCourse,
    PerformanceCondition,
    CourseDistance,
    Descent,
    CourseHeading,
    TimeSeated,
    Di2BatteryLevel,
    GpsSignalStrength,
    Reps,
    Compass,
    VerticalSpeed,
    MuscleOxygen,
    Heading,
    CompassHeading,
    BateryLevel,
    CourseTime,
    TrainingEffect,
    CourseEstimatedTimeOfArrival,
    GpsHeading,
    PowerZone,
    Cadence,
    Pressure,
    LightNetworkMode,
    TrainerTargetPower,
    UnknownVariant,
}
impl ExdDescriptors {
    pub fn from(content: u8) -> ExdDescriptors {
        match content {
            28 => ExdDescriptors::Calories,
            48 => ExdDescriptors::NavigationDistance,
            50 => ExdDescriptors::NavigationEstimatedTimeOfArrival,
            39 => ExdDescriptors::PowerRatio,
            17 => ExdDescriptors::Ascent,
            0 => ExdDescriptors::BikeLightBatteryStatus,
            21 => ExdDescriptors::FrontGear,
            35 => ExdDescriptors::Power,
            65 => ExdDescriptors::Gears,
            78 => ExdDescriptors::NavigationTurn,
            29 => ExdDescriptors::GpsAccuracy,
            88 => ExdDescriptors::AnaerobicTrainingEffect,
            79 => ExdDescriptors::CourseLocation,
            61 => ExdDescriptors::LeftPowerPhaseStartAngle,
            10 => ExdDescriptors::BatteryLevel,
            14 => ExdDescriptors::TimeStanding,
            31 => ExdDescriptors::Temperature,
            59 => ExdDescriptors::LeftPlatformCenterOffset,
            25 => ExdDescriptors::HeartRateZone,
            96 => ExdDescriptors::Vam,
            32 => ExdDescriptors::TimeOfDay,
            71 => ExdDescriptors::LeftGroundContactTimeBalance,
            77 => ExdDescriptors::TimeInPowerZone,
            9 => ExdDescriptors::Time,
            54 => ExdDescriptors::NavigationHeading,
            40 => ExdDescriptors::NormalizedPower,
            23 => ExdDescriptors::GearRatio,
            58 => ExdDescriptors::PowerWeightRatio,
            24 => ExdDescriptors::HeartRate,
            16 => ExdDescriptors::Grade,
            1 => ExdDescriptors::BeamAngleStatus,
            46 => ExdDescriptors::WorkoutStep,
            56 => ExdDescriptors::TorqueEffectiveness,
            36 => ExdDescriptors::FunctionalThresholdPower,
            44 => ExdDescriptors::Laps,
            42 => ExdDescriptors::TimeOnZone,
            38 => ExdDescriptors::Work,
            33 => ExdDescriptors::Balance,
            37 => ExdDescriptors::IntensityFactor,
            41 => ExdDescriptors::TrainingStressScore,
            62 => ExdDescriptors::RightPowerPhaseStartAngle,
            11 => ExdDescriptors::TrainerResistance,
            57 => ExdDescriptors::TimerTime,
            73 => ExdDescriptors::StrideLength,
            80 => ExdDescriptors::NavigationLocation,
            66 => ExdDescriptors::Pace,
            74 => ExdDescriptors::RunningCadence,
            64 => ExdDescriptors::RightPowerPhaseFinishAngle,
            76 => ExdDescriptors::CourseType,
            68 => ExdDescriptors::VerticalOscillation,
            7 => ExdDescriptors::EstimatedTimeOfArrival,
            82 => ExdDescriptors::GearCombo,
            87 => ExdDescriptors::GpsElevation,
            89 => ExdDescriptors::Course,
            93 => ExdDescriptors::Vmg,
            91 => ExdDescriptors::GlideRatio,
            69 => ExdDescriptors::VerticalRatio,
            84 => ExdDescriptors::Icon,
            92 => ExdDescriptors::VerticalDistance,
            72 => ExdDescriptors::RightGroundContactTimeBalance,
            4 => ExdDescriptors::NumberLightsConnected,
            6 => ExdDescriptors::Distance,
            70 => ExdDescriptors::GroundContactTime,
            22 => ExdDescriptors::RearGear,
            94 => ExdDescriptors::AmbientPressure,
            43 => ExdDescriptors::Speed,
            27 => ExdDescriptors::HeartRateReserve,
            26 => ExdDescriptors::TimeInHeartRateZone,
            34 => ExdDescriptors::PedalSmoothness,
            52 => ExdDescriptors::NavigationTime,
            15 => ExdDescriptors::Elevation,
            63 => ExdDescriptors::LeftPowerPhaseFinishAngle,
            60 => ExdDescriptors::RightPlatformCenterOffset,
            90 => ExdDescriptors::OffCourse,
            75 => ExdDescriptors::PerformanceCondition,
            47 => ExdDescriptors::CourseDistance,
            18 => ExdDescriptors::Descent,
            53 => ExdDescriptors::CourseHeading,
            13 => ExdDescriptors::TimeSeated,
            20 => ExdDescriptors::Di2BatteryLevel,
            30 => ExdDescriptors::GpsSignalStrength,
            45 => ExdDescriptors::Reps,
            81 => ExdDescriptors::Compass,
            19 => ExdDescriptors::VerticalSpeed,
            83 => ExdDescriptors::MuscleOxygen,
            8 => ExdDescriptors::Heading,
            85 => ExdDescriptors::CompassHeading,
            2 => ExdDescriptors::BateryLevel,
            51 => ExdDescriptors::CourseTime,
            67 => ExdDescriptors::TrainingEffect,
            49 => ExdDescriptors::CourseEstimatedTimeOfArrival,
            86 => ExdDescriptors::GpsHeading,
            55 => ExdDescriptors::PowerZone,
            5 => ExdDescriptors::Cadence,
            95 => ExdDescriptors::Pressure,
            3 => ExdDescriptors::LightNetworkMode,
            12 => ExdDescriptors::TrainerTargetPower,
            _ => ExdDescriptors::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ExdDescriptors(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum AutoActivityDetect {
    Walking,
    Elliptical,
    Swimming,
    Running,
    Sedentary,
    None,
    Cycling,
    UnknownVariant,
}
impl AutoActivityDetect {
    pub fn from(content: u32) -> AutoActivityDetect {
        match content {
            8 => AutoActivityDetect::Walking,
            32 => AutoActivityDetect::Elliptical,
            4 => AutoActivityDetect::Swimming,
            1 => AutoActivityDetect::Running,
            1024 => AutoActivityDetect::Sedentary,
            0 => AutoActivityDetect::None,
            2 => AutoActivityDetect::Cycling,
            _ => AutoActivityDetect::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::AutoActivityDetect(Self::from(
                reader.next_u32(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum FitBaseType {
    Sint16,
    Byte,
    Sint32,
    Uint64z,
    Uint8,
    Sint8,
    Uint8z,
    Uint16z,
    String,
    Uint16,
    Sint64,
    Uint32,
    Float32,
    Uint32z,
    Enum,
    Float64,
    Uint64,
    UnknownVariant,
}
impl FitBaseType {
    pub fn from(content: u8) -> FitBaseType {
        match content {
            131 => FitBaseType::Sint16,
            13 => FitBaseType::Byte,
            133 => FitBaseType::Sint32,
            144 => FitBaseType::Uint64z,
            2 => FitBaseType::Uint8,
            1 => FitBaseType::Sint8,
            10 => FitBaseType::Uint8z,
            139 => FitBaseType::Uint16z,
            7 => FitBaseType::String,
            132 => FitBaseType::Uint16,
            142 => FitBaseType::Sint64,
            134 => FitBaseType::Uint32,
            136 => FitBaseType::Float32,
            140 => FitBaseType::Uint32z,
            0 => FitBaseType::Enum,
            137 => FitBaseType::Float64,
            143 => FitBaseType::Uint64,
            _ => FitBaseType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::FitBaseType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum FitBaseUnit {
    Pound,
    Kilogram,
    Other,
    UnknownVariant,
}
impl FitBaseUnit {
    pub fn from(content: u16) -> FitBaseUnit {
        match content {
            2 => FitBaseUnit::Pound,
            1 => FitBaseUnit::Kilogram,
            0 => FitBaseUnit::Other,
            _ => FitBaseUnit::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::FitBaseUnit(Self::from(
                reader.next_u16(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SetType {
    Active,
    Rest,
    UnknownVariant,
}
impl SetType {
    pub fn from(content: u8) -> SetType {
        match content {
            1 => SetType::Active,
            0 => SetType::Rest,
            _ => SetType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SetType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum MaxMetCategory {
    Generic,
    Cycling,
    UnknownVariant,
}
impl MaxMetCategory {
    pub fn from(content: u8) -> MaxMetCategory {
        match content {
            0 => MaxMetCategory::Generic,
            1 => MaxMetCategory::Cycling,
            _ => MaxMetCategory::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::MaxMetCategory(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ExerciseCategory {
    FloorClimb,
    Suspension,
    TotalBody,
    Ladder,
    Tire,
    Lunge,
    Sled,
    Squat,
    Cardio,
    Plank,
    OlympicLift,
    Move,
    Core,
    Pose,
    IndoorRow,
    HipRaise,
    Sandbag,
    Curl,
    RunIndoor,
    WarmUp,
    Elliptical,
    Row,
    IndoorBike,
    Plyo,
    Shrug,
    HipSwing,
    PullUp,
    SitUp,
    TricepsExtension,
    LegCurl,
    HipStability,
    ShoulderStability,
    ShoulderPress,
    Chop,
    CalfRaise,
    BandedExercises,
    BattleRope,
    SledgeHammer,
    Bike,
    PushUp,
    StairStepper,
    Crunch,
    Deadlift,
    Hyperextension,
    LegRaise,
    LateralRaise,
    Flye,
    Run,
    CardioSensors,
    BikeOutdoor,
    BenchPress,
    Unknown,
    Carry,
    UnknownVariant,
}
impl ExerciseCategory {
    pub fn from(content: u16) -> ExerciseCategory {
        match content {
            40 => ExerciseCategory::FloorClimb,
            49 => ExerciseCategory::Suspension,
            29 => ExerciseCategory::TotalBody,
            43 => ExerciseCategory::Ladder,
            50 => ExerciseCategory::Tire,
            17 => ExerciseCategory::Lunge,
            45 => ExerciseCategory::Sled,
            28 => ExerciseCategory::Squat,
            2 => ExerciseCategory::Cardio,
            19 => ExerciseCategory::Plank,
            18 => ExerciseCategory::OlympicLift,
            35 => ExerciseCategory::Move,
            5 => ExerciseCategory::Core,
            36 => ExerciseCategory::Pose,
            42 => ExerciseCategory::IndoorRow,
            10 => ExerciseCategory::HipRaise,
            44 => ExerciseCategory::Sandbag,
            7 => ExerciseCategory::Curl,
            52 => ExerciseCategory::RunIndoor,
            31 => ExerciseCategory::WarmUp,
            39 => ExerciseCategory::Elliptical,
            23 => ExerciseCategory::Row,
            41 => ExerciseCategory::IndoorBike,
            20 => ExerciseCategory::Plyo,
            26 => ExerciseCategory::Shrug,
            12 => ExerciseCategory::HipSwing,
            21 => ExerciseCategory::PullUp,
            27 => ExerciseCategory::SitUp,
            30 => ExerciseCategory::TricepsExtension,
            15 => ExerciseCategory::LegCurl,
            11 => ExerciseCategory::HipStability,
            25 => ExerciseCategory::ShoulderStability,
            24 => ExerciseCategory::ShoulderPress,
            4 => ExerciseCategory::Chop,
            1 => ExerciseCategory::CalfRaise,
            37 => ExerciseCategory::BandedExercises,
            38 => ExerciseCategory::BattleRope,
            46 => ExerciseCategory::SledgeHammer,
            33 => ExerciseCategory::Bike,
            22 => ExerciseCategory::PushUp,
            47 => ExerciseCategory::StairStepper,
            6 => ExerciseCategory::Crunch,
            8 => ExerciseCategory::Deadlift,
            13 => ExerciseCategory::Hyperextension,
            16 => ExerciseCategory::LegRaise,
            14 => ExerciseCategory::LateralRaise,
            9 => ExerciseCategory::Flye,
            32 => ExerciseCategory::Run,
            34 => ExerciseCategory::CardioSensors,
            53 => ExerciseCategory::BikeOutdoor,
            0 => ExerciseCategory::BenchPress,
            65534 => ExerciseCategory::Unknown,
            3 => ExerciseCategory::Carry,
            _ => ExerciseCategory::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ExerciseCategory(Self::from(
                reader.next_u16(endianness)?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum WaterType {
    Salt,
    En13319,
    Fresh,
    Custom,
    UnknownVariant,
}
impl WaterType {
    pub fn from(content: u8) -> WaterType {
        match content {
            1 => WaterType::Salt,
            2 => WaterType::En13319,
            0 => WaterType::Fresh,
            3 => WaterType::Custom,
            _ => WaterType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::WaterType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum TissueModelType {
    Zhl16c,
    UnknownVariant,
}
impl TissueModelType {
    pub fn from(content: u8) -> TissueModelType {
        match content {
            0 => TissueModelType::Zhl16c,
            _ => TissueModelType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::TissueModelType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DiveGasStatus {
    Enabled,
    Disabled,
    BackupOnly,
    UnknownVariant,
}
impl DiveGasStatus {
    pub fn from(content: u8) -> DiveGasStatus {
        match content {
            1 => DiveGasStatus::Enabled,
            0 => DiveGasStatus::Disabled,
            2 => DiveGasStatus::BackupOnly,
            _ => DiveGasStatus::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DiveGasStatus(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DiveAlarmType {
    Time,
    Depth,
    Speed,
    UnknownVariant,
}
impl DiveAlarmType {
    pub fn from(content: u8) -> DiveAlarmType {
        match content {
            1 => DiveAlarmType::Time,
            0 => DiveAlarmType::Depth,
            2 => DiveAlarmType::Speed,
            _ => DiveAlarmType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DiveAlarmType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DiveBacklightMode {
    AtDepth,
    AlwaysOn,
    UnknownVariant,
}
impl DiveBacklightMode {
    pub fn from(content: u8) -> DiveBacklightMode {
        match content {
            0 => DiveBacklightMode::AtDepth,
            1 => DiveBacklightMode::AlwaysOn,
            _ => DiveBacklightMode::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DiveBacklightMode(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SleepLevel {
    Light,
    Rem,
    Unmeasurable,
    Deep,
    Awake,
    UnknownVariant,
}
impl SleepLevel {
    pub fn from(content: u8) -> SleepLevel {
        match content {
            2 => SleepLevel::Light,
            4 => SleepLevel::Rem,
            0 => SleepLevel::Unmeasurable,
            3 => SleepLevel::Deep,
            1 => SleepLevel::Awake,
            _ => SleepLevel::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SleepLevel(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum Spo2MeasurementType {
    SpotCheck,
    OffWrist,
    ContinuousCheck,
    Periodic,
    UnknownVariant,
}
impl Spo2MeasurementType {
    pub fn from(content: u8) -> Spo2MeasurementType {
        match content {
            1 => Spo2MeasurementType::SpotCheck,
            0 => Spo2MeasurementType::OffWrist,
            2 => Spo2MeasurementType::ContinuousCheck,
            3 => Spo2MeasurementType::Periodic,
            _ => Spo2MeasurementType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::Spo2MeasurementType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum CcrSetpointSwitchMode {
    Automatic,
    Manual,
    UnknownVariant,
}
impl CcrSetpointSwitchMode {
    pub fn from(content: u8) -> CcrSetpointSwitchMode {
        match content {
            1 => CcrSetpointSwitchMode::Automatic,
            0 => CcrSetpointSwitchMode::Manual,
            _ => CcrSetpointSwitchMode::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::CcrSetpointSwitchMode(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum DiveGasMode {
    OpenCircuit,
    ClosedCircuitDiluent,
    UnknownVariant,
}
impl DiveGasMode {
    pub fn from(content: u8) -> DiveGasMode {
        match content {
            0 => DiveGasMode::OpenCircuit,
            1 => DiveGasMode::ClosedCircuitDiluent,
            _ => DiveGasMode::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::DiveGasMode(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ProjectileType {
    Shotshell,
    PistolCartridge,
    AirRiflePellet,
    Arrow,
    RifleCartridge,
    Other,
    UnknownVariant,
}
impl ProjectileType {
    pub fn from(content: u8) -> ProjectileType {
        match content {
            3 => ProjectileType::Shotshell,
            2 => ProjectileType::PistolCartridge,
            4 => ProjectileType::AirRiflePellet,
            0 => ProjectileType::Arrow,
            1 => ProjectileType::RifleCartridge,
            5 => ProjectileType::Other,
            _ => ProjectileType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ProjectileType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum SplitType {
    ClimbRest,
    IntervalCooldown,
    Transition,
    IntervalWarmup,
    RwdStand,
    WorkoutRound,
    IntervalRecovery,
    RunRest,
    SkiLiftSplit,
    RunActive,
    ClimbActive,
    AscentSplit,
    IntervalActive,
    DescentSplit,
    IntervalRest,
    SurfActive,
    RwdRun,
    RwdWalk,
    WindsurfActive,
    IntervalOther,
    SkiRunSplit,
    UnknownVariant,
}
impl SplitType {
    pub fn from(content: u8) -> SplitType {
        match content {
            10 => SplitType::ClimbRest,
            6 => SplitType::IntervalCooldown,
            23 => SplitType::Transition,
            5 => SplitType::IntervalWarmup,
            22 => SplitType::RwdStand,
            14 => SplitType::WorkoutRound,
            7 => SplitType::IntervalRecovery,
            13 => SplitType::RunRest,
            28 => SplitType::SkiLiftSplit,
            12 => SplitType::RunActive,
            9 => SplitType::ClimbActive,
            1 => SplitType::AscentSplit,
            3 => SplitType::IntervalActive,
            2 => SplitType::DescentSplit,
            4 => SplitType::IntervalRest,
            11 => SplitType::SurfActive,
            17 => SplitType::RwdRun,
            18 => SplitType::RwdWalk,
            21 => SplitType::WindsurfActive,
            8 => SplitType::IntervalOther,
            29 => SplitType::SkiRunSplit,
            _ => SplitType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::SplitType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum ClimbProEvent {
    Start,
    Approach,
    Complete,
    UnknownVariant,
}
impl ClimbProEvent {
    pub fn from(content: u8) -> ClimbProEvent {
        match content {
            1 => ClimbProEvent::Start,
            0 => ClimbProEvent::Approach,
            2 => ClimbProEvent::Complete,
            _ => ClimbProEvent::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::ClimbProEvent(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum GasConsumptionRateType {
    VolumeSac,
    PressureSac,
    Rmv,
    UnknownVariant,
}
impl GasConsumptionRateType {
    pub fn from(content: u8) -> GasConsumptionRateType {
        match content {
            1 => GasConsumptionRateType::VolumeSac,
            0 => GasConsumptionRateType::PressureSac,
            2 => GasConsumptionRateType::Rmv,
            _ => GasConsumptionRateType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::GasConsumptionRateType(
                Self::from(reader.next_u8()?),
            )));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum TapSensitivity {
    Low,
    Medium,
    High,
    UnknownVariant,
}
impl TapSensitivity {
    pub fn from(content: u8) -> TapSensitivity {
        match content {
            2 => TapSensitivity::Low,
            1 => TapSensitivity::Medium,
            0 => TapSensitivity::High,
            _ => TapSensitivity::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::TapSensitivity(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum RadarThreatLevelType {
    ThreatApproaching,
    ThreatUnknown,
    ThreatApproachingFast,
    ThreatNone,
    UnknownVariant,
}
impl RadarThreatLevelType {
    pub fn from(content: u8) -> RadarThreatLevelType {
        match content {
            2 => RadarThreatLevelType::ThreatApproaching,
            0 => RadarThreatLevelType::ThreatUnknown,
            3 => RadarThreatLevelType::ThreatApproachingFast,
            1 => RadarThreatLevelType::ThreatNone,
            _ => RadarThreatLevelType::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::RadarThreatLevelType(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum MaxMetSpeedSource {
    Cadence,
    OnboardGps,
    ConnectedGps,
    UnknownVariant,
}
impl MaxMetSpeedSource {
    pub fn from(content: u8) -> MaxMetSpeedSource {
        match content {
            2 => MaxMetSpeedSource::Cadence,
            0 => MaxMetSpeedSource::OnboardGps,
            1 => MaxMetSpeedSource::ConnectedGps,
            _ => MaxMetSpeedSource::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::MaxMetSpeedSource(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum MaxMetHeartRateSource {
    Whr,
    Hrm,
    UnknownVariant,
}
impl MaxMetHeartRateSource {
    pub fn from(content: u8) -> MaxMetHeartRateSource {
        match content {
            0 => MaxMetHeartRateSource::Whr,
            1 => MaxMetHeartRateSource::Hrm,
            _ => MaxMetHeartRateSource::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::MaxMetHeartRateSource(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum HrvStatus {
    Poor,
    None,
    Balanced,
    Low,
    Unbalanced,
    UnknownVariant,
}
impl HrvStatus {
    pub fn from(content: u8) -> HrvStatus {
        match content {
            1 => HrvStatus::Poor,
            0 => HrvStatus::None,
            4 => HrvStatus::Balanced,
            2 => HrvStatus::Low,
            3 => HrvStatus::Unbalanced,
            _ => HrvStatus::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::HrvStatus(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum NoFlyTimeMode {
    Flat24Hours,
    Standard,
    UnknownVariant,
}
impl NoFlyTimeMode {
    pub fn from(content: u8) -> NoFlyTimeMode {
        match content {
            1 => NoFlyTimeMode::Flat24Hours,
            0 => NoFlyTimeMode::Standard,
            _ => NoFlyTimeMode::UnknownVariant,
        }
    }

    pub fn parse(
        reader: &mut Reader,
        endianness: &Endianness,
        number_of_bytes: u8,
    ) -> Result<Vec<DataValue>, DataTypeError> {
        let mut values = Vec::new();
        for _ in 0..number_of_bytes {
            values.push(DataValue::Enum(FitEnum::NoFlyTimeMode(Self::from(
                reader.next_u8()?,
            ))));
        }
        Ok(values)
    }
}

#[derive(Debug, PartialEq)]
pub enum FitMessage {
    FileId(FileIdMesg),
    FileCreator(FileCreatorMesg),
    TimestampCorrelation(TimestampCorrelationMesg),
    Software(SoftwareMesg),
    SlaveDevice(SlaveDeviceMesg),
    Capabilities(CapabilitiesMesg),
    FileCapabilities(FileCapabilitiesMesg),
    MesgCapabilities(MesgCapabilitiesMesg),
    FieldCapabilities(FieldCapabilitiesMesg),
    DeviceSettings(DeviceSettingsMesg),
    UserProfile(UserProfileMesg),
    HrmProfile(HrmProfileMesg),
    SdmProfile(SdmProfileMesg),
    BikeProfile(BikeProfileMesg),
    Connectivity(ConnectivityMesg),
    WatchfaceSettings(WatchfaceSettingsMesg),
    OhrSettings(OhrSettingsMesg),
    TimeInZone(TimeInZoneMesg),
    ZonesTarget(ZonesTargetMesg),
    Sport(SportMesg),
    HrZone(HrZoneMesg),
    SpeedZone(SpeedZoneMesg),
    CadenceZone(CadenceZoneMesg),
    PowerZone(PowerZoneMesg),
    MetZone(MetZoneMesg),
    TrainingSettings(TrainingSettingsMesg),
    DiveSettings(DiveSettingsMesg),
    DiveAlarm(DiveAlarmMesg),
    DiveApneaAlarm(DiveApneaAlarmMesg),
    DiveGas(DiveGasMesg),
    Goal(GoalMesg),
    Activity(ActivityMesg),
    Session(SessionMesg),
    Lap(LapMesg),
    Length(LengthMesg),
    Record(RecordMesg),
    Event(EventMesg),
    DeviceInfo(DeviceInfoMesg),
    DeviceAuxBatteryInfo(DeviceAuxBatteryInfoMesg),
    TrainingFile(TrainingFileMesg),
    WeatherConditions(WeatherConditionsMesg),
    WeatherAlert(WeatherAlertMesg),
    GpsMetadata(GpsMetadataMesg),
    CameraEvent(CameraEventMesg),
    GyroscopeData(GyroscopeDataMesg),
    AccelerometerData(AccelerometerDataMesg),
    MagnetometerData(MagnetometerDataMesg),
    BarometerData(BarometerDataMesg),
    ThreeDSensorCalibration(ThreeDSensorCalibrationMesg),
    OneDSensorCalibration(OneDSensorCalibrationMesg),
    VideoFrame(VideoFrameMesg),
    ObdiiData(ObdiiDataMesg),
    NmeaSentence(NmeaSentenceMesg),
    AviationAttitude(AviationAttitudeMesg),
    Video(VideoMesg),
    VideoTitle(VideoTitleMesg),
    VideoDescription(VideoDescriptionMesg),
    VideoClip(VideoClipMesg),
    Set(SetMesg),
    Jump(JumpMesg),
    Split(SplitMesg),
    SplitSummary(SplitSummaryMesg),
    ClimbPro(ClimbProMesg),
    FieldDescription(FieldDescriptionMesg),
    DeveloperDataId(DeveloperDataIdMesg),
    Course(CourseMesg),
    CoursePoint(CoursePointMesg),
    SegmentId(SegmentIdMesg),
    SegmentLeaderboardEntry(SegmentLeaderboardEntryMesg),
    SegmentPoint(SegmentPointMesg),
    SegmentLap(SegmentLapMesg),
    SegmentFile(SegmentFileMesg),
    Workout(WorkoutMesg),
    WorkoutSession(WorkoutSessionMesg),
    WorkoutStep(WorkoutStepMesg),
    ExerciseTitle(ExerciseTitleMesg),
    Schedule(ScheduleMesg),
    Totals(TotalsMesg),
    WeightScale(WeightScaleMesg),
    BloodPressure(BloodPressureMesg),
    MonitoringInfo(MonitoringInfoMesg),
    Monitoring(MonitoringMesg),
    MonitoringHrData(MonitoringHrDataMesg),
    Spo2Data(Spo2DataMesg),
    Hr(HrMesg),
    StressLevel(StressLevelMesg),
    MaxMetData(MaxMetDataMesg),
    HsaBodyBatteryData(HsaBodyBatteryDataMesg),
    HsaEvent(HsaEventMesg),
    HsaAccelerometerData(HsaAccelerometerDataMesg),
    HsaGyroscopeData(HsaGyroscopeDataMesg),
    HsaStepData(HsaStepDataMesg),
    HsaSpo2Data(HsaSpo2DataMesg),
    HsaStressData(HsaStressDataMesg),
    HsaRespirationData(HsaRespirationDataMesg),
    HsaHeartRateData(HsaHeartRateDataMesg),
    HsaConfigurationData(HsaConfigurationDataMesg),
    HsaWristTemperatureData(HsaWristTemperatureDataMesg),
    MemoGlob(MemoGlobMesg),
    SleepLevel(SleepLevelMesg),
    AntChannelId(AntChannelIdMesg),
    AntRx(AntRxMesg),
    AntTx(AntTxMesg),
    ExdScreenConfiguration(ExdScreenConfigurationMesg),
    ExdDataFieldConfiguration(ExdDataFieldConfigurationMesg),
    ExdDataConceptConfiguration(ExdDataConceptConfigurationMesg),
    DiveSummary(DiveSummaryMesg),
    AadAccelFeatures(AadAccelFeaturesMesg),
    Hrv(HrvMesg),
    BeatIntervals(BeatIntervalsMesg),
    HrvStatusSummary(HrvStatusSummaryMesg),
    HrvValue(HrvValueMesg),
    RawBbi(RawBbiMesg),
    RespirationRate(RespirationRateMesg),
    ChronoShotSession(ChronoShotSessionMesg),
    ChronoShotData(ChronoShotDataMesg),
    TankUpdate(TankUpdateMesg),
    TankSummary(TankSummaryMesg),
    SleepAssessment(SleepAssessmentMesg),
    SkinTempOvernight(SkinTempOvernightMesg),
}

#[derive(Debug, PartialEq)]
pub enum FileIdMesg {
    Type,
    Manufacturer,
    Product,
    SerialNumber,
    TimeCreated,
    Number,
    ProductName,
}
impl FileIdMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => File::parse,
            1 => Manufacturer::parse,
            2 => parse_uint16,
            3 => parse_unknown,
            4 => DateTime::parse,
            5 => parse_uint16,
            8 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum FileCreatorMesg {
    SoftwareVersion,
    HardwareVersion,
}
impl FileCreatorMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint16,
            1 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum TimestampCorrelationMesg {
    Timestamp,
    FractionalTimestamp,
    SystemTimestamp,
    FractionalSystemTimestamp,
    LocalTimestamp,
    TimestampMs,
    SystemTimestampMs,
}
impl TimestampCorrelationMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => DateTime::parse,
            2 => parse_uint16,
            3 => LocalDateTime::parse,
            4 => parse_uint16,
            5 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SoftwareMesg {
    MessageIndex,
    Version,
    PartNumber,
}
impl SoftwareMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            3 => parse_uint16,
            5 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SlaveDeviceMesg {
    Manufacturer,
    Product,
}
impl SlaveDeviceMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => Manufacturer::parse,
            1 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum CapabilitiesMesg {
    Languages,
    Sports,
    WorkoutsSupported,
    ConnectivitySupported,
}
impl CapabilitiesMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_unknown,
            1 => SportBits0::parse,
            21 => WorkoutCapabilities::parse,
            23 => ConnectivityCapabilities::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum FileCapabilitiesMesg {
    MessageIndex,
    Type,
    Flags,
    Directory,
    MaxCount,
    MaxSize,
}
impl FileCapabilitiesMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => File::parse,
            1 => FileFlags::parse,
            2 => parse_string,
            3 => parse_uint16,
            4 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum MesgCapabilitiesMesg {
    MessageIndex,
    File,
    MesgNum,
    CountType,
    Count,
}
impl MesgCapabilitiesMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => File::parse,
            1 => MesgNum::parse,
            2 => MesgCount::parse,
            3 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum FieldCapabilitiesMesg {
    MessageIndex,
    File,
    MesgNum,
    FieldNum,
    Count,
}
impl FieldCapabilitiesMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => File::parse,
            1 => MesgNum::parse,
            2 => parse_uint8,
            3 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum DeviceSettingsMesg {
    ActiveTimeZone,
    UtcOffset,
    TimeOffset,
    TimeMode,
    TimeZoneOffset,
    BacklightMode,
    ActivityTrackerEnabled,
    ClockTime,
    PagesEnabled,
    MoveAlertEnabled,
    DateMode,
    DisplayOrientation,
    MountingSide,
    DefaultPage,
    AutosyncMinSteps,
    AutosyncMinTime,
    LactateThresholdAutodetectEnabled,
    BleAutoUploadEnabled,
    AutoSyncFrequency,
    AutoActivityDetect,
    NumberOfScreens,
    SmartNotificationDisplayOrientation,
    TapInterface,
    TapSensitivity,
}
impl DeviceSettingsMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint8,
            1 => parse_uint32,
            2 => parse_uint32,
            4 => TimeMode::parse,
            5 => parse_sint8,
            12 => BacklightMode::parse,
            36 => parse_unknown,
            39 => DateTime::parse,
            40 => parse_uint16,
            46 => parse_unknown,
            47 => DateMode::parse,
            55 => DisplayOrientation::parse,
            56 => Side::parse,
            57 => parse_uint16,
            58 => parse_uint16,
            59 => parse_uint16,
            80 => parse_unknown,
            86 => parse_unknown,
            89 => AutoSyncFrequency::parse,
            90 => AutoActivityDetect::parse,
            94 => parse_uint8,
            95 => DisplayOrientation::parse,
            134 => Switch::parse,
            174 => TapSensitivity::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum UserProfileMesg {
    MessageIndex,
    FriendlyName,
    Gender,
    Age,
    Height,
    Weight,
    Language,
    ElevSetting,
    WeightSetting,
    RestingHeartRate,
    DefaultMaxRunningHeartRate,
    DefaultMaxBikingHeartRate,
    DefaultMaxHeartRate,
    HrSetting,
    SpeedSetting,
    DistSetting,
    PowerSetting,
    ActivityClass,
    PositionSetting,
    TemperatureSetting,
    LocalId,
    GlobalId,
    WakeTime,
    SleepTime,
    HeightSetting,
    UserRunningStepLength,
    UserWalkingStepLength,
    DepthSetting,
    DiveCount,
}
impl UserProfileMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_string,
            1 => Gender::parse,
            2 => parse_uint8,
            3 => parse_uint8,
            4 => parse_uint16,
            5 => Language::parse,
            6 => DisplayMeasure::parse,
            7 => DisplayMeasure::parse,
            8 => parse_uint8,
            9 => parse_uint8,
            10 => parse_uint8,
            11 => parse_uint8,
            12 => DisplayHeart::parse,
            13 => DisplayMeasure::parse,
            14 => DisplayMeasure::parse,
            16 => DisplayPower::parse,
            17 => ActivityClass::parse,
            18 => DisplayPosition::parse,
            21 => DisplayMeasure::parse,
            22 => UserLocalId::parse,
            23 => parse_byte,
            28 => LocaltimeIntoDay::parse,
            29 => LocaltimeIntoDay::parse,
            30 => DisplayMeasure::parse,
            31 => parse_uint16,
            32 => parse_uint16,
            47 => DisplayMeasure::parse,
            49 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HrmProfileMesg {
    MessageIndex,
    Enabled,
    HrmAntId,
    LogHrv,
    HrmAntIdTransType,
}
impl HrmProfileMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_unknown,
            1 => parse_unknown,
            2 => parse_unknown,
            3 => parse_unknown,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SdmProfileMesg {
    MessageIndex,
    Enabled,
    SdmAntId,
    SdmCalFactor,
    Odometer,
    SpeedSource,
    SdmAntIdTransType,
    OdometerRollover,
}
impl SdmProfileMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_unknown,
            1 => parse_unknown,
            2 => parse_uint16,
            3 => parse_uint32,
            4 => parse_unknown,
            5 => parse_unknown,
            7 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum BikeProfileMesg {
    MessageIndex,
    Name,
    Sport,
    SubSport,
    Odometer,
    BikeSpdAntId,
    BikeCadAntId,
    BikeSpdcadAntId,
    BikePowerAntId,
    CustomWheelsize,
    AutoWheelsize,
    BikeWeight,
    PowerCalFactor,
    AutoWheelCal,
    AutoPowerZero,
    Id,
    SpdEnabled,
    CadEnabled,
    SpdcadEnabled,
    PowerEnabled,
    CrankLength,
    Enabled,
    BikeSpdAntIdTransType,
    BikeCadAntIdTransType,
    BikeSpdcadAntIdTransType,
    BikePowerAntIdTransType,
    OdometerRollover,
    FrontGearNum,
    FrontGear,
    RearGearNum,
    RearGear,
    ShimanoDi2Enabled,
}
impl BikeProfileMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_string,
            1 => Sport::parse,
            2 => SubSport::parse,
            3 => parse_uint32,
            4 => parse_unknown,
            5 => parse_unknown,
            6 => parse_unknown,
            7 => parse_unknown,
            8 => parse_uint16,
            9 => parse_uint16,
            10 => parse_uint16,
            11 => parse_uint16,
            12 => parse_unknown,
            13 => parse_unknown,
            14 => parse_uint8,
            15 => parse_unknown,
            16 => parse_unknown,
            17 => parse_unknown,
            18 => parse_unknown,
            19 => parse_uint8,
            20 => parse_unknown,
            21 => parse_unknown,
            22 => parse_unknown,
            23 => parse_unknown,
            24 => parse_unknown,
            37 => parse_uint8,
            38 => parse_unknown,
            39 => parse_unknown,
            40 => parse_unknown,
            41 => parse_unknown,
            44 => parse_unknown,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ConnectivityMesg {
    BluetoothEnabled,
    BluetoothLeEnabled,
    AntEnabled,
    Name,
    LiveTrackingEnabled,
    WeatherConditionsEnabled,
    WeatherAlertsEnabled,
    AutoActivityUploadEnabled,
    CourseDownloadEnabled,
    WorkoutDownloadEnabled,
    GpsEphemerisDownloadEnabled,
    IncidentDetectionEnabled,
    GrouptrackEnabled,
}
impl ConnectivityMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_unknown,
            1 => parse_unknown,
            2 => parse_unknown,
            3 => parse_string,
            4 => parse_unknown,
            5 => parse_unknown,
            6 => parse_unknown,
            7 => parse_unknown,
            8 => parse_unknown,
            9 => parse_unknown,
            10 => parse_unknown,
            11 => parse_unknown,
            12 => parse_unknown,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum WatchfaceSettingsMesg {
    MessageIndex,
    Mode,
    Layout,
}
impl WatchfaceSettingsMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => WatchfaceMode::parse,
            1 => parse_byte,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum OhrSettingsMesg {
    Timestamp,
    Enabled,
}
impl OhrSettingsMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => Switch::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum TimeInZoneMesg {
    Timestamp,
    ReferenceMesg,
    ReferenceIndex,
    TimeInHrZone,
    TimeInSpeedZone,
    TimeInCadenceZone,
    TimeInPowerZone,
    HrZoneHighBoundary,
    SpeedZoneHighBoundary,
    CadenceZoneHighBondary,
    PowerZoneHighBoundary,
    HrCalcType,
    MaxHeartRate,
    RestingHeartRate,
    ThresholdHeartRate,
    PwrCalcType,
    FunctionalThresholdPower,
}
impl TimeInZoneMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => MesgNum::parse,
            1 => MessageIndex::parse,
            2 => parse_uint32,
            3 => parse_uint32,
            4 => parse_uint32,
            5 => parse_uint32,
            6 => parse_uint8,
            7 => parse_uint16,
            8 => parse_uint8,
            9 => parse_uint16,
            10 => HrZoneCalc::parse,
            11 => parse_uint8,
            12 => parse_uint8,
            13 => parse_uint8,
            14 => PwrZoneCalc::parse,
            15 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ZonesTargetMesg {
    MaxHeartRate,
    ThresholdHeartRate,
    FunctionalThresholdPower,
    HrCalcType,
    PwrCalcType,
}
impl ZonesTargetMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            1 => parse_uint8,
            2 => parse_uint8,
            3 => parse_uint16,
            5 => HrZoneCalc::parse,
            7 => PwrZoneCalc::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SportMesg {
    Sport,
    SubSport,
    Name,
}
impl SportMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => Sport::parse,
            1 => SubSport::parse,
            3 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HrZoneMesg {
    MessageIndex,
    HighBpm,
    Name,
}
impl HrZoneMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            1 => parse_uint8,
            2 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SpeedZoneMesg {
    MessageIndex,
    HighValue,
    Name,
}
impl SpeedZoneMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_uint16,
            1 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum CadenceZoneMesg {
    MessageIndex,
    HighValue,
    Name,
}
impl CadenceZoneMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_uint8,
            1 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum PowerZoneMesg {
    MessageIndex,
    HighValue,
    Name,
}
impl PowerZoneMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            1 => parse_uint16,
            2 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum MetZoneMesg {
    MessageIndex,
    HighBpm,
    Calories,
    FatCalories,
}
impl MetZoneMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            1 => parse_uint8,
            2 => parse_uint16,
            3 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum TrainingSettingsMesg {
    TargetDistance,
    TargetSpeed,
    TargetTime,
    PreciseTargetSpeed,
}
impl TrainingSettingsMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            31 => parse_uint32,
            32 => parse_uint16,
            33 => parse_uint32,
            153 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum DiveSettingsMesg {
    Timestamp,
    MessageIndex,
    Name,
    Model,
    GfLow,
    GfHigh,
    WaterType,
    WaterDensity,
    Po2Warn,
    Po2Critical,
    Po2Deco,
    SafetyStopEnabled,
    BottomDepth,
    BottomTime,
    ApneaCountdownEnabled,
    ApneaCountdownTime,
    BacklightMode,
    BacklightBrightness,
    BacklightTimeout,
    RepeatDiveInterval,
    SafetyStopTime,
    HeartRateSourceType,
    HeartRateSource,
    TravelGas,
    CcrLowSetpointSwitchMode,
    CcrLowSetpoint,
    CcrLowSetpointDepth,
    CcrHighSetpointSwitchMode,
    CcrHighSetpoint,
    CcrHighSetpointDepth,
    GasConsumptionDisplay,
    UpKeyEnabled,
    DiveSounds,
    LastStopMultiple,
    NoFlyTimeMode,
}
impl DiveSettingsMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            254 => MessageIndex::parse,
            0 => parse_string,
            1 => TissueModelType::parse,
            2 => parse_uint8,
            3 => parse_uint8,
            4 => WaterType::parse,
            5 => parse_float32,
            6 => parse_uint8,
            7 => parse_uint8,
            8 => parse_uint8,
            9 => parse_unknown,
            10 => parse_float32,
            11 => parse_uint32,
            12 => parse_unknown,
            13 => parse_uint32,
            14 => DiveBacklightMode::parse,
            15 => parse_uint8,
            16 => BacklightTimeout::parse,
            17 => parse_uint16,
            18 => parse_uint16,
            19 => SourceType::parse,
            20 => parse_uint8,
            21 => MessageIndex::parse,
            22 => CcrSetpointSwitchMode::parse,
            23 => parse_uint8,
            24 => parse_uint32,
            25 => CcrSetpointSwitchMode::parse,
            26 => parse_uint8,
            27 => parse_uint32,
            29 => GasConsumptionRateType::parse,
            30 => parse_unknown,
            35 => Tone::parse,
            36 => parse_uint8,
            37 => NoFlyTimeMode::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum DiveAlarmMesg {
    MessageIndex,
    Depth,
    Time,
    Enabled,
    AlarmType,
    Sound,
    DiveTypes,
    Id,
    PopupEnabled,
    TriggerOnDescent,
    TriggerOnAscent,
    Repeating,
    Speed,
}
impl DiveAlarmMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_uint32,
            1 => parse_sint32,
            2 => parse_unknown,
            3 => DiveAlarmType::parse,
            4 => Tone::parse,
            5 => SubSport::parse,
            6 => parse_uint32,
            7 => parse_unknown,
            8 => parse_unknown,
            9 => parse_unknown,
            10 => parse_unknown,
            11 => parse_sint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum DiveApneaAlarmMesg {
    MessageIndex,
    Depth,
    Time,
    Enabled,
    AlarmType,
    Sound,
    DiveTypes,
    Id,
    PopupEnabled,
    TriggerOnDescent,
    TriggerOnAscent,
    Repeating,
    Speed,
}
impl DiveApneaAlarmMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_uint32,
            1 => parse_sint32,
            2 => parse_unknown,
            3 => DiveAlarmType::parse,
            4 => Tone::parse,
            5 => SubSport::parse,
            6 => parse_uint32,
            7 => parse_unknown,
            8 => parse_unknown,
            9 => parse_unknown,
            10 => parse_unknown,
            11 => parse_sint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum DiveGasMesg {
    MessageIndex,
    HeliumContent,
    OxygenContent,
    Status,
    Mode,
}
impl DiveGasMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_uint8,
            1 => parse_uint8,
            2 => DiveGasStatus::parse,
            3 => DiveGasMode::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum GoalMesg {
    MessageIndex,
    Sport,
    SubSport,
    StartDate,
    EndDate,
    Type,
    Value,
    Repeat,
    TargetValue,
    Recurrence,
    RecurrenceValue,
    Enabled,
    Source,
}
impl GoalMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => Sport::parse,
            1 => SubSport::parse,
            2 => DateTime::parse,
            3 => DateTime::parse,
            4 => Goal::parse,
            5 => parse_uint32,
            6 => parse_unknown,
            7 => parse_uint32,
            8 => GoalRecurrence::parse,
            9 => parse_uint16,
            10 => parse_unknown,
            11 => GoalSource::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ActivityMesg {
    Timestamp,
    TotalTimerTime,
    NumSessions,
    Type,
    Event,
    EventType,
    LocalTimestamp,
    EventGroup,
}
impl ActivityMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint32,
            1 => parse_uint16,
            2 => Activity::parse,
            3 => Event::parse,
            4 => EventType::parse,
            5 => LocalDateTime::parse,
            6 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SessionMesg {
    MessageIndex,
    Timestamp,
    Event,
    EventType,
    StartTime,
    StartPositionLat,
    StartPositionLong,
    Sport,
    SubSport,
    TotalElapsedTime,
    TotalTimerTime,
    TotalDistance,
    TotalCycles,
    TotalCalories,
    TotalFatCalories,
    AvgSpeed,
    MaxSpeed,
    AvgHeartRate,
    MaxHeartRate,
    AvgCadence,
    MaxCadence,
    AvgPower,
    MaxPower,
    TotalAscent,
    TotalDescent,
    TotalTrainingEffect,
    FirstLapIndex,
    NumLaps,
    EventGroup,
    Trigger,
    NecLat,
    NecLong,
    SwcLat,
    SwcLong,
    NumLengths,
    NormalizedPower,
    TrainingStressScore,
    IntensityFactor,
    LeftRightBalance,
    EndPositionLat,
    EndPositionLong,
    AvgStrokeCount,
    AvgStrokeDistance,
    SwimStroke,
    PoolLength,
    ThresholdPower,
    PoolLengthUnit,
    NumActiveLengths,
    TotalWork,
    AvgAltitude,
    MaxAltitude,
    GpsAccuracy,
    AvgGrade,
    AvgPosGrade,
    AvgNegGrade,
    MaxPosGrade,
    MaxNegGrade,
    AvgTemperature,
    MaxTemperature,
    TotalMovingTime,
    AvgPosVerticalSpeed,
    AvgNegVerticalSpeed,
    MaxPosVerticalSpeed,
    MaxNegVerticalSpeed,
    MinHeartRate,
    TimeInHrZone,
    TimeInSpeedZone,
    TimeInCadenceZone,
    TimeInPowerZone,
    AvgLapTime,
    BestLapIndex,
    MinAltitude,
    PlayerScore,
    OpponentScore,
    OpponentName,
    StrokeCount,
    ZoneCount,
    MaxBallSpeed,
    AvgBallSpeed,
    AvgVerticalOscillation,
    AvgStanceTimePercent,
    AvgStanceTime,
    AvgFractionalCadence,
    MaxFractionalCadence,
    TotalFractionalCycles,
    AvgTotalHemoglobinConc,
    MinTotalHemoglobinConc,
    MaxTotalHemoglobinConc,
    AvgSaturatedHemoglobinPercent,
    MinSaturatedHemoglobinPercent,
    MaxSaturatedHemoglobinPercent,
    AvgLeftTorqueEffectiveness,
    AvgRightTorqueEffectiveness,
    AvgLeftPedalSmoothness,
    AvgRightPedalSmoothness,
    AvgCombinedPedalSmoothness,
    SportProfileName,
    SportIndex,
    TimeStanding,
    StandCount,
    AvgLeftPco,
    AvgRightPco,
    AvgLeftPowerPhase,
    AvgLeftPowerPhasePeak,
    AvgRightPowerPhase,
    AvgRightPowerPhasePeak,
    AvgPowerPosition,
    MaxPowerPosition,
    AvgCadencePosition,
    MaxCadencePosition,
    EnhancedAvgSpeed,
    EnhancedMaxSpeed,
    EnhancedAvgAltitude,
    EnhancedMinAltitude,
    EnhancedMaxAltitude,
    AvgLevMotorPower,
    MaxLevMotorPower,
    LevBatteryConsumption,
    AvgVerticalRatio,
    AvgStanceTimeBalance,
    AvgStepLength,
    TotalAnaerobicTrainingEffect,
    AvgVam,
    AvgDepth,
    MaxDepth,
    SurfaceInterval,
    StartCns,
    EndCns,
    StartN2,
    EndN2,
    AvgRespirationRate,
    MaxRespirationRate,
    MinRespirationRate,
    MinTemperature,
    O2Toxicity,
    DiveNumber,
    TrainingLoadPeak,
    EnhancedAvgRespirationRate,
    EnhancedMaxRespirationRate,
    EnhancedMinRespirationRate,
    TotalGrit,
    TotalFlow,
    JumpCount,
    AvgGrit,
    AvgFlow,
    WorkoutFeel,
    WorkoutRpe,
    AvgSpo2,
    AvgStress,
    SdrrHrv,
    RmssdHrv,
    TotalFractionalAscent,
    TotalFractionalDescent,
    AvgCoreTemperature,
    MinCoreTemperature,
    MaxCoreTemperature,
}
impl SessionMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            253 => DateTime::parse,
            0 => Event::parse,
            1 => EventType::parse,
            2 => DateTime::parse,
            3 => parse_sint32,
            4 => parse_sint32,
            5 => Sport::parse,
            6 => SubSport::parse,
            7 => parse_uint32,
            8 => parse_uint32,
            9 => parse_uint32,
            10 => parse_uint32,
            11 => parse_uint16,
            13 => parse_uint16,
            14 => parse_uint16,
            15 => parse_uint16,
            16 => parse_uint8,
            17 => parse_uint8,
            18 => parse_uint8,
            19 => parse_uint8,
            20 => parse_uint16,
            21 => parse_uint16,
            22 => parse_uint16,
            23 => parse_uint16,
            24 => parse_uint8,
            25 => parse_uint16,
            26 => parse_uint16,
            27 => parse_uint8,
            28 => SessionTrigger::parse,
            29 => parse_sint32,
            30 => parse_sint32,
            31 => parse_sint32,
            32 => parse_sint32,
            33 => parse_uint16,
            34 => parse_uint16,
            35 => parse_uint16,
            36 => parse_uint16,
            37 => LeftRightBalance100::parse,
            38 => parse_sint32,
            39 => parse_sint32,
            41 => parse_uint32,
            42 => parse_uint16,
            43 => SwimStroke::parse,
            44 => parse_uint16,
            45 => parse_uint16,
            46 => DisplayMeasure::parse,
            47 => parse_uint16,
            48 => parse_uint32,
            49 => parse_uint16,
            50 => parse_uint16,
            51 => parse_uint8,
            52 => parse_sint16,
            53 => parse_sint16,
            54 => parse_sint16,
            55 => parse_sint16,
            56 => parse_sint16,
            57 => parse_sint8,
            58 => parse_sint8,
            59 => parse_uint32,
            60 => parse_sint16,
            61 => parse_sint16,
            62 => parse_sint16,
            63 => parse_sint16,
            64 => parse_uint8,
            65 => parse_uint32,
            66 => parse_uint32,
            67 => parse_uint32,
            68 => parse_uint32,
            69 => parse_uint32,
            70 => parse_uint16,
            71 => parse_uint16,
            82 => parse_uint16,
            83 => parse_uint16,
            84 => parse_string,
            85 => parse_uint16,
            86 => parse_uint16,
            87 => parse_uint16,
            88 => parse_uint16,
            89 => parse_uint16,
            90 => parse_uint16,
            91 => parse_uint16,
            92 => parse_uint8,
            93 => parse_uint8,
            94 => parse_uint8,
            95 => parse_uint16,
            96 => parse_uint16,
            97 => parse_uint16,
            98 => parse_uint16,
            99 => parse_uint16,
            100 => parse_uint16,
            101 => parse_uint8,
            102 => parse_uint8,
            103 => parse_uint8,
            104 => parse_uint8,
            105 => parse_uint8,
            110 => parse_string,
            111 => parse_uint8,
            112 => parse_uint32,
            113 => parse_uint16,
            114 => parse_sint8,
            115 => parse_sint8,
            116 => parse_uint8,
            117 => parse_uint8,
            118 => parse_uint8,
            119 => parse_uint8,
            120 => parse_uint16,
            121 => parse_uint16,
            122 => parse_uint8,
            123 => parse_uint8,
            124 => parse_uint32,
            125 => parse_uint32,
            126 => parse_uint32,
            127 => parse_uint32,
            128 => parse_uint32,
            129 => parse_uint16,
            130 => parse_uint16,
            131 => parse_uint8,
            132 => parse_uint16,
            133 => parse_uint16,
            134 => parse_uint16,
            137 => parse_uint8,
            139 => parse_uint16,
            140 => parse_uint32,
            141 => parse_uint32,
            142 => parse_uint32,
            143 => parse_uint8,
            144 => parse_uint8,
            145 => parse_uint16,
            146 => parse_uint16,
            147 => parse_uint8,
            148 => parse_uint8,
            149 => parse_uint8,
            150 => parse_sint8,
            155 => parse_uint16,
            156 => parse_uint32,
            168 => parse_sint32,
            169 => parse_uint16,
            170 => parse_uint16,
            180 => parse_uint16,
            181 => parse_float32,
            182 => parse_float32,
            183 => parse_uint16,
            186 => parse_float32,
            187 => parse_float32,
            192 => parse_uint8,
            193 => parse_uint8,
            194 => parse_uint8,
            195 => parse_uint8,
            197 => parse_uint8,
            198 => parse_uint8,
            199 => parse_uint8,
            200 => parse_uint8,
            208 => parse_uint16,
            209 => parse_uint16,
            210 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum LapMesg {
    MessageIndex,
    Timestamp,
    Event,
    EventType,
    StartTime,
    StartPositionLat,
    StartPositionLong,
    EndPositionLat,
    EndPositionLong,
    TotalElapsedTime,
    TotalTimerTime,
    TotalDistance,
    TotalCycles,
    TotalCalories,
    TotalFatCalories,
    AvgSpeed,
    MaxSpeed,
    AvgHeartRate,
    MaxHeartRate,
    AvgCadence,
    MaxCadence,
    AvgPower,
    MaxPower,
    TotalAscent,
    TotalDescent,
    Intensity,
    LapTrigger,
    Sport,
    EventGroup,
    NumLengths,
    NormalizedPower,
    LeftRightBalance,
    FirstLengthIndex,
    AvgStrokeDistance,
    SwimStroke,
    SubSport,
    NumActiveLengths,
    TotalWork,
    AvgAltitude,
    MaxAltitude,
    GpsAccuracy,
    AvgGrade,
    AvgPosGrade,
    AvgNegGrade,
    MaxPosGrade,
    MaxNegGrade,
    AvgTemperature,
    MaxTemperature,
    TotalMovingTime,
    AvgPosVerticalSpeed,
    AvgNegVerticalSpeed,
    MaxPosVerticalSpeed,
    MaxNegVerticalSpeed,
    TimeInHrZone,
    TimeInSpeedZone,
    TimeInCadenceZone,
    TimeInPowerZone,
    RepetitionNum,
    MinAltitude,
    MinHeartRate,
    WktStepIndex,
    OpponentScore,
    StrokeCount,
    ZoneCount,
    AvgVerticalOscillation,
    AvgStanceTimePercent,
    AvgStanceTime,
    AvgFractionalCadence,
    MaxFractionalCadence,
    TotalFractionalCycles,
    PlayerScore,
    AvgTotalHemoglobinConc,
    MinTotalHemoglobinConc,
    MaxTotalHemoglobinConc,
    AvgSaturatedHemoglobinPercent,
    MinSaturatedHemoglobinPercent,
    MaxSaturatedHemoglobinPercent,
    AvgLeftTorqueEffectiveness,
    AvgRightTorqueEffectiveness,
    AvgLeftPedalSmoothness,
    AvgRightPedalSmoothness,
    AvgCombinedPedalSmoothness,
    TimeStanding,
    StandCount,
    AvgLeftPco,
    AvgRightPco,
    AvgLeftPowerPhase,
    AvgLeftPowerPhasePeak,
    AvgRightPowerPhase,
    AvgRightPowerPhasePeak,
    AvgPowerPosition,
    MaxPowerPosition,
    AvgCadencePosition,
    MaxCadencePosition,
    EnhancedAvgSpeed,
    EnhancedMaxSpeed,
    EnhancedAvgAltitude,
    EnhancedMinAltitude,
    EnhancedMaxAltitude,
    AvgLevMotorPower,
    MaxLevMotorPower,
    LevBatteryConsumption,
    AvgVerticalRatio,
    AvgStanceTimeBalance,
    AvgStepLength,
    AvgVam,
    AvgDepth,
    MaxDepth,
    MinTemperature,
    EnhancedAvgRespirationRate,
    EnhancedMaxRespirationRate,
    AvgRespirationRate,
    MaxRespirationRate,
    TotalGrit,
    TotalFlow,
    JumpCount,
    AvgGrit,
    AvgFlow,
    TotalFractionalAscent,
    TotalFractionalDescent,
    AvgCoreTemperature,
    MinCoreTemperature,
    MaxCoreTemperature,
}
impl LapMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            253 => DateTime::parse,
            0 => Event::parse,
            1 => EventType::parse,
            2 => DateTime::parse,
            3 => parse_sint32,
            4 => parse_sint32,
            5 => parse_sint32,
            6 => parse_sint32,
            7 => parse_uint32,
            8 => parse_uint32,
            9 => parse_uint32,
            10 => parse_uint32,
            11 => parse_uint16,
            12 => parse_uint16,
            13 => parse_uint16,
            14 => parse_uint16,
            15 => parse_uint8,
            16 => parse_uint8,
            17 => parse_uint8,
            18 => parse_uint8,
            19 => parse_uint16,
            20 => parse_uint16,
            21 => parse_uint16,
            22 => parse_uint16,
            23 => Intensity::parse,
            24 => LapTrigger::parse,
            25 => Sport::parse,
            26 => parse_uint8,
            32 => parse_uint16,
            33 => parse_uint16,
            34 => LeftRightBalance100::parse,
            35 => parse_uint16,
            37 => parse_uint16,
            38 => SwimStroke::parse,
            39 => SubSport::parse,
            40 => parse_uint16,
            41 => parse_uint32,
            42 => parse_uint16,
            43 => parse_uint16,
            44 => parse_uint8,
            45 => parse_sint16,
            46 => parse_sint16,
            47 => parse_sint16,
            48 => parse_sint16,
            49 => parse_sint16,
            50 => parse_sint8,
            51 => parse_sint8,
            52 => parse_uint32,
            53 => parse_sint16,
            54 => parse_sint16,
            55 => parse_sint16,
            56 => parse_sint16,
            57 => parse_uint32,
            58 => parse_uint32,
            59 => parse_uint32,
            60 => parse_uint32,
            61 => parse_uint16,
            62 => parse_uint16,
            63 => parse_uint8,
            71 => MessageIndex::parse,
            74 => parse_uint16,
            75 => parse_uint16,
            76 => parse_uint16,
            77 => parse_uint16,
            78 => parse_uint16,
            79 => parse_uint16,
            80 => parse_uint8,
            81 => parse_uint8,
            82 => parse_uint8,
            83 => parse_uint16,
            84 => parse_uint16,
            85 => parse_uint16,
            86 => parse_uint16,
            87 => parse_uint16,
            88 => parse_uint16,
            89 => parse_uint16,
            91 => parse_uint8,
            92 => parse_uint8,
            93 => parse_uint8,
            94 => parse_uint8,
            95 => parse_uint8,
            98 => parse_uint32,
            99 => parse_uint16,
            100 => parse_sint8,
            101 => parse_sint8,
            102 => parse_uint8,
            103 => parse_uint8,
            104 => parse_uint8,
            105 => parse_uint8,
            106 => parse_uint16,
            107 => parse_uint16,
            108 => parse_uint8,
            109 => parse_uint8,
            110 => parse_uint32,
            111 => parse_uint32,
            112 => parse_uint32,
            113 => parse_uint32,
            114 => parse_uint32,
            115 => parse_uint16,
            116 => parse_uint16,
            117 => parse_uint8,
            118 => parse_uint16,
            119 => parse_uint16,
            120 => parse_uint16,
            121 => parse_uint16,
            122 => parse_uint32,
            123 => parse_uint32,
            124 => parse_sint8,
            136 => parse_uint16,
            137 => parse_uint16,
            147 => parse_uint8,
            148 => parse_uint8,
            149 => parse_float32,
            150 => parse_float32,
            151 => parse_uint16,
            153 => parse_float32,
            154 => parse_float32,
            156 => parse_uint8,
            157 => parse_uint8,
            158 => parse_uint16,
            159 => parse_uint16,
            160 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum LengthMesg {
    MessageIndex,
    Timestamp,
    Event,
    EventType,
    StartTime,
    TotalElapsedTime,
    TotalTimerTime,
    TotalStrokes,
    AvgSpeed,
    SwimStroke,
    AvgSwimmingCadence,
    EventGroup,
    TotalCalories,
    LengthType,
    PlayerScore,
    OpponentScore,
    StrokeCount,
    ZoneCount,
    EnhancedAvgRespirationRate,
    EnhancedMaxRespirationRate,
    AvgRespirationRate,
    MaxRespirationRate,
}
impl LengthMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            253 => DateTime::parse,
            0 => Event::parse,
            1 => EventType::parse,
            2 => DateTime::parse,
            3 => parse_uint32,
            4 => parse_uint32,
            5 => parse_uint16,
            6 => parse_uint16,
            7 => SwimStroke::parse,
            9 => parse_uint8,
            10 => parse_uint8,
            11 => parse_uint16,
            12 => LengthType::parse,
            18 => parse_uint16,
            19 => parse_uint16,
            20 => parse_uint16,
            21 => parse_uint16,
            22 => parse_uint16,
            23 => parse_uint16,
            24 => parse_uint8,
            25 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum RecordMesg {
    Timestamp,
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
    EbikeTravelRange,
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
}
impl RecordMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_sint32,
            1 => parse_sint32,
            2 => parse_uint16,
            3 => parse_uint8,
            4 => parse_uint8,
            5 => parse_uint32,
            6 => parse_uint16,
            7 => parse_uint16,
            8 => parse_byte,
            9 => parse_sint16,
            10 => parse_uint8,
            11 => parse_sint32,
            12 => parse_uint8,
            13 => parse_sint8,
            17 => parse_uint8,
            18 => parse_uint8,
            19 => parse_uint32,
            28 => parse_uint16,
            29 => parse_uint32,
            30 => LeftRightBalance::parse,
            31 => parse_uint8,
            32 => parse_sint16,
            33 => parse_uint16,
            39 => parse_uint16,
            40 => parse_uint16,
            41 => parse_uint16,
            42 => ActivityType::parse,
            43 => parse_uint8,
            44 => parse_uint8,
            45 => parse_uint8,
            46 => parse_uint8,
            47 => parse_uint8,
            48 => parse_uint8,
            49 => StrokeType::parse,
            50 => parse_uint8,
            51 => parse_uint16,
            52 => parse_uint16,
            53 => parse_uint8,
            54 => parse_uint16,
            55 => parse_uint16,
            56 => parse_uint16,
            57 => parse_uint16,
            58 => parse_uint16,
            59 => parse_uint16,
            62 => DeviceIndex::parse,
            67 => parse_sint8,
            68 => parse_sint8,
            69 => parse_uint8,
            70 => parse_uint8,
            71 => parse_uint8,
            72 => parse_uint8,
            73 => parse_uint32,
            78 => parse_uint32,
            81 => parse_uint8,
            82 => parse_uint16,
            83 => parse_uint16,
            84 => parse_uint16,
            85 => parse_uint16,
            87 => parse_uint16,
            91 => parse_uint32,
            92 => parse_uint32,
            93 => parse_uint32,
            94 => parse_uint32,
            95 => parse_uint32,
            96 => parse_uint32,
            97 => parse_uint8,
            98 => parse_uint16,
            99 => parse_uint8,
            108 => parse_uint16,
            114 => parse_float32,
            115 => parse_float32,
            116 => parse_uint16,
            117 => parse_uint16,
            118 => parse_uint8,
            119 => parse_uint8,
            120 => parse_uint8,
            123 => parse_uint32,
            124 => parse_uint16,
            125 => parse_uint16,
            126 => parse_uint16,
            127 => parse_sint32,
            129 => parse_uint8,
            139 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum EventMesg {
    Timestamp,
    Event,
    EventType,
    Data16,
    Data,
    EventGroup,
    Score,
    OpponentScore,
    FrontGearNum,
    FrontGear,
    RearGearNum,
    RearGear,
    DeviceIndex,
    ActivityType,
    StartTimestamp,
    RadarThreatLevelMax,
    RadarThreatCount,
    RadarThreatAvgApproachSpeed,
    RadarThreatMaxApproachSpeed,
}
impl EventMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => Event::parse,
            1 => EventType::parse,
            2 => parse_uint16,
            3 => parse_uint32,
            4 => parse_uint8,
            7 => parse_uint16,
            8 => parse_uint16,
            9 => parse_unknown,
            10 => parse_unknown,
            11 => parse_unknown,
            12 => parse_unknown,
            13 => DeviceIndex::parse,
            14 => ActivityType::parse,
            15 => DateTime::parse,
            21 => RadarThreatLevelType::parse,
            22 => parse_uint8,
            23 => parse_uint8,
            24 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum DeviceInfoMesg {
    Timestamp,
    DeviceIndex,
    DeviceType,
    Manufacturer,
    SerialNumber,
    Product,
    SoftwareVersion,
    HardwareVersion,
    CumOperatingTime,
    BatteryVoltage,
    BatteryStatus,
    SensorPosition,
    Descriptor,
    AntTransmissionType,
    AntDeviceNumber,
    AntNetwork,
    SourceType,
    ProductName,
    BatteryLevel,
}
impl DeviceInfoMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => DeviceIndex::parse,
            1 => parse_uint8,
            2 => Manufacturer::parse,
            3 => parse_unknown,
            4 => parse_uint16,
            5 => parse_uint16,
            6 => parse_uint8,
            7 => parse_uint32,
            10 => parse_uint16,
            11 => BatteryStatus::parse,
            18 => BodyLocation::parse,
            19 => parse_string,
            20 => parse_unknown,
            21 => parse_unknown,
            22 => AntNetwork::parse,
            25 => SourceType::parse,
            27 => parse_string,
            32 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum DeviceAuxBatteryInfoMesg {
    Timestamp,
    DeviceIndex,
    BatteryVoltage,
    BatteryStatus,
    BatteryIdentifier,
}
impl DeviceAuxBatteryInfoMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => DeviceIndex::parse,
            1 => parse_uint16,
            2 => BatteryStatus::parse,
            3 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum TrainingFileMesg {
    Timestamp,
    Type,
    Manufacturer,
    Product,
    SerialNumber,
    TimeCreated,
}
impl TrainingFileMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => File::parse,
            1 => Manufacturer::parse,
            2 => parse_uint16,
            3 => parse_unknown,
            4 => DateTime::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum WeatherConditionsMesg {
    Timestamp,
    WeatherReport,
    Temperature,
    Condition,
    WindDirection,
    WindSpeed,
    PrecipitationProbability,
    TemperatureFeelsLike,
    RelativeHumidity,
    Location,
    ObservedAtTime,
    ObservedLocationLat,
    ObservedLocationLong,
    DayOfWeek,
    HighTemperature,
    LowTemperature,
}
impl WeatherConditionsMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => WeatherReport::parse,
            1 => parse_sint8,
            2 => WeatherStatus::parse,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => parse_uint8,
            6 => parse_sint8,
            7 => parse_uint8,
            8 => parse_string,
            9 => DateTime::parse,
            10 => parse_sint32,
            11 => parse_sint32,
            12 => DayOfWeek::parse,
            13 => parse_sint8,
            14 => parse_sint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum WeatherAlertMesg {
    Timestamp,
    ReportId,
    IssueTime,
    ExpireTime,
    Severity,
    Type,
}
impl WeatherAlertMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_string,
            1 => DateTime::parse,
            2 => DateTime::parse,
            3 => WeatherSeverity::parse,
            4 => WeatherSevereType::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum GpsMetadataMesg {
    Timestamp,
    TimestampMs,
    PositionLat,
    PositionLong,
    EnhancedAltitude,
    EnhancedSpeed,
    Heading,
    UtcTimestamp,
    Velocity,
}
impl GpsMetadataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_sint32,
            2 => parse_sint32,
            3 => parse_uint32,
            4 => parse_uint32,
            5 => parse_uint16,
            6 => DateTime::parse,
            7 => parse_sint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum CameraEventMesg {
    Timestamp,
    TimestampMs,
    CameraEventType,
    CameraFileUuid,
    CameraOrientation,
}
impl CameraEventMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => CameraEventType::parse,
            2 => parse_string,
            3 => CameraOrientationType::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum GyroscopeDataMesg {
    Timestamp,
    TimestampMs,
    SampleTimeOffset,
    GyroX,
    GyroY,
    GyroZ,
    CalibratedGyroX,
    CalibratedGyroY,
    CalibratedGyroZ,
}
impl GyroscopeDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_uint16,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => parse_float32,
            6 => parse_float32,
            7 => parse_float32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum AccelerometerDataMesg {
    Timestamp,
    TimestampMs,
    SampleTimeOffset,
    AccelX,
    AccelY,
    AccelZ,
    CalibratedAccelX,
    CalibratedAccelY,
    CalibratedAccelZ,
    CompressedCalibratedAccelX,
    CompressedCalibratedAccelY,
    CompressedCalibratedAccelZ,
}
impl AccelerometerDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_uint16,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => parse_float32,
            6 => parse_float32,
            7 => parse_float32,
            8 => parse_sint16,
            9 => parse_sint16,
            10 => parse_sint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum MagnetometerDataMesg {
    Timestamp,
    TimestampMs,
    SampleTimeOffset,
    MagX,
    MagY,
    MagZ,
    CalibratedMagX,
    CalibratedMagY,
    CalibratedMagZ,
}
impl MagnetometerDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_uint16,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => parse_float32,
            6 => parse_float32,
            7 => parse_float32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum BarometerDataMesg {
    Timestamp,
    TimestampMs,
    SampleTimeOffset,
    BaroPres,
}
impl BarometerDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ThreeDSensorCalibrationMesg {
    Timestamp,
    SensorType,
    CalibrationFactor,
    CalibrationDivisor,
    LevelShift,
    OffsetCal,
    OrientationMatrix,
}
impl ThreeDSensorCalibrationMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => SensorType::parse,
            1 => parse_uint32,
            2 => parse_uint32,
            3 => parse_uint32,
            4 => parse_sint32,
            5 => parse_sint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum OneDSensorCalibrationMesg {
    Timestamp,
    SensorType,
    CalibrationFactor,
    CalibrationDivisor,
    LevelShift,
    OffsetCal,
}
impl OneDSensorCalibrationMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => SensorType::parse,
            1 => parse_uint32,
            2 => parse_uint32,
            3 => parse_uint32,
            4 => parse_sint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum VideoFrameMesg {
    Timestamp,
    TimestampMs,
    FrameNumber,
}
impl VideoFrameMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ObdiiDataMesg {
    Timestamp,
    TimestampMs,
    TimeOffset,
    Pid,
    RawData,
    PidDataSize,
    SystemTime,
    StartTimestamp,
    StartTimestampMs,
}
impl ObdiiDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_byte,
            3 => parse_byte,
            4 => parse_uint8,
            5 => parse_uint32,
            6 => DateTime::parse,
            7 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum NmeaSentenceMesg {
    Timestamp,
    TimestampMs,
    Sentence,
}
impl NmeaSentenceMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum AviationAttitudeMesg {
    Timestamp,
    TimestampMs,
    SystemTime,
    Pitch,
    Roll,
    AccelLateral,
    AccelNormal,
    TurnRate,
    Stage,
    AttitudeStageComplete,
    Track,
    Validity,
}
impl AviationAttitudeMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint32,
            2 => parse_sint16,
            3 => parse_sint16,
            4 => parse_sint16,
            5 => parse_sint16,
            6 => parse_sint16,
            7 => AttitudeStage::parse,
            8 => parse_uint8,
            9 => parse_uint16,
            10 => AttitudeValidity::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum VideoMesg {
    Url,
    HostingProvider,
    Duration,
}
impl VideoMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_string,
            1 => parse_string,
            2 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum VideoTitleMesg {
    MessageIndex,
    MessageCount,
    Text,
}
impl VideoTitleMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_uint16,
            1 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum VideoDescriptionMesg {
    MessageIndex,
    MessageCount,
    Text,
}
impl VideoDescriptionMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_uint16,
            1 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum VideoClipMesg {
    ClipNumber,
    StartTimestamp,
    StartTimestampMs,
    EndTimestamp,
    EndTimestampMs,
    ClipStart,
    ClipEnd,
}
impl VideoClipMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint16,
            1 => DateTime::parse,
            2 => parse_uint16,
            3 => DateTime::parse,
            4 => parse_uint16,
            6 => parse_uint32,
            7 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SetMesg {
    Timestamp,
    Duration,
    Repetitions,
    Weight,
    SetType,
    StartTime,
    Category,
    CategorySubtype,
    WeightDisplayUnit,
    MessageIndex,
    WktStepIndex,
}
impl SetMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => DateTime::parse,
            0 => parse_uint32,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => SetType::parse,
            6 => DateTime::parse,
            7 => ExerciseCategory::parse,
            8 => parse_uint16,
            9 => FitBaseUnit::parse,
            10 => MessageIndex::parse,
            11 => MessageIndex::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum JumpMesg {
    Timestamp,
    Distance,
    Height,
    Rotations,
    HangTime,
    Score,
    PositionLat,
    PositionLong,
    Speed,
    EnhancedSpeed,
}
impl JumpMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_float32,
            1 => parse_float32,
            2 => parse_uint8,
            3 => parse_float32,
            4 => parse_float32,
            5 => parse_sint32,
            6 => parse_sint32,
            7 => parse_uint16,
            8 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SplitMesg {
    MessageIndex,
    SplitType,
    TotalElapsedTime,
    TotalTimerTime,
    TotalDistance,
    AvgSpeed,
    StartTime,
    TotalAscent,
    TotalDescent,
    StartPositionLat,
    StartPositionLong,
    EndPositionLat,
    EndPositionLong,
    MaxSpeed,
    AvgVertSpeed,
    EndTime,
    TotalCalories,
    StartElevation,
    TotalMovingTime,
}
impl SplitMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => SplitType::parse,
            1 => parse_uint32,
            2 => parse_uint32,
            3 => parse_uint32,
            4 => parse_uint32,
            9 => DateTime::parse,
            13 => parse_uint16,
            14 => parse_uint16,
            21 => parse_sint32,
            22 => parse_sint32,
            23 => parse_sint32,
            24 => parse_sint32,
            25 => parse_uint32,
            26 => parse_sint32,
            27 => DateTime::parse,
            28 => parse_uint32,
            74 => parse_uint32,
            110 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SplitSummaryMesg {
    MessageIndex,
    SplitType,
    NumSplits,
    TotalTimerTime,
    TotalDistance,
    AvgSpeed,
    MaxSpeed,
    TotalAscent,
    TotalDescent,
    AvgHeartRate,
    MaxHeartRate,
    AvgVertSpeed,
    TotalCalories,
    TotalMovingTime,
}
impl SplitSummaryMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => SplitType::parse,
            3 => parse_uint16,
            4 => parse_uint32,
            5 => parse_uint32,
            6 => parse_uint32,
            7 => parse_uint32,
            8 => parse_uint16,
            9 => parse_uint16,
            10 => parse_uint8,
            11 => parse_uint8,
            12 => parse_sint32,
            13 => parse_uint32,
            77 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ClimbProMesg {
    Timestamp,
    PositionLat,
    PositionLong,
    ClimbProEvent,
    ClimbNumber,
    ClimbCategory,
    CurrentDist,
}
impl ClimbProMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_sint32,
            1 => parse_sint32,
            2 => ClimbProEvent::parse,
            3 => parse_uint16,
            4 => parse_uint8,
            5 => parse_float32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum FieldDescriptionMesg {
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
}
impl FieldDescriptionMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint8,
            1 => parse_uint8,
            2 => FitBaseType::parse,
            3 => parse_string,
            4 => parse_uint8,
            5 => parse_string,
            6 => parse_uint8,
            7 => parse_sint8,
            8 => parse_string,
            9 => parse_string,
            10 => parse_string,
            13 => FitBaseUnit::parse,
            14 => MesgNum::parse,
            15 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum DeveloperDataIdMesg {
    DeveloperId,
    ApplicationId,
    ManufacturerId,
    DeveloperDataIndex,
    ApplicationVersion,
}
impl DeveloperDataIdMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_byte,
            1 => parse_byte,
            2 => Manufacturer::parse,
            3 => parse_uint8,
            4 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum CourseMesg {
    Sport,
    Name,
    Capabilities,
    SubSport,
}
impl CourseMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            4 => Sport::parse,
            5 => parse_string,
            6 => CourseCapabilities::parse,
            7 => SubSport::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum CoursePointMesg {
    MessageIndex,
    Timestamp,
    PositionLat,
    PositionLong,
    Distance,
    Type,
    Name,
    Favorite,
}
impl CoursePointMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            1 => DateTime::parse,
            2 => parse_sint32,
            3 => parse_sint32,
            4 => parse_uint32,
            5 => CoursePoint::parse,
            6 => parse_string,
            8 => parse_unknown,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SegmentIdMesg {
    Name,
    Uuid,
    Sport,
    Enabled,
    UserProfilePrimaryKey,
    DeviceId,
    DefaultRaceLeader,
    DeleteStatus,
    SelectionType,
}
impl SegmentIdMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_string,
            1 => parse_string,
            2 => Sport::parse,
            3 => parse_unknown,
            4 => parse_uint32,
            5 => parse_uint32,
            6 => parse_uint8,
            7 => SegmentDeleteStatus::parse,
            8 => SegmentSelectionType::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SegmentLeaderboardEntryMesg {
    MessageIndex,
    Name,
    Type,
    GroupPrimaryKey,
    ActivityId,
    SegmentTime,
    ActivityIdString,
}
impl SegmentLeaderboardEntryMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_string,
            1 => SegmentLeaderboardType::parse,
            2 => parse_uint32,
            3 => parse_uint32,
            4 => parse_uint32,
            5 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SegmentPointMesg {
    MessageIndex,
    PositionLat,
    PositionLong,
    Distance,
    Altitude,
    LeaderTime,
    EnhancedAltitude,
}
impl SegmentPointMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            1 => parse_sint32,
            2 => parse_sint32,
            3 => parse_uint32,
            4 => parse_uint16,
            5 => parse_uint32,
            6 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SegmentLapMesg {
    MessageIndex,
    Timestamp,
    Event,
    EventType,
    StartTime,
    StartPositionLat,
    StartPositionLong,
    EndPositionLat,
    EndPositionLong,
    TotalElapsedTime,
    TotalTimerTime,
    TotalDistance,
    TotalCycles,
    TotalCalories,
    TotalFatCalories,
    AvgSpeed,
    MaxSpeed,
    AvgHeartRate,
    MaxHeartRate,
    AvgCadence,
    MaxCadence,
    AvgPower,
    MaxPower,
    TotalAscent,
    TotalDescent,
    Sport,
    EventGroup,
    NecLat,
    NecLong,
    SwcLat,
    SwcLong,
    Name,
    NormalizedPower,
    LeftRightBalance,
    SubSport,
    TotalWork,
    AvgAltitude,
    MaxAltitude,
    GpsAccuracy,
    AvgGrade,
    AvgPosGrade,
    AvgNegGrade,
    MaxPosGrade,
    MaxNegGrade,
    AvgTemperature,
    MaxTemperature,
    TotalMovingTime,
    AvgPosVerticalSpeed,
    AvgNegVerticalSpeed,
    MaxPosVerticalSpeed,
    MaxNegVerticalSpeed,
    TimeInHrZone,
    TimeInSpeedZone,
    TimeInCadenceZone,
    TimeInPowerZone,
    RepetitionNum,
    MinAltitude,
    MinHeartRate,
    ActiveTime,
    WktStepIndex,
    SportEvent,
    AvgLeftTorqueEffectiveness,
    AvgRightTorqueEffectiveness,
    AvgLeftPedalSmoothness,
    AvgRightPedalSmoothness,
    AvgCombinedPedalSmoothness,
    Status,
    Uuid,
    AvgFractionalCadence,
    MaxFractionalCadence,
    TotalFractionalCycles,
    FrontGearShiftCount,
    RearGearShiftCount,
    TimeStanding,
    StandCount,
    AvgLeftPco,
    AvgRightPco,
    AvgLeftPowerPhase,
    AvgLeftPowerPhasePeak,
    AvgRightPowerPhase,
    AvgRightPowerPhasePeak,
    AvgPowerPosition,
    MaxPowerPosition,
    AvgCadencePosition,
    MaxCadencePosition,
    Manufacturer,
    TotalGrit,
    TotalFlow,
    AvgGrit,
    AvgFlow,
    TotalFractionalAscent,
    TotalFractionalDescent,
    EnhancedAvgAltitude,
    EnhancedMaxAltitude,
    EnhancedMinAltitude,
}
impl SegmentLapMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            253 => DateTime::parse,
            0 => Event::parse,
            1 => EventType::parse,
            2 => DateTime::parse,
            3 => parse_sint32,
            4 => parse_sint32,
            5 => parse_sint32,
            6 => parse_sint32,
            7 => parse_uint32,
            8 => parse_uint32,
            9 => parse_uint32,
            10 => parse_uint32,
            11 => parse_uint16,
            12 => parse_uint16,
            13 => parse_uint16,
            14 => parse_uint16,
            15 => parse_uint8,
            16 => parse_uint8,
            17 => parse_uint8,
            18 => parse_uint8,
            19 => parse_uint16,
            20 => parse_uint16,
            21 => parse_uint16,
            22 => parse_uint16,
            23 => Sport::parse,
            24 => parse_uint8,
            25 => parse_sint32,
            26 => parse_sint32,
            27 => parse_sint32,
            28 => parse_sint32,
            29 => parse_string,
            30 => parse_uint16,
            31 => LeftRightBalance100::parse,
            32 => SubSport::parse,
            33 => parse_uint32,
            34 => parse_uint16,
            35 => parse_uint16,
            36 => parse_uint8,
            37 => parse_sint16,
            38 => parse_sint16,
            39 => parse_sint16,
            40 => parse_sint16,
            41 => parse_sint16,
            42 => parse_sint8,
            43 => parse_sint8,
            44 => parse_uint32,
            45 => parse_sint16,
            46 => parse_sint16,
            47 => parse_sint16,
            48 => parse_sint16,
            49 => parse_uint32,
            50 => parse_uint32,
            51 => parse_uint32,
            52 => parse_uint32,
            53 => parse_uint16,
            54 => parse_uint16,
            55 => parse_uint8,
            56 => parse_uint32,
            57 => MessageIndex::parse,
            58 => SportEvent::parse,
            59 => parse_uint8,
            60 => parse_uint8,
            61 => parse_uint8,
            62 => parse_uint8,
            63 => parse_uint8,
            64 => SegmentLapStatus::parse,
            65 => parse_string,
            66 => parse_uint8,
            67 => parse_uint8,
            68 => parse_uint8,
            69 => parse_uint16,
            70 => parse_uint16,
            71 => parse_uint32,
            72 => parse_uint16,
            73 => parse_sint8,
            74 => parse_sint8,
            75 => parse_uint8,
            76 => parse_uint8,
            77 => parse_uint8,
            78 => parse_uint8,
            79 => parse_uint16,
            80 => parse_uint16,
            81 => parse_uint8,
            82 => parse_uint8,
            83 => Manufacturer::parse,
            84 => parse_float32,
            85 => parse_float32,
            86 => parse_float32,
            87 => parse_float32,
            89 => parse_uint8,
            90 => parse_uint8,
            91 => parse_uint32,
            92 => parse_uint32,
            93 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SegmentFileMesg {
    MessageIndex,
    FileUuid,
    Enabled,
    UserProfilePrimaryKey,
    LeaderType,
    LeaderGroupPrimaryKey,
    LeaderActivityId,
    LeaderActivityIdString,
    DefaultRaceLeader,
}
impl SegmentFileMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            1 => parse_string,
            3 => parse_unknown,
            4 => parse_uint32,
            7 => SegmentLeaderboardType::parse,
            8 => parse_uint32,
            9 => parse_uint32,
            10 => parse_string,
            11 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum WorkoutMesg {
    MessageIndex,
    Sport,
    Capabilities,
    NumValidSteps,
    WktName,
    SubSport,
    PoolLength,
    PoolLengthUnit,
    WktDescription,
}
impl WorkoutMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            4 => Sport::parse,
            5 => WorkoutCapabilities::parse,
            6 => parse_uint16,
            8 => parse_string,
            11 => SubSport::parse,
            14 => parse_uint16,
            15 => DisplayMeasure::parse,
            17 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum WorkoutSessionMesg {
    MessageIndex,
    Sport,
    SubSport,
    NumValidSteps,
    FirstStepIndex,
    PoolLength,
    PoolLengthUnit,
}
impl WorkoutSessionMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => Sport::parse,
            1 => SubSport::parse,
            2 => parse_uint16,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => DisplayMeasure::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum WorkoutStepMesg {
    MessageIndex,
    WktStepName,
    DurationType,
    DurationValue,
    TargetType,
    TargetValue,
    CustomTargetValueLow,
    CustomTargetValueHigh,
    Intensity,
    Notes,
    Equipment,
    ExerciseCategory,
    ExerciseName,
    ExerciseWeight,
    WeightDisplayUnit,
    SecondaryTargetType,
    SecondaryTargetValue,
    SecondaryCustomTargetValueLow,
    SecondaryCustomTargetValueHigh,
}
impl WorkoutStepMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => parse_string,
            1 => WktStepDuration::parse,
            2 => parse_uint32,
            3 => WktStepTarget::parse,
            4 => parse_uint32,
            5 => parse_uint32,
            6 => parse_uint32,
            7 => Intensity::parse,
            8 => parse_string,
            9 => WorkoutEquipment::parse,
            10 => ExerciseCategory::parse,
            11 => parse_uint16,
            12 => parse_uint16,
            13 => FitBaseUnit::parse,
            19 => WktStepTarget::parse,
            20 => parse_uint32,
            21 => parse_uint32,
            22 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ExerciseTitleMesg {
    MessageIndex,
    ExerciseCategory,
    ExerciseName,
    WktStepName,
}
impl ExerciseTitleMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            0 => ExerciseCategory::parse,
            1 => parse_uint16,
            2 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ScheduleMesg {
    Manufacturer,
    Product,
    SerialNumber,
    TimeCreated,
    Completed,
    Type,
    ScheduledTime,
}
impl ScheduleMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => Manufacturer::parse,
            1 => parse_uint16,
            2 => parse_unknown,
            3 => DateTime::parse,
            4 => parse_unknown,
            5 => Schedule::parse,
            6 => LocalDateTime::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum TotalsMesg {
    MessageIndex,
    Timestamp,
    TimerTime,
    Distance,
    Calories,
    Sport,
    ElapsedTime,
    Sessions,
    ActiveTime,
    SportIndex,
}
impl TotalsMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            254 => MessageIndex::parse,
            253 => DateTime::parse,
            0 => parse_uint32,
            1 => parse_uint32,
            2 => parse_uint32,
            3 => Sport::parse,
            4 => parse_uint32,
            5 => parse_uint16,
            6 => parse_uint32,
            9 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum WeightScaleMesg {
    Timestamp,
    Weight,
    PercentFat,
    PercentHydration,
    VisceralFatMass,
    BoneMass,
    MuscleMass,
    BasalMet,
    PhysiqueRating,
    ActiveMet,
    MetabolicAge,
    VisceralFatRating,
    UserProfileIndex,
    Bmi,
}
impl WeightScaleMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => Weight::parse,
            1 => parse_uint16,
            2 => parse_uint16,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => parse_uint16,
            7 => parse_uint16,
            8 => parse_uint8,
            9 => parse_uint16,
            10 => parse_uint8,
            11 => parse_uint8,
            12 => MessageIndex::parse,
            13 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum BloodPressureMesg {
    Timestamp,
    SystolicPressure,
    DiastolicPressure,
    MeanArterialPressure,
    Map3SampleMean,
    MapMorningValues,
    MapEveningValues,
    HeartRate,
    HeartRateType,
    Status,
    UserProfileIndex,
}
impl BloodPressureMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_uint16,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => parse_uint16,
            6 => parse_uint8,
            7 => HrType::parse,
            8 => BpStatus::parse,
            9 => MessageIndex::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum MonitoringInfoMesg {
    Timestamp,
    LocalTimestamp,
    ActivityType,
    CyclesToDistance,
    CyclesToCalories,
    RestingMetabolicRate,
}
impl MonitoringInfoMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => LocalDateTime::parse,
            1 => ActivityType::parse,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum MonitoringMesg {
    Timestamp,
    DeviceIndex,
    Calories,
    Distance,
    Cycles,
    ActiveTime,
    ActivityType,
    ActivitySubtype,
    ActivityLevel,
    Distance16,
    Cycles16,
    ActiveTime16,
    LocalTimestamp,
    Temperature,
    TemperatureMin,
    TemperatureMax,
    ActivityTime,
    ActiveCalories,
    CurrentActivityTypeIntensity,
    TimestampMin8,
    Timestamp16,
    HeartRate,
    Intensity,
    DurationMin,
    Duration,
    Ascent,
    Descent,
    ModerateActivityMinutes,
    VigorousActivityMinutes,
}
impl MonitoringMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => DeviceIndex::parse,
            1 => parse_uint16,
            2 => parse_uint32,
            3 => parse_uint32,
            4 => parse_uint32,
            5 => ActivityType::parse,
            6 => ActivitySubtype::parse,
            7 => ActivityLevel::parse,
            8 => parse_uint16,
            9 => parse_uint16,
            10 => parse_uint16,
            11 => LocalDateTime::parse,
            12 => parse_sint16,
            14 => parse_sint16,
            15 => parse_sint16,
            16 => parse_uint16,
            19 => parse_uint16,
            24 => parse_byte,
            25 => parse_uint8,
            26 => parse_uint16,
            27 => parse_uint8,
            28 => parse_uint8,
            29 => parse_uint16,
            30 => parse_uint32,
            31 => parse_uint32,
            32 => parse_uint32,
            33 => parse_uint16,
            34 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum MonitoringHrDataMesg {
    Timestamp,
    RestingHeartRate,
    CurrentDayRestingHeartRate,
}
impl MonitoringHrDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint8,
            1 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum Spo2DataMesg {
    Timestamp,
    ReadingSpo2,
    ReadingConfidence,
    Mode,
}
impl Spo2DataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint8,
            1 => parse_uint8,
            2 => Spo2MeasurementType::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HrMesg {
    Timestamp,
    FractionalTimestamp,
    Time256,
    FilteredBpm,
    EventTimestamp,
    EventTimestamp12,
}
impl HrMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint8,
            6 => parse_uint8,
            9 => parse_uint32,
            10 => parse_byte,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum StressLevelMesg {
    StressLevelValue,
    StressLevelTime,
}
impl StressLevelMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_sint16,
            1 => DateTime::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum MaxMetDataMesg {
    UpdateTime,
    Vo2Max,
    Sport,
    SubSport,
    MaxMetCategory,
    CalibratedData,
    HrSource,
    SpeedSource,
}
impl MaxMetDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => DateTime::parse,
            2 => parse_uint16,
            5 => Sport::parse,
            6 => SubSport::parse,
            8 => MaxMetCategory::parse,
            9 => parse_unknown,
            12 => MaxMetHeartRateSource::parse,
            13 => MaxMetSpeedSource::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaBodyBatteryDataMesg {
    Timestamp,
    ProcessingInterval,
    Level,
    Charged,
    Uncharged,
}
impl HsaBodyBatteryDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_sint8,
            2 => parse_sint16,
            3 => parse_sint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaEventMesg {
    Timestamp,
    EventId,
}
impl HsaEventMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaAccelerometerDataMesg {
    Timestamp,
    TimestampMs,
    SamplingInterval,
    AccelX,
    AccelY,
    AccelZ,
    Timestamp32k,
}
impl HsaAccelerometerDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_sint16,
            3 => parse_sint16,
            4 => parse_sint16,
            5 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaGyroscopeDataMesg {
    Timestamp,
    TimestampMs,
    SamplingInterval,
    GyroX,
    GyroY,
    GyroZ,
    Timestamp32k,
}
impl HsaGyroscopeDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_sint16,
            3 => parse_sint16,
            4 => parse_sint16,
            5 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaStepDataMesg {
    Timestamp,
    ProcessingInterval,
    Steps,
}
impl HsaStepDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaSpo2DataMesg {
    Timestamp,
    ProcessingInterval,
    ReadingSpo2,
    Confidence,
}
impl HsaSpo2DataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint8,
            2 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaStressDataMesg {
    Timestamp,
    ProcessingInterval,
    StressLevel,
}
impl HsaStressDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_sint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaRespirationDataMesg {
    Timestamp,
    ProcessingInterval,
    RespirationRate,
}
impl HsaRespirationDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_sint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaHeartRateDataMesg {
    Timestamp,
    ProcessingInterval,
    Status,
    HeartRate,
}
impl HsaHeartRateDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint8,
            2 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaConfigurationDataMesg {
    Timestamp,
    Data,
    DataSize,
}
impl HsaConfigurationDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_byte,
            1 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HsaWristTemperatureDataMesg {
    Timestamp,
    ProcessingInterval,
    Value,
}
impl HsaWristTemperatureDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum MemoGlobMesg {
    PartIndex,
    Memo,
    MesgNum,
    ParentIndex,
    FieldNum,
    Data,
}
impl MemoGlobMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            250 => parse_uint32,
            0 => parse_byte,
            1 => MesgNum::parse,
            2 => MessageIndex::parse,
            3 => parse_uint8,
            4 => parse_unknown,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SleepLevelMesg {
    Timestamp,
    SleepLevel,
}
impl SleepLevelMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => SleepLevel::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum AntChannelIdMesg {
    ChannelNumber,
    DeviceType,
    DeviceNumber,
    TransmissionType,
    DeviceIndex,
}
impl AntChannelIdMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint8,
            1 => parse_unknown,
            2 => parse_unknown,
            3 => parse_unknown,
            4 => DeviceIndex::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum AntRxMesg {
    Timestamp,
    FractionalTimestamp,
    MesgId,
    MesgData,
    ChannelNumber,
    Data,
}
impl AntRxMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_byte,
            2 => parse_byte,
            3 => parse_uint8,
            4 => parse_byte,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum AntTxMesg {
    Timestamp,
    FractionalTimestamp,
    MesgId,
    MesgData,
    ChannelNumber,
    Data,
}
impl AntTxMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_byte,
            2 => parse_byte,
            3 => parse_uint8,
            4 => parse_byte,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ExdScreenConfigurationMesg {
    ScreenIndex,
    FieldCount,
    Layout,
    ScreenEnabled,
}
impl ExdScreenConfigurationMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint8,
            1 => parse_uint8,
            2 => ExdLayout::parse,
            3 => parse_unknown,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ExdDataFieldConfigurationMesg {
    ScreenIndex,
    ConceptField,
    FieldId,
    ConceptCount,
    DisplayType,
    Title,
}
impl ExdDataFieldConfigurationMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint8,
            1 => parse_byte,
            2 => parse_uint8,
            3 => parse_uint8,
            4 => ExdDisplayType::parse,
            5 => parse_string,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ExdDataConceptConfigurationMesg {
    ScreenIndex,
    ConceptField,
    FieldId,
    ConceptIndex,
    DataPage,
    ConceptKey,
    Scaling,
    DataUnits,
    Qualifier,
    Descriptor,
    IsSigned,
}
impl ExdDataConceptConfigurationMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint8,
            1 => parse_byte,
            2 => parse_uint8,
            3 => parse_uint8,
            4 => parse_uint8,
            5 => parse_uint8,
            6 => parse_uint8,
            8 => ExdDataUnits::parse,
            9 => ExdQualifiers::parse,
            10 => ExdDescriptors::parse,
            11 => parse_unknown,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum DiveSummaryMesg {
    Timestamp,
    ReferenceMesg,
    ReferenceIndex,
    AvgDepth,
    MaxDepth,
    SurfaceInterval,
    StartCns,
    EndCns,
    StartN2,
    EndN2,
    O2Toxicity,
    DiveNumber,
    BottomTime,
    AvgPressureSac,
    AvgVolumeSac,
    AvgRmv,
    DescentTime,
    AscentTime,
    AvgAscentRate,
    AvgDescentRate,
    MaxAscentRate,
    MaxDescentRate,
    HangTime,
}
impl DiveSummaryMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => MesgNum::parse,
            1 => MessageIndex::parse,
            2 => parse_uint32,
            3 => parse_uint32,
            4 => parse_uint32,
            5 => parse_uint8,
            6 => parse_uint8,
            7 => parse_uint16,
            8 => parse_uint16,
            9 => parse_uint16,
            10 => parse_uint32,
            11 => parse_uint32,
            12 => parse_uint16,
            13 => parse_uint16,
            14 => parse_uint16,
            15 => parse_uint32,
            16 => parse_uint32,
            17 => parse_sint32,
            22 => parse_uint32,
            23 => parse_uint32,
            24 => parse_uint32,
            25 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum AadAccelFeaturesMesg {
    Timestamp,
    Time,
    EnergyTotal,
    ZeroCrossCnt,
    Instance,
    TimeAboveThreshold,
}
impl AadAccelFeaturesMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint32,
            2 => parse_uint16,
            3 => parse_uint8,
            4 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HrvMesg {
    Time,
}
impl HrvMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum BeatIntervalsMesg {
    Timestamp,
    TimestampMs,
    Time,
}
impl BeatIntervalsMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HrvStatusSummaryMesg {
    Timestamp,
    WeeklyAverage,
    LastNightAverage,
    LastNight5MinHigh,
    BaselineLowUpper,
    BaselineBalancedLower,
    BaselineBalancedUpper,
    Status,
}
impl HrvStatusSummaryMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_uint16,
            3 => parse_uint16,
            4 => parse_uint16,
            5 => parse_uint16,
            6 => HrvStatus::parse,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum HrvValueMesg {
    Timestamp,
    Value,
}
impl HrvValueMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum RawBbiMesg {
    Timestamp,
    TimestampMs,
    Data,
    Time,
    Quality,
    Gap,
}
impl RawBbiMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint16,
            1 => parse_uint16,
            2 => parse_uint16,
            3 => parse_uint8,
            4 => parse_uint8,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum RespirationRateMesg {
    Timestamp,
    RespirationRate,
}
impl RespirationRateMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_sint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ChronoShotSessionMesg {
    Timestamp,
    MinSpeed,
    MaxSpeed,
    AvgSpeed,
    ShotCount,
    ProjectileType,
    GrainWeight,
    StandardDeviation,
}
impl ChronoShotSessionMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint32,
            1 => parse_uint32,
            2 => parse_uint32,
            3 => parse_uint16,
            4 => ProjectileType::parse,
            5 => parse_uint32,
            6 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ChronoShotDataMesg {
    Timestamp,
    ShotSpeed,
    ShotNum,
}
impl ChronoShotDataMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => parse_uint32,
            1 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum TankUpdateMesg {
    Timestamp,
    Sensor,
    Pressure,
}
impl TankUpdateMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => AntChannelId::parse,
            1 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum TankSummaryMesg {
    Timestamp,
    Sensor,
    StartPressure,
    EndPressure,
    VolumeUsed,
}
impl TankSummaryMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => AntChannelId::parse,
            1 => parse_uint16,
            2 => parse_uint16,
            3 => parse_uint32,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SleepAssessmentMesg {
    CombinedAwakeScore,
    AwakeTimeScore,
    AwakeningsCountScore,
    DeepSleepScore,
    SleepDurationScore,
    LightSleepScore,
    OverallSleepScore,
    SleepQualityScore,
    SleepRecoveryScore,
    RemSleepScore,
    SleepRestlessnessScore,
    AwakeningsCount,
    InterruptionsScore,
    AverageStressDuringSleep,
}
impl SleepAssessmentMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            0 => parse_uint8,
            1 => parse_uint8,
            2 => parse_uint8,
            3 => parse_uint8,
            4 => parse_uint8,
            5 => parse_uint8,
            6 => parse_uint8,
            7 => parse_uint8,
            8 => parse_uint8,
            9 => parse_uint8,
            10 => parse_uint8,
            11 => parse_uint8,
            14 => parse_uint8,
            15 => parse_uint16,
            _ => parse_uint8,
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SkinTempOvernightMesg {
    Timestamp,
    LocalTimestamp,
    AverageDeviation,
    Average7DayDeviation,
    NightlyValue,
}
impl SkinTempOvernightMesg {
    fn get_parse_function(
        def_number: u8,
    ) -> fn(&mut Reader, &Endianness, u8) -> Result<Vec<DataValue>, DataTypeError> {
        match def_number {
            253 => DateTime::parse,
            0 => LocalDateTime::parse,
            1 => parse_float32,
            2 => parse_float32,
            4 => parse_float32,
            _ => parse_uint8,
        }
    }
}
