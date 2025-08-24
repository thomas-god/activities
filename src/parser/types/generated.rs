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
    Device,
    Settings,
    Sport,
    Activity,
    Workout,
    Course,
    Schedules,
    Weight,
    Totals,
    Goals,
    BloodPressure,
    MonitoringA,
    ActivitySummary,
    MonitoringDaily,
    MonitoringB,
    Segment,
    SegmentList,
    ExdConfiguration,
    MfgRangeMin,
    MfgRangeMax,
    UnknownVariant,
}
impl File {
    pub fn from(content: u8) -> File {
        match content {
            1 => File::Device,
            2 => File::Settings,
            3 => File::Sport,
            4 => File::Activity,
            5 => File::Workout,
            6 => File::Course,
            7 => File::Schedules,
            9 => File::Weight,
            10 => File::Totals,
            11 => File::Goals,
            14 => File::BloodPressure,
            15 => File::MonitoringA,
            20 => File::ActivitySummary,
            28 => File::MonitoringDaily,
            32 => File::MonitoringB,
            34 => File::Segment,
            35 => File::SegmentList,
            40 => File::ExdConfiguration,
            247 => File::MfgRangeMin,
            254 => File::MfgRangeMax,
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
    FileId,
    Capabilities,
    DeviceSettings,
    UserProfile,
    HrmProfile,
    SdmProfile,
    BikeProfile,
    ZonesTarget,
    HrZone,
    PowerZone,
    MetZone,
    Sport,
    TrainingSettings,
    Goal,
    Session,
    Lap,
    Record,
    Event,
    DeviceInfo,
    Workout,
    WorkoutStep,
    Schedule,
    WeightScale,
    Course,
    CoursePoint,
    Totals,
    Activity,
    Software,
    FileCapabilities,
    MesgCapabilities,
    FieldCapabilities,
    FileCreator,
    BloodPressure,
    SpeedZone,
    Monitoring,
    TrainingFile,
    Hrv,
    AntRx,
    AntTx,
    AntChannelId,
    Length,
    MonitoringInfo,
    Pad,
    SlaveDevice,
    Connectivity,
    WeatherConditions,
    WeatherAlert,
    CadenceZone,
    Hr,
    SegmentLap,
    MemoGlob,
    SegmentId,
    SegmentLeaderboardEntry,
    SegmentPoint,
    SegmentFile,
    WorkoutSession,
    WatchfaceSettings,
    GpsMetadata,
    CameraEvent,
    TimestampCorrelation,
    GyroscopeData,
    AccelerometerData,
    ThreeDSensorCalibration,
    VideoFrame,
    ObdiiData,
    NmeaSentence,
    AviationAttitude,
    Video,
    VideoTitle,
    VideoDescription,
    VideoClip,
    OhrSettings,
    ExdScreenConfiguration,
    ExdDataFieldConfiguration,
    ExdDataConceptConfiguration,
    FieldDescription,
    DeveloperDataId,
    MagnetometerData,
    BarometerData,
    OneDSensorCalibration,
    MonitoringHrData,
    TimeInZone,
    Set,
    StressLevel,
    MaxMetData,
    DiveSettings,
    DiveGas,
    DiveAlarm,
    ExerciseTitle,
    DiveSummary,
    Spo2Data,
    SleepLevel,
    Jump,
    AadAccelFeatures,
    BeatIntervals,
    RespirationRate,
    HsaAccelerometerData,
    HsaStepData,
    HsaSpo2Data,
    HsaStressData,
    HsaRespirationData,
    HsaHeartRateData,
    Split,
    SplitSummary,
    HsaBodyBatteryData,
    HsaEvent,
    ClimbPro,
    TankUpdate,
    TankSummary,
    SleepAssessment,
    HrvStatusSummary,
    HrvValue,
    RawBbi,
    DeviceAuxBatteryInfo,
    HsaGyroscopeData,
    ChronoShotSession,
    ChronoShotData,
    HsaConfigurationData,
    DiveApneaAlarm,
    SkinTempOvernight,
    HsaWristTemperatureData,
    MfgRangeMin,
    MfgRangeMax,
    UnknownVariant,
}
impl MesgNum {
    pub fn from(content: u16) -> MesgNum {
        match content {
            0 => MesgNum::FileId,
            1 => MesgNum::Capabilities,
            2 => MesgNum::DeviceSettings,
            3 => MesgNum::UserProfile,
            4 => MesgNum::HrmProfile,
            5 => MesgNum::SdmProfile,
            6 => MesgNum::BikeProfile,
            7 => MesgNum::ZonesTarget,
            8 => MesgNum::HrZone,
            9 => MesgNum::PowerZone,
            10 => MesgNum::MetZone,
            12 => MesgNum::Sport,
            13 => MesgNum::TrainingSettings,
            15 => MesgNum::Goal,
            18 => MesgNum::Session,
            19 => MesgNum::Lap,
            20 => MesgNum::Record,
            21 => MesgNum::Event,
            23 => MesgNum::DeviceInfo,
            26 => MesgNum::Workout,
            27 => MesgNum::WorkoutStep,
            28 => MesgNum::Schedule,
            30 => MesgNum::WeightScale,
            31 => MesgNum::Course,
            32 => MesgNum::CoursePoint,
            33 => MesgNum::Totals,
            34 => MesgNum::Activity,
            35 => MesgNum::Software,
            37 => MesgNum::FileCapabilities,
            38 => MesgNum::MesgCapabilities,
            39 => MesgNum::FieldCapabilities,
            49 => MesgNum::FileCreator,
            51 => MesgNum::BloodPressure,
            53 => MesgNum::SpeedZone,
            55 => MesgNum::Monitoring,
            72 => MesgNum::TrainingFile,
            78 => MesgNum::Hrv,
            80 => MesgNum::AntRx,
            81 => MesgNum::AntTx,
            82 => MesgNum::AntChannelId,
            101 => MesgNum::Length,
            103 => MesgNum::MonitoringInfo,
            105 => MesgNum::Pad,
            106 => MesgNum::SlaveDevice,
            127 => MesgNum::Connectivity,
            128 => MesgNum::WeatherConditions,
            129 => MesgNum::WeatherAlert,
            131 => MesgNum::CadenceZone,
            132 => MesgNum::Hr,
            142 => MesgNum::SegmentLap,
            145 => MesgNum::MemoGlob,
            148 => MesgNum::SegmentId,
            149 => MesgNum::SegmentLeaderboardEntry,
            150 => MesgNum::SegmentPoint,
            151 => MesgNum::SegmentFile,
            158 => MesgNum::WorkoutSession,
            159 => MesgNum::WatchfaceSettings,
            160 => MesgNum::GpsMetadata,
            161 => MesgNum::CameraEvent,
            162 => MesgNum::TimestampCorrelation,
            164 => MesgNum::GyroscopeData,
            165 => MesgNum::AccelerometerData,
            167 => MesgNum::ThreeDSensorCalibration,
            169 => MesgNum::VideoFrame,
            174 => MesgNum::ObdiiData,
            177 => MesgNum::NmeaSentence,
            178 => MesgNum::AviationAttitude,
            184 => MesgNum::Video,
            185 => MesgNum::VideoTitle,
            186 => MesgNum::VideoDescription,
            187 => MesgNum::VideoClip,
            188 => MesgNum::OhrSettings,
            200 => MesgNum::ExdScreenConfiguration,
            201 => MesgNum::ExdDataFieldConfiguration,
            202 => MesgNum::ExdDataConceptConfiguration,
            206 => MesgNum::FieldDescription,
            207 => MesgNum::DeveloperDataId,
            208 => MesgNum::MagnetometerData,
            209 => MesgNum::BarometerData,
            210 => MesgNum::OneDSensorCalibration,
            211 => MesgNum::MonitoringHrData,
            216 => MesgNum::TimeInZone,
            225 => MesgNum::Set,
            227 => MesgNum::StressLevel,
            229 => MesgNum::MaxMetData,
            258 => MesgNum::DiveSettings,
            259 => MesgNum::DiveGas,
            262 => MesgNum::DiveAlarm,
            264 => MesgNum::ExerciseTitle,
            268 => MesgNum::DiveSummary,
            269 => MesgNum::Spo2Data,
            275 => MesgNum::SleepLevel,
            285 => MesgNum::Jump,
            289 => MesgNum::AadAccelFeatures,
            290 => MesgNum::BeatIntervals,
            297 => MesgNum::RespirationRate,
            302 => MesgNum::HsaAccelerometerData,
            304 => MesgNum::HsaStepData,
            305 => MesgNum::HsaSpo2Data,
            306 => MesgNum::HsaStressData,
            307 => MesgNum::HsaRespirationData,
            308 => MesgNum::HsaHeartRateData,
            312 => MesgNum::Split,
            313 => MesgNum::SplitSummary,
            314 => MesgNum::HsaBodyBatteryData,
            315 => MesgNum::HsaEvent,
            317 => MesgNum::ClimbPro,
            319 => MesgNum::TankUpdate,
            323 => MesgNum::TankSummary,
            346 => MesgNum::SleepAssessment,
            370 => MesgNum::HrvStatusSummary,
            371 => MesgNum::HrvValue,
            372 => MesgNum::RawBbi,
            375 => MesgNum::DeviceAuxBatteryInfo,
            376 => MesgNum::HsaGyroscopeData,
            387 => MesgNum::ChronoShotSession,
            388 => MesgNum::ChronoShotData,
            389 => MesgNum::HsaConfigurationData,
            393 => MesgNum::DiveApneaAlarm,
            398 => MesgNum::SkinTempOvernight,
            409 => MesgNum::HsaWristTemperatureData,
            65280 => MesgNum::MfgRangeMin,
            65534 => MesgNum::MfgRangeMax,
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
    Read,
    Write,
    Erase,
    UnknownVariant,
}
impl FileFlags {
    pub fn from(content: u8) -> FileFlags {
        match content {
            2 => FileFlags::Read,
            4 => FileFlags::Write,
            8 => FileFlags::Erase,
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
    NumPerFile,
    MaxPerFile,
    MaxPerFileType,
    UnknownVariant,
}
impl MesgCount {
    pub fn from(content: u8) -> MesgCount {
        match content {
            0 => MesgCount::NumPerFile,
            1 => MesgCount::MaxPerFile,
            2 => MesgCount::MaxPerFileType,
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
    Female,
    Male,
    UnknownVariant,
}
impl Gender {
    pub fn from(content: u8) -> Gender {
        match content {
            0 => Gender::Female,
            1 => Gender::Male,
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
    English,
    French,
    Italian,
    German,
    Spanish,
    Croatian,
    Czech,
    Danish,
    Dutch,
    Finnish,
    Greek,
    Hungarian,
    Norwegian,
    Polish,
    Portuguese,
    Slovakian,
    Slovenian,
    Swedish,
    Russian,
    Turkish,
    Latvian,
    Ukrainian,
    Arabic,
    Farsi,
    Bulgarian,
    Romanian,
    Chinese,
    Japanese,
    Korean,
    Taiwanese,
    Thai,
    Hebrew,
    BrazilianPortuguese,
    Indonesian,
    Malaysian,
    Vietnamese,
    Burmese,
    Mongolian,
    Custom,
    UnknownVariant,
}
impl Language {
    pub fn from(content: u8) -> Language {
        match content {
            0 => Language::English,
            1 => Language::French,
            2 => Language::Italian,
            3 => Language::German,
            4 => Language::Spanish,
            5 => Language::Croatian,
            6 => Language::Czech,
            7 => Language::Danish,
            8 => Language::Dutch,
            9 => Language::Finnish,
            10 => Language::Greek,
            11 => Language::Hungarian,
            12 => Language::Norwegian,
            13 => Language::Polish,
            14 => Language::Portuguese,
            15 => Language::Slovakian,
            16 => Language::Slovenian,
            17 => Language::Swedish,
            18 => Language::Russian,
            19 => Language::Turkish,
            20 => Language::Latvian,
            21 => Language::Ukrainian,
            22 => Language::Arabic,
            23 => Language::Farsi,
            24 => Language::Bulgarian,
            25 => Language::Romanian,
            26 => Language::Chinese,
            27 => Language::Japanese,
            28 => Language::Korean,
            29 => Language::Taiwanese,
            30 => Language::Thai,
            31 => Language::Hebrew,
            32 => Language::BrazilianPortuguese,
            33 => Language::Indonesian,
            34 => Language::Malaysian,
            35 => Language::Vietnamese,
            36 => Language::Burmese,
            37 => Language::Mongolian,
            254 => Language::Custom,
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
    Metric,
    Statute,
    Nautical,
    UnknownVariant,
}
impl DisplayMeasure {
    pub fn from(content: u8) -> DisplayMeasure {
        match content {
            0 => DisplayMeasure::Metric,
            1 => DisplayMeasure::Statute,
            2 => DisplayMeasure::Nautical,
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
    Bpm,
    Max,
    Reserve,
    UnknownVariant,
}
impl DisplayHeart {
    pub fn from(content: u8) -> DisplayHeart {
        match content {
            0 => DisplayHeart::Bpm,
            1 => DisplayHeart::Max,
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
    Degree,
    DegreeMinute,
    DegreeMinuteSecond,
    AustrianGrid,
    BritishGrid,
    DutchGrid,
    HungarianGrid,
    FinnishGrid,
    GermanGrid,
    IcelandicGrid,
    IndonesianEquatorial,
    IndonesianIrian,
    IndonesianSouthern,
    IndiaZone0,
    IndiaZoneIA,
    IndiaZoneIB,
    IndiaZoneIIA,
    IndiaZoneIIB,
    IndiaZoneIIIA,
    IndiaZoneIIIB,
    IndiaZoneIVA,
    IndiaZoneIVB,
    IrishTransverse,
    IrishGrid,
    Loran,
    MaidenheadGrid,
    MgrsGrid,
    NewZealandGrid,
    NewZealandTransverse,
    QatarGrid,
    ModifiedSwedishGrid,
    SwedishGrid,
    SouthAfricanGrid,
    SwissGrid,
    TaiwanGrid,
    UnitedStatesGrid,
    UtmUpsGrid,
    WestMalayan,
    BorneoRso,
    EstonianGrid,
    LatvianGrid,
    SwedishRef99Grid,
    UnknownVariant,
}
impl DisplayPosition {
    pub fn from(content: u8) -> DisplayPosition {
        match content {
            0 => DisplayPosition::Degree,
            1 => DisplayPosition::DegreeMinute,
            2 => DisplayPosition::DegreeMinuteSecond,
            3 => DisplayPosition::AustrianGrid,
            4 => DisplayPosition::BritishGrid,
            5 => DisplayPosition::DutchGrid,
            6 => DisplayPosition::HungarianGrid,
            7 => DisplayPosition::FinnishGrid,
            8 => DisplayPosition::GermanGrid,
            9 => DisplayPosition::IcelandicGrid,
            10 => DisplayPosition::IndonesianEquatorial,
            11 => DisplayPosition::IndonesianIrian,
            12 => DisplayPosition::IndonesianSouthern,
            13 => DisplayPosition::IndiaZone0,
            14 => DisplayPosition::IndiaZoneIA,
            15 => DisplayPosition::IndiaZoneIB,
            16 => DisplayPosition::IndiaZoneIIA,
            17 => DisplayPosition::IndiaZoneIIB,
            18 => DisplayPosition::IndiaZoneIIIA,
            19 => DisplayPosition::IndiaZoneIIIB,
            20 => DisplayPosition::IndiaZoneIVA,
            21 => DisplayPosition::IndiaZoneIVB,
            22 => DisplayPosition::IrishTransverse,
            23 => DisplayPosition::IrishGrid,
            24 => DisplayPosition::Loran,
            25 => DisplayPosition::MaidenheadGrid,
            26 => DisplayPosition::MgrsGrid,
            27 => DisplayPosition::NewZealandGrid,
            28 => DisplayPosition::NewZealandTransverse,
            29 => DisplayPosition::QatarGrid,
            30 => DisplayPosition::ModifiedSwedishGrid,
            31 => DisplayPosition::SwedishGrid,
            32 => DisplayPosition::SouthAfricanGrid,
            33 => DisplayPosition::SwissGrid,
            34 => DisplayPosition::TaiwanGrid,
            35 => DisplayPosition::UnitedStatesGrid,
            36 => DisplayPosition::UtmUpsGrid,
            37 => DisplayPosition::WestMalayan,
            38 => DisplayPosition::BorneoRso,
            39 => DisplayPosition::EstonianGrid,
            40 => DisplayPosition::LatvianGrid,
            41 => DisplayPosition::SwedishRef99Grid,
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
    Off,
    On,
    Auto,
    UnknownVariant,
}
impl Switch {
    pub fn from(content: u8) -> Switch {
        match content {
            0 => Switch::Off,
            1 => Switch::On,
            2 => Switch::Auto,
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
    Generic,
    Running,
    Cycling,
    Transition,
    FitnessEquipment,
    Swimming,
    Basketball,
    Soccer,
    Tennis,
    AmericanFootball,
    Training,
    Walking,
    CrossCountrySkiing,
    AlpineSkiing,
    Snowboarding,
    Rowing,
    Mountaineering,
    Hiking,
    Multisport,
    Paddling,
    Flying,
    EBiking,
    Motorcycling,
    Boating,
    Driving,
    Golf,
    HangGliding,
    HorsebackRiding,
    Hunting,
    Fishing,
    InlineSkating,
    RockClimbing,
    Sailing,
    IceSkating,
    SkyDiving,
    Snowshoeing,
    Snowmobiling,
    StandUpPaddleboarding,
    Surfing,
    Wakeboarding,
    WaterSkiing,
    Kayaking,
    Rafting,
    Windsurfing,
    Kitesurfing,
    Tactical,
    Jumpmaster,
    Boxing,
    FloorClimbing,
    Baseball,
    Diving,
    Hiit,
    Racket,
    WheelchairPushWalk,
    WheelchairPushRun,
    Meditation,
    DiscGolf,
    Cricket,
    Rugby,
    Hockey,
    Lacrosse,
    Volleyball,
    WaterTubing,
    Wakesurfing,
    MixedMartialArts,
    Snorkeling,
    Dance,
    JumpRope,
    All,
    UnknownVariant,
}
impl Sport {
    pub fn from(content: u8) -> Sport {
        match content {
            0 => Sport::Generic,
            1 => Sport::Running,
            2 => Sport::Cycling,
            3 => Sport::Transition,
            4 => Sport::FitnessEquipment,
            5 => Sport::Swimming,
            6 => Sport::Basketball,
            7 => Sport::Soccer,
            8 => Sport::Tennis,
            9 => Sport::AmericanFootball,
            10 => Sport::Training,
            11 => Sport::Walking,
            12 => Sport::CrossCountrySkiing,
            13 => Sport::AlpineSkiing,
            14 => Sport::Snowboarding,
            15 => Sport::Rowing,
            16 => Sport::Mountaineering,
            17 => Sport::Hiking,
            18 => Sport::Multisport,
            19 => Sport::Paddling,
            20 => Sport::Flying,
            21 => Sport::EBiking,
            22 => Sport::Motorcycling,
            23 => Sport::Boating,
            24 => Sport::Driving,
            25 => Sport::Golf,
            26 => Sport::HangGliding,
            27 => Sport::HorsebackRiding,
            28 => Sport::Hunting,
            29 => Sport::Fishing,
            30 => Sport::InlineSkating,
            31 => Sport::RockClimbing,
            32 => Sport::Sailing,
            33 => Sport::IceSkating,
            34 => Sport::SkyDiving,
            35 => Sport::Snowshoeing,
            36 => Sport::Snowmobiling,
            37 => Sport::StandUpPaddleboarding,
            38 => Sport::Surfing,
            39 => Sport::Wakeboarding,
            40 => Sport::WaterSkiing,
            41 => Sport::Kayaking,
            42 => Sport::Rafting,
            43 => Sport::Windsurfing,
            44 => Sport::Kitesurfing,
            45 => Sport::Tactical,
            46 => Sport::Jumpmaster,
            47 => Sport::Boxing,
            48 => Sport::FloorClimbing,
            49 => Sport::Baseball,
            53 => Sport::Diving,
            62 => Sport::Hiit,
            64 => Sport::Racket,
            65 => Sport::WheelchairPushWalk,
            66 => Sport::WheelchairPushRun,
            67 => Sport::Meditation,
            69 => Sport::DiscGolf,
            71 => Sport::Cricket,
            72 => Sport::Rugby,
            73 => Sport::Hockey,
            74 => Sport::Lacrosse,
            75 => Sport::Volleyball,
            76 => Sport::WaterTubing,
            77 => Sport::Wakesurfing,
            80 => Sport::MixedMartialArts,
            82 => Sport::Snorkeling,
            83 => Sport::Dance,
            84 => Sport::JumpRope,
            254 => Sport::All,
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
    Generic,
    Running,
    Cycling,
    Transition,
    FitnessEquipment,
    Swimming,
    Basketball,
    Soccer,
    UnknownVariant,
}
impl SportBits0 {
    pub fn from(content: u8) -> SportBits0 {
        match content {
            1 => SportBits0::Generic,
            2 => SportBits0::Running,
            4 => SportBits0::Cycling,
            8 => SportBits0::Transition,
            16 => SportBits0::FitnessEquipment,
            32 => SportBits0::Swimming,
            64 => SportBits0::Basketball,
            128 => SportBits0::Soccer,
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
    Generic,
    Treadmill,
    Street,
    Trail,
    Track,
    Spin,
    IndoorCycling,
    Road,
    Mountain,
    Downhill,
    Recumbent,
    Cyclocross,
    HandCycling,
    TrackCycling,
    IndoorRowing,
    Elliptical,
    StairClimbing,
    LapSwimming,
    OpenWater,
    FlexibilityTraining,
    StrengthTraining,
    WarmUp,
    Match,
    Exercise,
    Challenge,
    IndoorSkiing,
    CardioTraining,
    IndoorWalking,
    EBikeFitness,
    Bmx,
    CasualWalking,
    SpeedWalking,
    BikeToRunTransition,
    RunToBikeTransition,
    SwimToBikeTransition,
    Atv,
    Motocross,
    Backcountry,
    Resort,
    RcDrone,
    Wingsuit,
    Whitewater,
    SkateSkiing,
    Yoga,
    Pilates,
    IndoorRunning,
    GravelCycling,
    EBikeMountain,
    Commuting,
    MixedSurface,
    Navigate,
    TrackMe,
    Map,
    SingleGasDiving,
    MultiGasDiving,
    GaugeDiving,
    ApneaDiving,
    ApneaHunting,
    VirtualActivity,
    Obstacle,
    Breathing,
    SailRace,
    Ultra,
    IndoorClimbing,
    Bouldering,
    Hiit,
    Amrap,
    Emom,
    Tabata,
    Pickleball,
    Padel,
    IndoorWheelchairWalk,
    IndoorWheelchairRun,
    IndoorHandCycling,
    Squash,
    Badminton,
    Racquetball,
    TableTennis,
    FlyCanopy,
    FlyParaglide,
    FlyParamotor,
    FlyPressurized,
    FlyNavigate,
    FlyTimer,
    FlyAltimeter,
    FlyWx,
    FlyVfr,
    FlyIfr,
    All,
    UnknownVariant,
}
impl SubSport {
    pub fn from(content: u8) -> SubSport {
        match content {
            0 => SubSport::Generic,
            1 => SubSport::Treadmill,
            2 => SubSport::Street,
            3 => SubSport::Trail,
            4 => SubSport::Track,
            5 => SubSport::Spin,
            6 => SubSport::IndoorCycling,
            7 => SubSport::Road,
            8 => SubSport::Mountain,
            9 => SubSport::Downhill,
            10 => SubSport::Recumbent,
            11 => SubSport::Cyclocross,
            12 => SubSport::HandCycling,
            13 => SubSport::TrackCycling,
            14 => SubSport::IndoorRowing,
            15 => SubSport::Elliptical,
            16 => SubSport::StairClimbing,
            17 => SubSport::LapSwimming,
            18 => SubSport::OpenWater,
            19 => SubSport::FlexibilityTraining,
            20 => SubSport::StrengthTraining,
            21 => SubSport::WarmUp,
            22 => SubSport::Match,
            23 => SubSport::Exercise,
            24 => SubSport::Challenge,
            25 => SubSport::IndoorSkiing,
            26 => SubSport::CardioTraining,
            27 => SubSport::IndoorWalking,
            28 => SubSport::EBikeFitness,
            29 => SubSport::Bmx,
            30 => SubSport::CasualWalking,
            31 => SubSport::SpeedWalking,
            32 => SubSport::BikeToRunTransition,
            33 => SubSport::RunToBikeTransition,
            34 => SubSport::SwimToBikeTransition,
            35 => SubSport::Atv,
            36 => SubSport::Motocross,
            37 => SubSport::Backcountry,
            38 => SubSport::Resort,
            39 => SubSport::RcDrone,
            40 => SubSport::Wingsuit,
            41 => SubSport::Whitewater,
            42 => SubSport::SkateSkiing,
            43 => SubSport::Yoga,
            44 => SubSport::Pilates,
            45 => SubSport::IndoorRunning,
            46 => SubSport::GravelCycling,
            47 => SubSport::EBikeMountain,
            48 => SubSport::Commuting,
            49 => SubSport::MixedSurface,
            50 => SubSport::Navigate,
            51 => SubSport::TrackMe,
            52 => SubSport::Map,
            53 => SubSport::SingleGasDiving,
            54 => SubSport::MultiGasDiving,
            55 => SubSport::GaugeDiving,
            56 => SubSport::ApneaDiving,
            57 => SubSport::ApneaHunting,
            58 => SubSport::VirtualActivity,
            59 => SubSport::Obstacle,
            62 => SubSport::Breathing,
            65 => SubSport::SailRace,
            67 => SubSport::Ultra,
            68 => SubSport::IndoorClimbing,
            69 => SubSport::Bouldering,
            70 => SubSport::Hiit,
            73 => SubSport::Amrap,
            74 => SubSport::Emom,
            75 => SubSport::Tabata,
            84 => SubSport::Pickleball,
            85 => SubSport::Padel,
            86 => SubSport::IndoorWheelchairWalk,
            87 => SubSport::IndoorWheelchairRun,
            88 => SubSport::IndoorHandCycling,
            94 => SubSport::Squash,
            95 => SubSport::Badminton,
            96 => SubSport::Racquetball,
            97 => SubSport::TableTennis,
            110 => SubSport::FlyCanopy,
            111 => SubSport::FlyParaglide,
            112 => SubSport::FlyParamotor,
            113 => SubSport::FlyPressurized,
            114 => SubSport::FlyNavigate,
            115 => SubSport::FlyTimer,
            116 => SubSport::FlyAltimeter,
            117 => SubSport::FlyWx,
            118 => SubSport::FlyVfr,
            119 => SubSport::FlyIfr,
            254 => SubSport::All,
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
    Uncategorized,
    Geocaching,
    Fitness,
    Recreation,
    Race,
    SpecialEvent,
    Training,
    Transportation,
    Touring,
    UnknownVariant,
}
impl SportEvent {
    pub fn from(content: u8) -> SportEvent {
        match content {
            0 => SportEvent::Uncategorized,
            1 => SportEvent::Geocaching,
            2 => SportEvent::Fitness,
            3 => SportEvent::Recreation,
            4 => SportEvent::Race,
            5 => SportEvent::SpecialEvent,
            6 => SportEvent::Training,
            7 => SportEvent::Transportation,
            8 => SportEvent::Touring,
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
    Active,
    Rest,
    Warmup,
    Cooldown,
    Recovery,
    Interval,
    Other,
    UnknownVariant,
}
impl Intensity {
    pub fn from(content: u8) -> Intensity {
        match content {
            0 => Intensity::Active,
            1 => Intensity::Rest,
            2 => Intensity::Warmup,
            3 => Intensity::Cooldown,
            4 => Intensity::Recovery,
            5 => Intensity::Interval,
            6 => Intensity::Other,
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
    ActivityEnd,
    Manual,
    AutoMultiSport,
    FitnessEquipment,
    UnknownVariant,
}
impl SessionTrigger {
    pub fn from(content: u8) -> SessionTrigger {
        match content {
            0 => SessionTrigger::ActivityEnd,
            1 => SessionTrigger::Manual,
            2 => SessionTrigger::AutoMultiSport,
            3 => SessionTrigger::FitnessEquipment,
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
    Manual,
    Time,
    Distance,
    PositionStart,
    PositionLap,
    PositionWaypoint,
    PositionMarked,
    SessionEnd,
    FitnessEquipment,
    UnknownVariant,
}
impl LapTrigger {
    pub fn from(content: u8) -> LapTrigger {
        match content {
            0 => LapTrigger::Manual,
            1 => LapTrigger::Time,
            2 => LapTrigger::Distance,
            3 => LapTrigger::PositionStart,
            4 => LapTrigger::PositionLap,
            5 => LapTrigger::PositionWaypoint,
            6 => LapTrigger::PositionMarked,
            7 => LapTrigger::SessionEnd,
            8 => LapTrigger::FitnessEquipment,
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
    Hour12,
    Hour24,
    Military,
    Hour12WithSeconds,
    Hour24WithSeconds,
    Utc,
    UnknownVariant,
}
impl TimeMode {
    pub fn from(content: u8) -> TimeMode {
        match content {
            0 => TimeMode::Hour12,
            1 => TimeMode::Hour24,
            2 => TimeMode::Military,
            3 => TimeMode::Hour12WithSeconds,
            4 => TimeMode::Hour24WithSeconds,
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
    KeyAndMessages,
    AutoBrightness,
    SmartNotifications,
    KeyAndMessagesNight,
    KeyAndMessagesAndSmartNotifications,
    UnknownVariant,
}
impl BacklightMode {
    pub fn from(content: u8) -> BacklightMode {
        match content {
            0 => BacklightMode::Off,
            1 => BacklightMode::Manual,
            2 => BacklightMode::KeyAndMessages,
            3 => BacklightMode::AutoBrightness,
            4 => BacklightMode::SmartNotifications,
            5 => BacklightMode::KeyAndMessagesNight,
            6 => BacklightMode::KeyAndMessagesAndSmartNotifications,
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
    Timer,
    Workout,
    WorkoutStep,
    PowerDown,
    PowerUp,
    OffCourse,
    Session,
    Lap,
    CoursePoint,
    Battery,
    VirtualPartnerPace,
    HrHighAlert,
    HrLowAlert,
    SpeedHighAlert,
    SpeedLowAlert,
    CadHighAlert,
    CadLowAlert,
    PowerHighAlert,
    PowerLowAlert,
    RecoveryHr,
    BatteryLow,
    TimeDurationAlert,
    DistanceDurationAlert,
    CalorieDurationAlert,
    Activity,
    FitnessEquipment,
    Length,
    UserMarker,
    SportPoint,
    Calibration,
    FrontGearChange,
    RearGearChange,
    RiderPositionChange,
    ElevHighAlert,
    ElevLowAlert,
    CommTimeout,
    AutoActivityDetect,
    DiveAlert,
    DiveGasSwitched,
    TankPressureReserve,
    TankPressureCritical,
    TankLost,
    RadarThreatAlert,
    TankBatteryLow,
    TankPodConnected,
    TankPodDisconnected,
    UnknownVariant,
}
impl Event {
    pub fn from(content: u8) -> Event {
        match content {
            0 => Event::Timer,
            3 => Event::Workout,
            4 => Event::WorkoutStep,
            5 => Event::PowerDown,
            6 => Event::PowerUp,
            7 => Event::OffCourse,
            8 => Event::Session,
            9 => Event::Lap,
            10 => Event::CoursePoint,
            11 => Event::Battery,
            12 => Event::VirtualPartnerPace,
            13 => Event::HrHighAlert,
            14 => Event::HrLowAlert,
            15 => Event::SpeedHighAlert,
            16 => Event::SpeedLowAlert,
            17 => Event::CadHighAlert,
            18 => Event::CadLowAlert,
            19 => Event::PowerHighAlert,
            20 => Event::PowerLowAlert,
            21 => Event::RecoveryHr,
            22 => Event::BatteryLow,
            23 => Event::TimeDurationAlert,
            24 => Event::DistanceDurationAlert,
            25 => Event::CalorieDurationAlert,
            26 => Event::Activity,
            27 => Event::FitnessEquipment,
            28 => Event::Length,
            32 => Event::UserMarker,
            33 => Event::SportPoint,
            36 => Event::Calibration,
            42 => Event::FrontGearChange,
            43 => Event::RearGearChange,
            44 => Event::RiderPositionChange,
            45 => Event::ElevHighAlert,
            46 => Event::ElevLowAlert,
            47 => Event::CommTimeout,
            54 => Event::AutoActivityDetect,
            56 => Event::DiveAlert,
            57 => Event::DiveGasSwitched,
            71 => Event::TankPressureReserve,
            72 => Event::TankPressureCritical,
            73 => Event::TankLost,
            75 => Event::RadarThreatAlert,
            76 => Event::TankBatteryLow,
            81 => Event::TankPodConnected,
            82 => Event::TankPodDisconnected,
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
    Start,
    Stop,
    ConsecutiveDepreciated,
    Marker,
    StopAll,
    BeginDepreciated,
    EndDepreciated,
    EndAllDepreciated,
    StopDisable,
    StopDisableAll,
    UnknownVariant,
}
impl EventType {
    pub fn from(content: u8) -> EventType {
        match content {
            0 => EventType::Start,
            1 => EventType::Stop,
            2 => EventType::ConsecutiveDepreciated,
            3 => EventType::Marker,
            4 => EventType::StopAll,
            5 => EventType::BeginDepreciated,
            6 => EventType::EndDepreciated,
            7 => EventType::EndAllDepreciated,
            8 => EventType::StopDisable,
            9 => EventType::StopDisableAll,
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
    Tone,
    Vibrate,
    ToneAndVibrate,
    UnknownVariant,
}
impl Tone {
    pub fn from(content: u8) -> Tone {
        match content {
            0 => Tone::Off,
            1 => Tone::Tone,
            2 => Tone::Vibrate,
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
    Level,
    LevelMax,
    Athlete,
    UnknownVariant,
}
impl ActivityClass {
    pub fn from(content: u8) -> ActivityClass {
        match content {
            127 => ActivityClass::Level,
            100 => ActivityClass::LevelMax,
            128 => ActivityClass::Athlete,
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
    PercentHrr,
    PercentLthr,
    UnknownVariant,
}
impl HrZoneCalc {
    pub fn from(content: u8) -> HrZoneCalc {
        match content {
            0 => HrZoneCalc::Custom,
            1 => HrZoneCalc::PercentMaxHr,
            2 => HrZoneCalc::PercentHrr,
            3 => HrZoneCalc::PercentLthr,
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
    Time,
    Distance,
    HrLessThan,
    HrGreaterThan,
    Calories,
    Open,
    RepeatUntilStepsCmplt,
    RepeatUntilTime,
    RepeatUntilDistance,
    RepeatUntilCalories,
    RepeatUntilHrLessThan,
    RepeatUntilHrGreaterThan,
    RepeatUntilPowerLessThan,
    RepeatUntilPowerGreaterThan,
    PowerLessThan,
    PowerGreaterThan,
    TrainingPeaksTss,
    RepeatUntilPowerLastLapLessThan,
    RepeatUntilMaxPowerLastLapLessThan,
    Power3sLessThan,
    Power10sLessThan,
    Power30sLessThan,
    Power3sGreaterThan,
    Power10sGreaterThan,
    Power30sGreaterThan,
    PowerLapLessThan,
    PowerLapGreaterThan,
    RepeatUntilTrainingPeaksTss,
    RepetitionTime,
    Reps,
    TimeOnly,
    UnknownVariant,
}
impl WktStepDuration {
    pub fn from(content: u8) -> WktStepDuration {
        match content {
            0 => WktStepDuration::Time,
            1 => WktStepDuration::Distance,
            2 => WktStepDuration::HrLessThan,
            3 => WktStepDuration::HrGreaterThan,
            4 => WktStepDuration::Calories,
            5 => WktStepDuration::Open,
            6 => WktStepDuration::RepeatUntilStepsCmplt,
            7 => WktStepDuration::RepeatUntilTime,
            8 => WktStepDuration::RepeatUntilDistance,
            9 => WktStepDuration::RepeatUntilCalories,
            10 => WktStepDuration::RepeatUntilHrLessThan,
            11 => WktStepDuration::RepeatUntilHrGreaterThan,
            12 => WktStepDuration::RepeatUntilPowerLessThan,
            13 => WktStepDuration::RepeatUntilPowerGreaterThan,
            14 => WktStepDuration::PowerLessThan,
            15 => WktStepDuration::PowerGreaterThan,
            16 => WktStepDuration::TrainingPeaksTss,
            17 => WktStepDuration::RepeatUntilPowerLastLapLessThan,
            18 => WktStepDuration::RepeatUntilMaxPowerLastLapLessThan,
            19 => WktStepDuration::Power3sLessThan,
            20 => WktStepDuration::Power10sLessThan,
            21 => WktStepDuration::Power30sLessThan,
            22 => WktStepDuration::Power3sGreaterThan,
            23 => WktStepDuration::Power10sGreaterThan,
            24 => WktStepDuration::Power30sGreaterThan,
            25 => WktStepDuration::PowerLapLessThan,
            26 => WktStepDuration::PowerLapGreaterThan,
            27 => WktStepDuration::RepeatUntilTrainingPeaksTss,
            28 => WktStepDuration::RepetitionTime,
            29 => WktStepDuration::Reps,
            31 => WktStepDuration::TimeOnly,
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
    Speed,
    HeartRate,
    Open,
    Cadence,
    Power,
    Grade,
    Resistance,
    Power3s,
    Power10s,
    Power30s,
    PowerLap,
    SwimStroke,
    SpeedLap,
    HeartRateLap,
    UnknownVariant,
}
impl WktStepTarget {
    pub fn from(content: u8) -> WktStepTarget {
        match content {
            0 => WktStepTarget::Speed,
            1 => WktStepTarget::HeartRate,
            2 => WktStepTarget::Open,
            3 => WktStepTarget::Cadence,
            4 => WktStepTarget::Power,
            5 => WktStepTarget::Grade,
            6 => WktStepTarget::Resistance,
            7 => WktStepTarget::Power3s,
            8 => WktStepTarget::Power10s,
            9 => WktStepTarget::Power30s,
            10 => WktStepTarget::PowerLap,
            11 => WktStepTarget::SwimStroke,
            12 => WktStepTarget::SpeedLap,
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
    Distance,
    Calories,
    Frequency,
    Steps,
    Ascent,
    ActiveMinutes,
    UnknownVariant,
}
impl Goal {
    pub fn from(content: u8) -> Goal {
        match content {
            0 => Goal::Time,
            1 => Goal::Distance,
            2 => Goal::Calories,
            3 => Goal::Frequency,
            4 => Goal::Steps,
            5 => Goal::Ascent,
            6 => Goal::ActiveMinutes,
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
    Off,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Custom,
    UnknownVariant,
}
impl GoalRecurrence {
    pub fn from(content: u8) -> GoalRecurrence {
        match content {
            0 => GoalRecurrence::Off,
            1 => GoalRecurrence::Daily,
            2 => GoalRecurrence::Weekly,
            3 => GoalRecurrence::Monthly,
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
    Workout,
    Course,
    UnknownVariant,
}
impl Schedule {
    pub fn from(content: u8) -> Schedule {
        match content {
            0 => Schedule::Workout,
            1 => Schedule::Course,
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
    Generic,
    Summit,
    Valley,
    Water,
    Food,
    Danger,
    Left,
    Right,
    Straight,
    FirstAid,
    FourthCategory,
    ThirdCategory,
    SecondCategory,
    FirstCategory,
    HorsCategory,
    Sprint,
    LeftFork,
    RightFork,
    MiddleFork,
    SlightLeft,
    SharpLeft,
    SlightRight,
    SharpRight,
    UTurn,
    SegmentStart,
    SegmentEnd,
    Campsite,
    AidStation,
    RestArea,
    GeneralDistance,
    Service,
    EnergyGel,
    SportsDrink,
    MileMarker,
    Checkpoint,
    Shelter,
    MeetingSpot,
    Overlook,
    Toilet,
    Shower,
    Gear,
    SharpCurve,
    SteepIncline,
    Tunnel,
    Bridge,
    Obstacle,
    Crossing,
    Store,
    Transition,
    Navaid,
    Transport,
    Alert,
    Info,
    UnknownVariant,
}
impl CoursePoint {
    pub fn from(content: u8) -> CoursePoint {
        match content {
            0 => CoursePoint::Generic,
            1 => CoursePoint::Summit,
            2 => CoursePoint::Valley,
            3 => CoursePoint::Water,
            4 => CoursePoint::Food,
            5 => CoursePoint::Danger,
            6 => CoursePoint::Left,
            7 => CoursePoint::Right,
            8 => CoursePoint::Straight,
            9 => CoursePoint::FirstAid,
            10 => CoursePoint::FourthCategory,
            11 => CoursePoint::ThirdCategory,
            12 => CoursePoint::SecondCategory,
            13 => CoursePoint::FirstCategory,
            14 => CoursePoint::HorsCategory,
            15 => CoursePoint::Sprint,
            16 => CoursePoint::LeftFork,
            17 => CoursePoint::RightFork,
            18 => CoursePoint::MiddleFork,
            19 => CoursePoint::SlightLeft,
            20 => CoursePoint::SharpLeft,
            21 => CoursePoint::SlightRight,
            22 => CoursePoint::SharpRight,
            23 => CoursePoint::UTurn,
            24 => CoursePoint::SegmentStart,
            25 => CoursePoint::SegmentEnd,
            27 => CoursePoint::Campsite,
            28 => CoursePoint::AidStation,
            29 => CoursePoint::RestArea,
            30 => CoursePoint::GeneralDistance,
            31 => CoursePoint::Service,
            32 => CoursePoint::EnergyGel,
            33 => CoursePoint::SportsDrink,
            34 => CoursePoint::MileMarker,
            35 => CoursePoint::Checkpoint,
            36 => CoursePoint::Shelter,
            37 => CoursePoint::MeetingSpot,
            38 => CoursePoint::Overlook,
            39 => CoursePoint::Toilet,
            40 => CoursePoint::Shower,
            41 => CoursePoint::Gear,
            42 => CoursePoint::SharpCurve,
            43 => CoursePoint::SteepIncline,
            44 => CoursePoint::Tunnel,
            45 => CoursePoint::Bridge,
            46 => CoursePoint::Obstacle,
            47 => CoursePoint::Crossing,
            48 => CoursePoint::Store,
            49 => CoursePoint::Transition,
            50 => CoursePoint::Navaid,
            51 => CoursePoint::Transport,
            52 => CoursePoint::Alert,
            53 => CoursePoint::Info,
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
    Garmin,
    GarminFr405Antfs,
    Zephyr,
    Dayton,
    Idt,
    Srm,
    Quarq,
    Ibike,
    Saris,
    SparkHk,
    Tanita,
    Echowell,
    DynastreamOem,
    Nautilus,
    Dynastream,
    Timex,
    Metrigear,
    Xelic,
    Beurer,
    Cardiosport,
    AAndD,
    Hmm,
    Suunto,
    ThitaElektronik,
    Gpulse,
    CleanMobile,
    PedalBrain,
    Peaksware,
    Saxonar,
    LemondFitness,
    Dexcom,
    WahooFitness,
    OctaneFitness,
    Archinoetics,
    TheHurtBox,
    CitizenSystems,
    Magellan,
    Osynce,
    Holux,
    Concept2,
    Shimano,
    OneGiantLeap,
    AceSensor,
    BrimBrothers,
    Xplova,
    PerceptionDigital,
    Bf1systems,
    Pioneer,
    Spantec,
    Metalogics,
    Iiiis,
    SeikoEpson,
    SeikoEpsonOem,
    IforPowell,
    MaxwellGuider,
    StarTrac,
    Breakaway,
    AlatechTechnologyLtd,
    MioTechnologyEurope,
    Rotor,
    Geonaute,
    IdBike,
    Specialized,
    Wtek,
    PhysicalEnterprises,
    NorthPoleEngineering,
    Bkool,
    Cateye,
    StagesCycling,
    Sigmasport,
    Tomtom,
    Peripedal,
    Wattbike,
    Moxy,
    Ciclosport,
    Powerbahn,
    AcornProjectsAps,
    Lifebeam,
    Bontrager,
    Wellgo,
    Scosche,
    Magura,
    Woodway,
    Elite,
    NielsenKellerman,
    DkCity,
    Tacx,
    DirectionTechnology,
    Magtonic,
    Partcarbon,
    InsideRideTechnologies,
    SoundOfMotion,
    Stryd,
    Icg,
    MiPulse,
    BsxAthletics,
    Look,
    CampagnoloSrl,
    BodyBikeSmart,
    Praxisworks,
    LimitsTechnology,
    TopactionTechnology,
    Cosinuss,
    Fitcare,
    Magene,
    GiantManufacturingCo,
    Tigrasport,
    Salutron,
    Technogym,
    BrytonSensors,
    LatitudeLimited,
    SoaringTechnology,
    Igpsport,
    Thinkrider,
    GopherSport,
    Waterrower,
    Orangetheory,
    Inpeak,
    Kinetic,
    JohnsonHealthTech,
    PolarElectro,
    Seesense,
    NciTechnology,
    Iqsquare,
    Leomo,
    IfitCom,
    CorosByte,
    VersaDesign,
    Chileaf,
    Cycplus,
    GravaaByte,
    Sigeyi,
    Coospo,
    Geoid,
    Bosch,
    Kyto,
    KineticSports,
    DecathlonByte,
    TqSystems,
    TagHeuer,
    KeiserFitness,
    ZwiftByte,
    PorscheEp,
    Blackbird,
    MeilanByte,
    Ezon,
    Laisi,
    Myzone,
    Abawo,
    Bafang,
    LuhongTechnology,
    Development,
    Healthandlife,
    Lezyne,
    ScribeLabs,
    Zwift,
    Watteam,
    Recon,
    FaveroElectronics,
    Dynovelo,
    Strava,
    Precor,
    Bryton,
    Sram,
    Navman,
    Cobi,
    Spivi,
    MioMagellan,
    Evesports,
    SensitivusGauge,
    Podoon,
    LifeTimeFitness,
    FalcoEMotors,
    Minoura,
    Cycliq,
    Luxottica,
    TrainerRoad,
    TheSufferfest,
    Fullspeedahead,
    Virtualtraining,
    Feedbacksports,
    Omata,
    Vdo,
    Magneticdays,
    Hammerhead,
    KineticByKurt,
    Shapelog,
    Dabuziduo,
    Jetblack,
    Coros,
    Virtugo,
    Velosense,
    Cycligentinc,
    Trailforks,
    MahleEbikemotion,
    Nurvv,
    Microprogram,
    Zone5cloud,
    Greenteg,
    YamahaMotors,
    Whoop,
    Gravaa,
    Onelap,
    MonarkExercise,
    Form,
    Decathlon,
    Syncros,
    Heatup,
    Cannondale,
    TrueFitness,
    RGTCycling,
    Vasa,
    RaceRepublic,
    Fazua,
    OrekaTraining,
    Lsec,
    LululemonStudio,
    Shanyue,
    SpinningMda,
    Hilldating,
    AeroSensor,
    Nike,
    Magicshine,
    Ictrainer,
    AbsoluteCycling,
    EoSwimbetter,
    Mywhoosh,
    Ravemen,
    TektroRacingProducts,
    DaradInnovationCorporation,
    Cycloptim,
    Actigraphcorp,
    UnknownVariant,
}
impl Manufacturer {
    pub fn from(content: u16) -> Manufacturer {
        match content {
            1 => Manufacturer::Garmin,
            2 => Manufacturer::GarminFr405Antfs,
            3 => Manufacturer::Zephyr,
            4 => Manufacturer::Dayton,
            5 => Manufacturer::Idt,
            6 => Manufacturer::Srm,
            7 => Manufacturer::Quarq,
            8 => Manufacturer::Ibike,
            9 => Manufacturer::Saris,
            10 => Manufacturer::SparkHk,
            11 => Manufacturer::Tanita,
            12 => Manufacturer::Echowell,
            13 => Manufacturer::DynastreamOem,
            14 => Manufacturer::Nautilus,
            15 => Manufacturer::Dynastream,
            16 => Manufacturer::Timex,
            17 => Manufacturer::Metrigear,
            18 => Manufacturer::Xelic,
            19 => Manufacturer::Beurer,
            20 => Manufacturer::Cardiosport,
            21 => Manufacturer::AAndD,
            22 => Manufacturer::Hmm,
            23 => Manufacturer::Suunto,
            24 => Manufacturer::ThitaElektronik,
            25 => Manufacturer::Gpulse,
            26 => Manufacturer::CleanMobile,
            27 => Manufacturer::PedalBrain,
            28 => Manufacturer::Peaksware,
            29 => Manufacturer::Saxonar,
            30 => Manufacturer::LemondFitness,
            31 => Manufacturer::Dexcom,
            32 => Manufacturer::WahooFitness,
            33 => Manufacturer::OctaneFitness,
            34 => Manufacturer::Archinoetics,
            35 => Manufacturer::TheHurtBox,
            36 => Manufacturer::CitizenSystems,
            37 => Manufacturer::Magellan,
            38 => Manufacturer::Osynce,
            39 => Manufacturer::Holux,
            40 => Manufacturer::Concept2,
            41 => Manufacturer::Shimano,
            42 => Manufacturer::OneGiantLeap,
            43 => Manufacturer::AceSensor,
            44 => Manufacturer::BrimBrothers,
            45 => Manufacturer::Xplova,
            46 => Manufacturer::PerceptionDigital,
            47 => Manufacturer::Bf1systems,
            48 => Manufacturer::Pioneer,
            49 => Manufacturer::Spantec,
            50 => Manufacturer::Metalogics,
            51 => Manufacturer::Iiiis,
            52 => Manufacturer::SeikoEpson,
            53 => Manufacturer::SeikoEpsonOem,
            54 => Manufacturer::IforPowell,
            55 => Manufacturer::MaxwellGuider,
            56 => Manufacturer::StarTrac,
            57 => Manufacturer::Breakaway,
            58 => Manufacturer::AlatechTechnologyLtd,
            59 => Manufacturer::MioTechnologyEurope,
            60 => Manufacturer::Rotor,
            61 => Manufacturer::Geonaute,
            62 => Manufacturer::IdBike,
            63 => Manufacturer::Specialized,
            64 => Manufacturer::Wtek,
            65 => Manufacturer::PhysicalEnterprises,
            66 => Manufacturer::NorthPoleEngineering,
            67 => Manufacturer::Bkool,
            68 => Manufacturer::Cateye,
            69 => Manufacturer::StagesCycling,
            70 => Manufacturer::Sigmasport,
            71 => Manufacturer::Tomtom,
            72 => Manufacturer::Peripedal,
            73 => Manufacturer::Wattbike,
            76 => Manufacturer::Moxy,
            77 => Manufacturer::Ciclosport,
            78 => Manufacturer::Powerbahn,
            79 => Manufacturer::AcornProjectsAps,
            80 => Manufacturer::Lifebeam,
            81 => Manufacturer::Bontrager,
            82 => Manufacturer::Wellgo,
            83 => Manufacturer::Scosche,
            84 => Manufacturer::Magura,
            85 => Manufacturer::Woodway,
            86 => Manufacturer::Elite,
            87 => Manufacturer::NielsenKellerman,
            88 => Manufacturer::DkCity,
            89 => Manufacturer::Tacx,
            90 => Manufacturer::DirectionTechnology,
            91 => Manufacturer::Magtonic,
            92 => Manufacturer::Partcarbon,
            93 => Manufacturer::InsideRideTechnologies,
            94 => Manufacturer::SoundOfMotion,
            95 => Manufacturer::Stryd,
            96 => Manufacturer::Icg,
            97 => Manufacturer::MiPulse,
            98 => Manufacturer::BsxAthletics,
            99 => Manufacturer::Look,
            100 => Manufacturer::CampagnoloSrl,
            101 => Manufacturer::BodyBikeSmart,
            102 => Manufacturer::Praxisworks,
            103 => Manufacturer::LimitsTechnology,
            104 => Manufacturer::TopactionTechnology,
            105 => Manufacturer::Cosinuss,
            106 => Manufacturer::Fitcare,
            107 => Manufacturer::Magene,
            108 => Manufacturer::GiantManufacturingCo,
            109 => Manufacturer::Tigrasport,
            110 => Manufacturer::Salutron,
            111 => Manufacturer::Technogym,
            112 => Manufacturer::BrytonSensors,
            113 => Manufacturer::LatitudeLimited,
            114 => Manufacturer::SoaringTechnology,
            115 => Manufacturer::Igpsport,
            116 => Manufacturer::Thinkrider,
            117 => Manufacturer::GopherSport,
            118 => Manufacturer::Waterrower,
            119 => Manufacturer::Orangetheory,
            120 => Manufacturer::Inpeak,
            121 => Manufacturer::Kinetic,
            122 => Manufacturer::JohnsonHealthTech,
            123 => Manufacturer::PolarElectro,
            124 => Manufacturer::Seesense,
            125 => Manufacturer::NciTechnology,
            126 => Manufacturer::Iqsquare,
            127 => Manufacturer::Leomo,
            128 => Manufacturer::IfitCom,
            129 => Manufacturer::CorosByte,
            130 => Manufacturer::VersaDesign,
            131 => Manufacturer::Chileaf,
            132 => Manufacturer::Cycplus,
            133 => Manufacturer::GravaaByte,
            134 => Manufacturer::Sigeyi,
            135 => Manufacturer::Coospo,
            136 => Manufacturer::Geoid,
            137 => Manufacturer::Bosch,
            138 => Manufacturer::Kyto,
            139 => Manufacturer::KineticSports,
            140 => Manufacturer::DecathlonByte,
            141 => Manufacturer::TqSystems,
            142 => Manufacturer::TagHeuer,
            143 => Manufacturer::KeiserFitness,
            144 => Manufacturer::ZwiftByte,
            145 => Manufacturer::PorscheEp,
            146 => Manufacturer::Blackbird,
            147 => Manufacturer::MeilanByte,
            148 => Manufacturer::Ezon,
            149 => Manufacturer::Laisi,
            150 => Manufacturer::Myzone,
            151 => Manufacturer::Abawo,
            152 => Manufacturer::Bafang,
            153 => Manufacturer::LuhongTechnology,
            255 => Manufacturer::Development,
            257 => Manufacturer::Healthandlife,
            258 => Manufacturer::Lezyne,
            259 => Manufacturer::ScribeLabs,
            260 => Manufacturer::Zwift,
            261 => Manufacturer::Watteam,
            262 => Manufacturer::Recon,
            263 => Manufacturer::FaveroElectronics,
            264 => Manufacturer::Dynovelo,
            265 => Manufacturer::Strava,
            266 => Manufacturer::Precor,
            267 => Manufacturer::Bryton,
            268 => Manufacturer::Sram,
            269 => Manufacturer::Navman,
            270 => Manufacturer::Cobi,
            271 => Manufacturer::Spivi,
            272 => Manufacturer::MioMagellan,
            273 => Manufacturer::Evesports,
            274 => Manufacturer::SensitivusGauge,
            275 => Manufacturer::Podoon,
            276 => Manufacturer::LifeTimeFitness,
            277 => Manufacturer::FalcoEMotors,
            278 => Manufacturer::Minoura,
            279 => Manufacturer::Cycliq,
            280 => Manufacturer::Luxottica,
            281 => Manufacturer::TrainerRoad,
            282 => Manufacturer::TheSufferfest,
            283 => Manufacturer::Fullspeedahead,
            284 => Manufacturer::Virtualtraining,
            285 => Manufacturer::Feedbacksports,
            286 => Manufacturer::Omata,
            287 => Manufacturer::Vdo,
            288 => Manufacturer::Magneticdays,
            289 => Manufacturer::Hammerhead,
            290 => Manufacturer::KineticByKurt,
            291 => Manufacturer::Shapelog,
            292 => Manufacturer::Dabuziduo,
            293 => Manufacturer::Jetblack,
            294 => Manufacturer::Coros,
            295 => Manufacturer::Virtugo,
            296 => Manufacturer::Velosense,
            297 => Manufacturer::Cycligentinc,
            298 => Manufacturer::Trailforks,
            299 => Manufacturer::MahleEbikemotion,
            300 => Manufacturer::Nurvv,
            301 => Manufacturer::Microprogram,
            302 => Manufacturer::Zone5cloud,
            303 => Manufacturer::Greenteg,
            304 => Manufacturer::YamahaMotors,
            305 => Manufacturer::Whoop,
            306 => Manufacturer::Gravaa,
            307 => Manufacturer::Onelap,
            308 => Manufacturer::MonarkExercise,
            309 => Manufacturer::Form,
            310 => Manufacturer::Decathlon,
            311 => Manufacturer::Syncros,
            312 => Manufacturer::Heatup,
            313 => Manufacturer::Cannondale,
            314 => Manufacturer::TrueFitness,
            315 => Manufacturer::RGTCycling,
            316 => Manufacturer::Vasa,
            317 => Manufacturer::RaceRepublic,
            318 => Manufacturer::Fazua,
            319 => Manufacturer::OrekaTraining,
            320 => Manufacturer::Lsec,
            321 => Manufacturer::LululemonStudio,
            322 => Manufacturer::Shanyue,
            323 => Manufacturer::SpinningMda,
            324 => Manufacturer::Hilldating,
            325 => Manufacturer::AeroSensor,
            326 => Manufacturer::Nike,
            327 => Manufacturer::Magicshine,
            328 => Manufacturer::Ictrainer,
            329 => Manufacturer::AbsoluteCycling,
            330 => Manufacturer::EoSwimbetter,
            331 => Manufacturer::Mywhoosh,
            332 => Manufacturer::Ravemen,
            333 => Manufacturer::TektroRacingProducts,
            334 => Manufacturer::DaradInnovationCorporation,
            335 => Manufacturer::Cycloptim,
            5759 => Manufacturer::Actigraphcorp,
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
    Public,
    Antplus,
    Antfs,
    Private,
    UnknownVariant,
}
impl AntNetwork {
    pub fn from(content: u8) -> AntNetwork {
        match content {
            0 => AntNetwork::Public,
            1 => AntNetwork::Antplus,
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
    Interval,
    Custom,
    FitnessEquipment,
    Firstbeat,
    NewLeaf,
    Tcx,
    Speed,
    HeartRate,
    Distance,
    Cadence,
    Power,
    Grade,
    Resistance,
    Protected,
    UnknownVariant,
}
impl WorkoutCapabilities {
    pub fn from(content: u32) -> WorkoutCapabilities {
        match content {
            1 => WorkoutCapabilities::Interval,
            2 => WorkoutCapabilities::Custom,
            4 => WorkoutCapabilities::FitnessEquipment,
            8 => WorkoutCapabilities::Firstbeat,
            16 => WorkoutCapabilities::NewLeaf,
            32 => WorkoutCapabilities::Tcx,
            128 => WorkoutCapabilities::Speed,
            256 => WorkoutCapabilities::HeartRate,
            512 => WorkoutCapabilities::Distance,
            1024 => WorkoutCapabilities::Cadence,
            2048 => WorkoutCapabilities::Power,
            4096 => WorkoutCapabilities::Grade,
            8192 => WorkoutCapabilities::Resistance,
            16384 => WorkoutCapabilities::Protected,
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
    New,
    Good,
    Ok,
    Low,
    Critical,
    Charging,
    Unknown,
    UnknownVariant,
}
impl BatteryStatus {
    pub fn from(content: u8) -> BatteryStatus {
        match content {
            1 => BatteryStatus::New,
            2 => BatteryStatus::Good,
            3 => BatteryStatus::Ok,
            4 => BatteryStatus::Low,
            5 => BatteryStatus::Critical,
            6 => BatteryStatus::Charging,
            7 => BatteryStatus::Unknown,
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
    Processed,
    Valid,
    Time,
    Distance,
    Position,
    HeartRate,
    Power,
    Cadence,
    Training,
    Navigation,
    Bikeway,
    Aviation,
    UnknownVariant,
}
impl CourseCapabilities {
    pub fn from(content: u32) -> CourseCapabilities {
        match content {
            1 => CourseCapabilities::Processed,
            2 => CourseCapabilities::Valid,
            4 => CourseCapabilities::Time,
            8 => CourseCapabilities::Distance,
            16 => CourseCapabilities::Position,
            32 => CourseCapabilities::HeartRate,
            64 => CourseCapabilities::Power,
            128 => CourseCapabilities::Cadence,
            256 => CourseCapabilities::Training,
            512 => CourseCapabilities::Navigation,
            1024 => CourseCapabilities::Bikeway,
            4096 => CourseCapabilities::Aviation,
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
    NoError,
    ErrorIncompleteData,
    ErrorNoMeasurement,
    ErrorDataOutOfRange,
    ErrorIrregularHeartRate,
    UnknownVariant,
}
impl BpStatus {
    pub fn from(content: u8) -> BpStatus {
        match content {
            0 => BpStatus::NoError,
            1 => BpStatus::ErrorIncompleteData,
            2 => BpStatus::ErrorNoMeasurement,
            3 => BpStatus::ErrorDataOutOfRange,
            4 => BpStatus::ErrorIrregularHeartRate,
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
    LocalMin,
    LocalMax,
    StationaryMin,
    StationaryMax,
    PortableMin,
    PortableMax,
    UnknownVariant,
}
impl UserLocalId {
    pub fn from(content: u16) -> UserLocalId {
        match content {
            0 => UserLocalId::LocalMin,
            15 => UserLocalId::LocalMax,
            16 => UserLocalId::StationaryMin,
            255 => UserLocalId::StationaryMax,
            256 => UserLocalId::PortableMin,
            65534 => UserLocalId::PortableMax,
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
    Backstroke,
    Breaststroke,
    Butterfly,
    Drill,
    Mixed,
    Im,
    ImByRound,
    Rimo,
    UnknownVariant,
}
impl SwimStroke {
    pub fn from(content: u8) -> SwimStroke {
        match content {
            0 => SwimStroke::Freestyle,
            1 => SwimStroke::Backstroke,
            2 => SwimStroke::Breaststroke,
            3 => SwimStroke::Butterfly,
            4 => SwimStroke::Drill,
            5 => SwimStroke::Mixed,
            6 => SwimStroke::Im,
            7 => SwimStroke::ImByRound,
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
    Generic,
    Running,
    Cycling,
    Transition,
    FitnessEquipment,
    Swimming,
    Walking,
    Sedentary,
    All,
    UnknownVariant,
}
impl ActivityType {
    pub fn from(content: u8) -> ActivityType {
        match content {
            0 => ActivityType::Generic,
            1 => ActivityType::Running,
            2 => ActivityType::Cycling,
            3 => ActivityType::Transition,
            4 => ActivityType::FitnessEquipment,
            5 => ActivityType::Swimming,
            6 => ActivityType::Walking,
            8 => ActivityType::Sedentary,
            254 => ActivityType::All,
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
    Generic,
    Treadmill,
    Street,
    Trail,
    Track,
    Spin,
    IndoorCycling,
    Road,
    Mountain,
    Downhill,
    Recumbent,
    Cyclocross,
    HandCycling,
    TrackCycling,
    IndoorRowing,
    Elliptical,
    StairClimbing,
    LapSwimming,
    OpenWater,
    All,
    UnknownVariant,
}
impl ActivitySubtype {
    pub fn from(content: u8) -> ActivitySubtype {
        match content {
            0 => ActivitySubtype::Generic,
            1 => ActivitySubtype::Treadmill,
            2 => ActivitySubtype::Street,
            3 => ActivitySubtype::Trail,
            4 => ActivitySubtype::Track,
            5 => ActivitySubtype::Spin,
            6 => ActivitySubtype::IndoorCycling,
            7 => ActivitySubtype::Road,
            8 => ActivitySubtype::Mountain,
            9 => ActivitySubtype::Downhill,
            10 => ActivitySubtype::Recumbent,
            11 => ActivitySubtype::Cyclocross,
            12 => ActivitySubtype::HandCycling,
            13 => ActivitySubtype::TrackCycling,
            14 => ActivitySubtype::IndoorRowing,
            15 => ActivitySubtype::Elliptical,
            16 => ActivitySubtype::StairClimbing,
            17 => ActivitySubtype::LapSwimming,
            18 => ActivitySubtype::OpenWater,
            254 => ActivitySubtype::All,
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
    Low,
    Medium,
    High,
    UnknownVariant,
}
impl ActivityLevel {
    pub fn from(content: u8) -> ActivityLevel {
        match content {
            0 => ActivityLevel::Low,
            1 => ActivityLevel::Medium,
            2 => ActivityLevel::High,
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
    Mask,
    Right,
    UnknownVariant,
}
impl LeftRightBalance {
    pub fn from(content: u8) -> LeftRightBalance {
        match content {
            127 => LeftRightBalance::Mask,
            128 => LeftRightBalance::Right,
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
    Mask,
    Right,
    UnknownVariant,
}
impl LeftRightBalance100 {
    pub fn from(content: u16) -> LeftRightBalance100 {
        match content {
            16383 => LeftRightBalance100::Mask,
            32768 => LeftRightBalance100::Right,
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
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    UnknownVariant,
}
impl DayOfWeek {
    pub fn from(content: u8) -> DayOfWeek {
        match content {
            0 => DayOfWeek::Sunday,
            1 => DayOfWeek::Monday,
            2 => DayOfWeek::Tuesday,
            3 => DayOfWeek::Wednesday,
            4 => DayOfWeek::Thursday,
            5 => DayOfWeek::Friday,
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
    Bluetooth,
    BluetoothLe,
    Ant,
    ActivityUpload,
    CourseDownload,
    WorkoutDownload,
    LiveTrack,
    WeatherConditions,
    WeatherAlerts,
    GpsEphemerisDownload,
    ExplicitArchive,
    SetupIncomplete,
    ContinueSyncAfterSoftwareUpdate,
    ConnectIqAppDownload,
    GolfCourseDownload,
    DeviceInitiatesSync,
    ConnectIqWatchAppDownload,
    ConnectIqWidgetDownload,
    ConnectIqWatchFaceDownload,
    ConnectIqDataFieldDownload,
    ConnectIqAppManagment,
    SwingSensor,
    SwingSensorRemote,
    IncidentDetection,
    AudioPrompts,
    WifiVerification,
    TrueUp,
    FindMyWatch,
    RemoteManualSync,
    LiveTrackAutoStart,
    LiveTrackMessaging,
    InstantInput,
    UnknownVariant,
}
impl ConnectivityCapabilities {
    pub fn from(content: u32) -> ConnectivityCapabilities {
        match content {
            1 => ConnectivityCapabilities::Bluetooth,
            2 => ConnectivityCapabilities::BluetoothLe,
            4 => ConnectivityCapabilities::Ant,
            8 => ConnectivityCapabilities::ActivityUpload,
            16 => ConnectivityCapabilities::CourseDownload,
            32 => ConnectivityCapabilities::WorkoutDownload,
            64 => ConnectivityCapabilities::LiveTrack,
            128 => ConnectivityCapabilities::WeatherConditions,
            256 => ConnectivityCapabilities::WeatherAlerts,
            512 => ConnectivityCapabilities::GpsEphemerisDownload,
            1024 => ConnectivityCapabilities::ExplicitArchive,
            2048 => ConnectivityCapabilities::SetupIncomplete,
            4096 => ConnectivityCapabilities::ContinueSyncAfterSoftwareUpdate,
            8192 => ConnectivityCapabilities::ConnectIqAppDownload,
            16384 => ConnectivityCapabilities::GolfCourseDownload,
            32768 => ConnectivityCapabilities::DeviceInitiatesSync,
            65536 => ConnectivityCapabilities::ConnectIqWatchAppDownload,
            131072 => ConnectivityCapabilities::ConnectIqWidgetDownload,
            262144 => ConnectivityCapabilities::ConnectIqWatchFaceDownload,
            524288 => ConnectivityCapabilities::ConnectIqDataFieldDownload,
            1048576 => ConnectivityCapabilities::ConnectIqAppManagment,
            2097152 => ConnectivityCapabilities::SwingSensor,
            4194304 => ConnectivityCapabilities::SwingSensorRemote,
            8388608 => ConnectivityCapabilities::IncidentDetection,
            16777216 => ConnectivityCapabilities::AudioPrompts,
            33554432 => ConnectivityCapabilities::WifiVerification,
            67108864 => ConnectivityCapabilities::TrueUp,
            134217728 => ConnectivityCapabilities::FindMyWatch,
            268435456 => ConnectivityCapabilities::RemoteManualSync,
            536870912 => ConnectivityCapabilities::LiveTrackAutoStart,
            1073741824 => ConnectivityCapabilities::LiveTrackMessaging,
            2147483648 => ConnectivityCapabilities::InstantInput,
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
    Current,
    Forecast,
    HourlyForecast,
    DailyForecast,
    UnknownVariant,
}
impl WeatherReport {
    pub fn from(content: u8) -> WeatherReport {
        match content {
            0 => WeatherReport::Current,
            1 => WeatherReport::Forecast,
            1 => WeatherReport::HourlyForecast,
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
    Clear,
    PartlyCloudy,
    MostlyCloudy,
    Rain,
    Snow,
    Windy,
    Thunderstorms,
    WintryMix,
    Fog,
    Hazy,
    Hail,
    ScatteredShowers,
    ScatteredThunderstorms,
    UnknownPrecipitation,
    LightRain,
    HeavyRain,
    LightSnow,
    HeavySnow,
    LightRainSnow,
    HeavyRainSnow,
    Cloudy,
    UnknownVariant,
}
impl WeatherStatus {
    pub fn from(content: u8) -> WeatherStatus {
        match content {
            0 => WeatherStatus::Clear,
            1 => WeatherStatus::PartlyCloudy,
            2 => WeatherStatus::MostlyCloudy,
            3 => WeatherStatus::Rain,
            4 => WeatherStatus::Snow,
            5 => WeatherStatus::Windy,
            6 => WeatherStatus::Thunderstorms,
            7 => WeatherStatus::WintryMix,
            8 => WeatherStatus::Fog,
            11 => WeatherStatus::Hazy,
            12 => WeatherStatus::Hail,
            13 => WeatherStatus::ScatteredShowers,
            14 => WeatherStatus::ScatteredThunderstorms,
            15 => WeatherStatus::UnknownPrecipitation,
            16 => WeatherStatus::LightRain,
            17 => WeatherStatus::HeavyRain,
            18 => WeatherStatus::LightSnow,
            19 => WeatherStatus::HeavySnow,
            20 => WeatherStatus::LightRainSnow,
            21 => WeatherStatus::HeavyRainSnow,
            22 => WeatherStatus::Cloudy,
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
    Unknown,
    Warning,
    Watch,
    Advisory,
    Statement,
    UnknownVariant,
}
impl WeatherSeverity {
    pub fn from(content: u8) -> WeatherSeverity {
        match content {
            0 => WeatherSeverity::Unknown,
            1 => WeatherSeverity::Warning,
            2 => WeatherSeverity::Watch,
            3 => WeatherSeverity::Advisory,
            4 => WeatherSeverity::Statement,
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
    Unspecified,
    Tornado,
    Tsunami,
    Hurricane,
    ExtremeWind,
    Typhoon,
    InlandHurricane,
    HurricaneForceWind,
    Waterspout,
    SevereThunderstorm,
    WreckhouseWinds,
    LesSuetesWind,
    Avalanche,
    FlashFlood,
    TropicalStorm,
    InlandTropicalStorm,
    Blizzard,
    IceStorm,
    FreezingRain,
    DebrisFlow,
    FlashFreeze,
    DustStorm,
    HighWind,
    WinterStorm,
    HeavyFreezingSpray,
    ExtremeCold,
    WindChill,
    ColdWave,
    HeavySnowAlert,
    LakeEffectBlowingSnow,
    SnowSquall,
    LakeEffectSnow,
    WinterWeather,
    Sleet,
    Snowfall,
    SnowAndBlowingSnow,
    BlowingSnow,
    SnowAlert,
    ArcticOutflow,
    FreezingDrizzle,
    Storm,
    StormSurge,
    Rainfall,
    ArealFlood,
    CoastalFlood,
    LakeshoreFlood,
    ExcessiveHeat,
    Heat,
    Weather,
    HighHeatAndHumidity,
    HumidexAndHealth,
    Humidex,
    Gale,
    FreezingSpray,
    SpecialMarine,
    Squall,
    StrongWind,
    LakeWind,
    MarineWeather,
    Wind,
    SmallCraftHazardousSeas,
    HazardousSeas,
    SmallCraft,
    SmallCraftWinds,
    SmallCraftRoughBar,
    HighWaterLevel,
    Ashfall,
    FreezingFog,
    DenseFog,
    DenseSmoke,
    BlowingDust,
    HardFreeze,
    Freeze,
    Frost,
    FireWeather,
    Flood,
    RipTide,
    HighSurf,
    Smog,
    AirQuality,
    BriskWind,
    AirStagnation,
    LowWater,
    Hydrological,
    SpecialWeather,
    UnknownVariant,
}
impl WeatherSevereType {
    pub fn from(content: u8) -> WeatherSevereType {
        match content {
            0 => WeatherSevereType::Unspecified,
            1 => WeatherSevereType::Tornado,
            2 => WeatherSevereType::Tsunami,
            3 => WeatherSevereType::Hurricane,
            4 => WeatherSevereType::ExtremeWind,
            5 => WeatherSevereType::Typhoon,
            6 => WeatherSevereType::InlandHurricane,
            7 => WeatherSevereType::HurricaneForceWind,
            8 => WeatherSevereType::Waterspout,
            9 => WeatherSevereType::SevereThunderstorm,
            10 => WeatherSevereType::WreckhouseWinds,
            11 => WeatherSevereType::LesSuetesWind,
            12 => WeatherSevereType::Avalanche,
            13 => WeatherSevereType::FlashFlood,
            14 => WeatherSevereType::TropicalStorm,
            15 => WeatherSevereType::InlandTropicalStorm,
            16 => WeatherSevereType::Blizzard,
            17 => WeatherSevereType::IceStorm,
            18 => WeatherSevereType::FreezingRain,
            19 => WeatherSevereType::DebrisFlow,
            20 => WeatherSevereType::FlashFreeze,
            21 => WeatherSevereType::DustStorm,
            22 => WeatherSevereType::HighWind,
            23 => WeatherSevereType::WinterStorm,
            24 => WeatherSevereType::HeavyFreezingSpray,
            25 => WeatherSevereType::ExtremeCold,
            26 => WeatherSevereType::WindChill,
            27 => WeatherSevereType::ColdWave,
            28 => WeatherSevereType::HeavySnowAlert,
            29 => WeatherSevereType::LakeEffectBlowingSnow,
            30 => WeatherSevereType::SnowSquall,
            31 => WeatherSevereType::LakeEffectSnow,
            32 => WeatherSevereType::WinterWeather,
            33 => WeatherSevereType::Sleet,
            34 => WeatherSevereType::Snowfall,
            35 => WeatherSevereType::SnowAndBlowingSnow,
            36 => WeatherSevereType::BlowingSnow,
            37 => WeatherSevereType::SnowAlert,
            38 => WeatherSevereType::ArcticOutflow,
            39 => WeatherSevereType::FreezingDrizzle,
            40 => WeatherSevereType::Storm,
            41 => WeatherSevereType::StormSurge,
            42 => WeatherSevereType::Rainfall,
            43 => WeatherSevereType::ArealFlood,
            44 => WeatherSevereType::CoastalFlood,
            45 => WeatherSevereType::LakeshoreFlood,
            46 => WeatherSevereType::ExcessiveHeat,
            47 => WeatherSevereType::Heat,
            48 => WeatherSevereType::Weather,
            49 => WeatherSevereType::HighHeatAndHumidity,
            50 => WeatherSevereType::HumidexAndHealth,
            51 => WeatherSevereType::Humidex,
            52 => WeatherSevereType::Gale,
            53 => WeatherSevereType::FreezingSpray,
            54 => WeatherSevereType::SpecialMarine,
            55 => WeatherSevereType::Squall,
            56 => WeatherSevereType::StrongWind,
            57 => WeatherSevereType::LakeWind,
            58 => WeatherSevereType::MarineWeather,
            59 => WeatherSevereType::Wind,
            60 => WeatherSevereType::SmallCraftHazardousSeas,
            61 => WeatherSevereType::HazardousSeas,
            62 => WeatherSevereType::SmallCraft,
            63 => WeatherSevereType::SmallCraftWinds,
            64 => WeatherSevereType::SmallCraftRoughBar,
            65 => WeatherSevereType::HighWaterLevel,
            66 => WeatherSevereType::Ashfall,
            67 => WeatherSevereType::FreezingFog,
            68 => WeatherSevereType::DenseFog,
            69 => WeatherSevereType::DenseSmoke,
            70 => WeatherSevereType::BlowingDust,
            71 => WeatherSevereType::HardFreeze,
            72 => WeatherSevereType::Freeze,
            73 => WeatherSevereType::Frost,
            74 => WeatherSevereType::FireWeather,
            75 => WeatherSevereType::Flood,
            76 => WeatherSevereType::RipTide,
            77 => WeatherSevereType::HighSurf,
            78 => WeatherSevereType::Smog,
            79 => WeatherSevereType::AirQuality,
            80 => WeatherSevereType::BriskWind,
            81 => WeatherSevereType::AirStagnation,
            82 => WeatherSevereType::LowWater,
            83 => WeatherSevereType::Hydrological,
            84 => WeatherSevereType::SpecialWeather,
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
    Other,
    Serve,
    Forehand,
    Backhand,
    Smash,
    UnknownVariant,
}
impl StrokeType {
    pub fn from(content: u8) -> StrokeType {
        match content {
            0 => StrokeType::NoEvent,
            1 => StrokeType::Other,
            2 => StrokeType::Serve,
            3 => StrokeType::Forehand,
            4 => StrokeType::Backhand,
            5 => StrokeType::Smash,
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
    LeftLeg,
    LeftCalf,
    LeftShin,
    LeftHamstring,
    LeftQuad,
    LeftGlute,
    RightLeg,
    RightCalf,
    RightShin,
    RightHamstring,
    RightQuad,
    RightGlute,
    TorsoBack,
    LeftLowerBack,
    LeftUpperBack,
    RightLowerBack,
    RightUpperBack,
    TorsoFront,
    LeftAbdomen,
    LeftChest,
    RightAbdomen,
    RightChest,
    LeftArm,
    LeftShoulder,
    LeftBicep,
    LeftTricep,
    LeftBrachioradialis,
    LeftForearmExtensors,
    RightArm,
    RightShoulder,
    RightBicep,
    RightTricep,
    RightBrachioradialis,
    RightForearmExtensors,
    Neck,
    Throat,
    WaistMidBack,
    WaistFront,
    WaistLeft,
    WaistRight,
    UnknownVariant,
}
impl BodyLocation {
    pub fn from(content: u8) -> BodyLocation {
        match content {
            0 => BodyLocation::LeftLeg,
            1 => BodyLocation::LeftCalf,
            2 => BodyLocation::LeftShin,
            3 => BodyLocation::LeftHamstring,
            4 => BodyLocation::LeftQuad,
            5 => BodyLocation::LeftGlute,
            6 => BodyLocation::RightLeg,
            7 => BodyLocation::RightCalf,
            8 => BodyLocation::RightShin,
            9 => BodyLocation::RightHamstring,
            10 => BodyLocation::RightQuad,
            11 => BodyLocation::RightGlute,
            12 => BodyLocation::TorsoBack,
            13 => BodyLocation::LeftLowerBack,
            14 => BodyLocation::LeftUpperBack,
            15 => BodyLocation::RightLowerBack,
            16 => BodyLocation::RightUpperBack,
            17 => BodyLocation::TorsoFront,
            18 => BodyLocation::LeftAbdomen,
            19 => BodyLocation::LeftChest,
            20 => BodyLocation::RightAbdomen,
            21 => BodyLocation::RightChest,
            22 => BodyLocation::LeftArm,
            23 => BodyLocation::LeftShoulder,
            24 => BodyLocation::LeftBicep,
            25 => BodyLocation::LeftTricep,
            26 => BodyLocation::LeftBrachioradialis,
            27 => BodyLocation::LeftForearmExtensors,
            28 => BodyLocation::RightArm,
            29 => BodyLocation::RightShoulder,
            30 => BodyLocation::RightBicep,
            31 => BodyLocation::RightTricep,
            32 => BodyLocation::RightBrachioradialis,
            33 => BodyLocation::RightForearmExtensors,
            34 => BodyLocation::Neck,
            35 => BodyLocation::Throat,
            36 => BodyLocation::WaistMidBack,
            37 => BodyLocation::WaistFront,
            38 => BodyLocation::WaistLeft,
            39 => BodyLocation::WaistRight,
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
    Overall,
    PersonalBest,
    Connections,
    Group,
    Challenger,
    Kom,
    Qom,
    Pr,
    Goal,
    Carrot,
    ClubLeader,
    Rival,
    Last,
    RecentBest,
    CourseRecord,
    UnknownVariant,
}
impl SegmentLeaderboardType {
    pub fn from(content: u8) -> SegmentLeaderboardType {
        match content {
            0 => SegmentLeaderboardType::Overall,
            1 => SegmentLeaderboardType::PersonalBest,
            2 => SegmentLeaderboardType::Connections,
            3 => SegmentLeaderboardType::Group,
            4 => SegmentLeaderboardType::Challenger,
            5 => SegmentLeaderboardType::Kom,
            6 => SegmentLeaderboardType::Qom,
            7 => SegmentLeaderboardType::Pr,
            8 => SegmentLeaderboardType::Goal,
            9 => SegmentLeaderboardType::Carrot,
            10 => SegmentLeaderboardType::ClubLeader,
            11 => SegmentLeaderboardType::Rival,
            12 => SegmentLeaderboardType::Last,
            13 => SegmentLeaderboardType::RecentBest,
            14 => SegmentLeaderboardType::CourseRecord,
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
    Starred,
    Suggested,
    UnknownVariant,
}
impl SegmentSelectionType {
    pub fn from(content: u8) -> SegmentSelectionType {
        match content {
            0 => SegmentSelectionType::Starred,
            1 => SegmentSelectionType::Suggested,
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
    Antplus,
    Bluetooth,
    BluetoothLowEnergy,
    Wifi,
    Local,
    UnknownVariant,
}
impl SourceType {
    pub fn from(content: u8) -> SourceType {
        match content {
            0 => SourceType::Ant,
            1 => SourceType::Antplus,
            2 => SourceType::Bluetooth,
            3 => SourceType::BluetoothLowEnergy,
            4 => SourceType::Wifi,
            5 => SourceType::Local,
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
    AntExtendedDeviceNumberUpperNibble,
    AntTransmissionTypeLowerNibble,
    AntDeviceType,
    AntDeviceNumber,
    UnknownVariant,
}
impl AntChannelId {
    pub fn from(content: u32) -> AntChannelId {
        match content {
            4026531840 => AntChannelId::AntExtendedDeviceNumberUpperNibble,
            251658240 => AntChannelId::AntTransmissionTypeLowerNibble,
            16711680 => AntChannelId::AntDeviceType,
            65535 => AntChannelId::AntDeviceNumber,
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
    Portrait,
    Landscape,
    PortraitFlipped,
    LandscapeFlipped,
    UnknownVariant,
}
impl DisplayOrientation {
    pub fn from(content: u8) -> DisplayOrientation {
        match content {
            0 => DisplayOrientation::Auto,
            1 => DisplayOrientation::Portrait,
            2 => DisplayOrientation::Landscape,
            3 => DisplayOrientation::PortraitFlipped,
            4 => DisplayOrientation::LandscapeFlipped,
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
    None,
    SwimFins,
    SwimKickboard,
    SwimPaddles,
    SwimPullBuoy,
    SwimSnorkel,
    UnknownVariant,
}
impl WorkoutEquipment {
    pub fn from(content: u8) -> WorkoutEquipment {
        match content {
            0 => WorkoutEquipment::None,
            1 => WorkoutEquipment::SwimFins,
            2 => WorkoutEquipment::SwimKickboard,
            3 => WorkoutEquipment::SwimPaddles,
            4 => WorkoutEquipment::SwimPullBuoy,
            5 => WorkoutEquipment::SwimSnorkel,
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
    Digital,
    Analog,
    ConnectIq,
    Disabled,
    UnknownVariant,
}
impl WatchfaceMode {
    pub fn from(content: u8) -> WatchfaceMode {
        match content {
            0 => WatchfaceMode::Digital,
            1 => WatchfaceMode::Analog,
            2 => WatchfaceMode::ConnectIq,
            3 => WatchfaceMode::Disabled,
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
    VideoSplit,
    VideoEnd,
    PhotoTaken,
    VideoSecondStreamStart,
    VideoSecondStreamSplit,
    VideoSecondStreamEnd,
    VideoSplitStart,
    VideoSecondStreamSplitStart,
    VideoPause,
    VideoSecondStreamPause,
    VideoResume,
    VideoSecondStreamResume,
    UnknownVariant,
}
impl CameraEventType {
    pub fn from(content: u8) -> CameraEventType {
        match content {
            0 => CameraEventType::VideoStart,
            1 => CameraEventType::VideoSplit,
            2 => CameraEventType::VideoEnd,
            3 => CameraEventType::PhotoTaken,
            4 => CameraEventType::VideoSecondStreamStart,
            5 => CameraEventType::VideoSecondStreamSplit,
            6 => CameraEventType::VideoSecondStreamEnd,
            7 => CameraEventType::VideoSplitStart,
            8 => CameraEventType::VideoSecondStreamSplitStart,
            11 => CameraEventType::VideoPause,
            12 => CameraEventType::VideoSecondStreamPause,
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
    Compass,
    Barometer,
    UnknownVariant,
}
impl SensorType {
    pub fn from(content: u8) -> SensorType {
        match content {
            0 => SensorType::Accelerometer,
            1 => SensorType::Gyroscope,
            2 => SensorType::Compass,
            3 => SensorType::Barometer,
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
    CameraOrientation90,
    CameraOrientation180,
    CameraOrientation270,
    UnknownVariant,
}
impl CameraOrientationType {
    pub fn from(content: u8) -> CameraOrientationType {
        match content {
            0 => CameraOrientationType::CameraOrientation0,
            1 => CameraOrientationType::CameraOrientation90,
            2 => CameraOrientationType::CameraOrientation180,
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
    Failed,
    Aligning,
    Degraded,
    Valid,
    UnknownVariant,
}
impl AttitudeStage {
    pub fn from(content: u8) -> AttitudeStage {
        match content {
            0 => AttitudeStage::Failed,
            1 => AttitudeStage::Aligning,
            2 => AttitudeStage::Degraded,
            3 => AttitudeStage::Valid,
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
    TrackAngleHeadingValid,
    PitchValid,
    RollValid,
    LateralBodyAccelValid,
    NormalBodyAccelValid,
    TurnRateValid,
    HwFail,
    MagInvalid,
    NoGps,
    GpsInvalid,
    SolutionCoasting,
    TrueTrackAngle,
    MagneticHeading,
    UnknownVariant,
}
impl AttitudeValidity {
    pub fn from(content: u16) -> AttitudeValidity {
        match content {
            1 => AttitudeValidity::TrackAngleHeadingValid,
            2 => AttitudeValidity::PitchValid,
            4 => AttitudeValidity::RollValid,
            8 => AttitudeValidity::LateralBodyAccelValid,
            16 => AttitudeValidity::NormalBodyAccelValid,
            32 => AttitudeValidity::TurnRateValid,
            64 => AttitudeValidity::HwFail,
            128 => AttitudeValidity::MagInvalid,
            256 => AttitudeValidity::NoGps,
            512 => AttitudeValidity::GpsInvalid,
            1024 => AttitudeValidity::SolutionCoasting,
            2048 => AttitudeValidity::TrueTrackAngle,
            4096 => AttitudeValidity::MagneticHeading,
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
    Never,
    Occasionally,
    Frequent,
    OnceADay,
    Remote,
    UnknownVariant,
}
impl AutoSyncFrequency {
    pub fn from(content: u8) -> AutoSyncFrequency {
        match content {
            0 => AutoSyncFrequency::Never,
            1 => AutoSyncFrequency::Occasionally,
            2 => AutoSyncFrequency::Frequent,
            3 => AutoSyncFrequency::OnceADay,
            4 => AutoSyncFrequency::Remote,
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
    FullScreen,
    HalfVertical,
    HalfHorizontal,
    HalfVerticalRightSplit,
    HalfHorizontalBottomSplit,
    FullQuarterSplit,
    HalfVerticalLeftSplit,
    HalfHorizontalTopSplit,
    Dynamic,
    UnknownVariant,
}
impl ExdLayout {
    pub fn from(content: u8) -> ExdLayout {
        match content {
            0 => ExdLayout::FullScreen,
            1 => ExdLayout::HalfVertical,
            2 => ExdLayout::HalfHorizontal,
            3 => ExdLayout::HalfVerticalRightSplit,
            4 => ExdLayout::HalfHorizontalBottomSplit,
            5 => ExdLayout::FullQuarterSplit,
            6 => ExdLayout::HalfVerticalLeftSplit,
            7 => ExdLayout::HalfHorizontalTopSplit,
            8 => ExdLayout::Dynamic,
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
    Numerical,
    Simple,
    Graph,
    Bar,
    CircleGraph,
    VirtualPartner,
    Balance,
    StringList,
    String,
    SimpleDynamicIcon,
    Gauge,
    UnknownVariant,
}
impl ExdDisplayType {
    pub fn from(content: u8) -> ExdDisplayType {
        match content {
            0 => ExdDisplayType::Numerical,
            1 => ExdDisplayType::Simple,
            2 => ExdDisplayType::Graph,
            3 => ExdDisplayType::Bar,
            4 => ExdDisplayType::CircleGraph,
            5 => ExdDisplayType::VirtualPartner,
            6 => ExdDisplayType::Balance,
            7 => ExdDisplayType::StringList,
            8 => ExdDisplayType::String,
            9 => ExdDisplayType::SimpleDynamicIcon,
            10 => ExdDisplayType::Gauge,
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
    NoUnits,
    Laps,
    MilesPerHour,
    KilometersPerHour,
    FeetPerHour,
    MetersPerHour,
    DegreesCelsius,
    DegreesFarenheit,
    Zone,
    Gear,
    Rpm,
    Bpm,
    Degrees,
    Millimeters,
    Meters,
    Kilometers,
    Feet,
    Yards,
    Kilofeet,
    Miles,
    Time,
    EnumTurnType,
    Percent,
    Watts,
    WattsPerKilogram,
    EnumBatteryStatus,
    EnumBikeLightBeamAngleMode,
    EnumBikeLightBatteryStatus,
    EnumBikeLightNetworkConfigType,
    Lights,
    Seconds,
    Minutes,
    Hours,
    Calories,
    Kilojoules,
    Milliseconds,
    SecondPerMile,
    SecondPerKilometer,
    Centimeter,
    EnumCoursePoint,
    Bradians,
    EnumSport,
    InchesHg,
    MmHg,
    Mbars,
    HectoPascals,
    FeetPerMin,
    MetersPerMin,
    MetersPerSec,
    EightCardinal,
    UnknownVariant,
}
impl ExdDataUnits {
    pub fn from(content: u8) -> ExdDataUnits {
        match content {
            0 => ExdDataUnits::NoUnits,
            1 => ExdDataUnits::Laps,
            2 => ExdDataUnits::MilesPerHour,
            3 => ExdDataUnits::KilometersPerHour,
            4 => ExdDataUnits::FeetPerHour,
            5 => ExdDataUnits::MetersPerHour,
            6 => ExdDataUnits::DegreesCelsius,
            7 => ExdDataUnits::DegreesFarenheit,
            8 => ExdDataUnits::Zone,
            9 => ExdDataUnits::Gear,
            10 => ExdDataUnits::Rpm,
            11 => ExdDataUnits::Bpm,
            12 => ExdDataUnits::Degrees,
            13 => ExdDataUnits::Millimeters,
            14 => ExdDataUnits::Meters,
            15 => ExdDataUnits::Kilometers,
            16 => ExdDataUnits::Feet,
            17 => ExdDataUnits::Yards,
            18 => ExdDataUnits::Kilofeet,
            19 => ExdDataUnits::Miles,
            20 => ExdDataUnits::Time,
            21 => ExdDataUnits::EnumTurnType,
            22 => ExdDataUnits::Percent,
            23 => ExdDataUnits::Watts,
            24 => ExdDataUnits::WattsPerKilogram,
            25 => ExdDataUnits::EnumBatteryStatus,
            26 => ExdDataUnits::EnumBikeLightBeamAngleMode,
            27 => ExdDataUnits::EnumBikeLightBatteryStatus,
            28 => ExdDataUnits::EnumBikeLightNetworkConfigType,
            29 => ExdDataUnits::Lights,
            30 => ExdDataUnits::Seconds,
            31 => ExdDataUnits::Minutes,
            32 => ExdDataUnits::Hours,
            33 => ExdDataUnits::Calories,
            34 => ExdDataUnits::Kilojoules,
            35 => ExdDataUnits::Milliseconds,
            36 => ExdDataUnits::SecondPerMile,
            37 => ExdDataUnits::SecondPerKilometer,
            38 => ExdDataUnits::Centimeter,
            39 => ExdDataUnits::EnumCoursePoint,
            40 => ExdDataUnits::Bradians,
            41 => ExdDataUnits::EnumSport,
            42 => ExdDataUnits::InchesHg,
            43 => ExdDataUnits::MmHg,
            44 => ExdDataUnits::Mbars,
            45 => ExdDataUnits::HectoPascals,
            46 => ExdDataUnits::FeetPerMin,
            47 => ExdDataUnits::MetersPerMin,
            48 => ExdDataUnits::MetersPerSec,
            49 => ExdDataUnits::EightCardinal,
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
    NoQualifier,
    Instantaneous,
    Average,
    Lap,
    Maximum,
    MaximumAverage,
    MaximumLap,
    LastLap,
    AverageLap,
    ToDestination,
    ToGo,
    ToNext,
    NextCoursePoint,
    Total,
    ThreeSecondAverage,
    TenSecondAverage,
    ThirtySecondAverage,
    PercentMaximum,
    PercentMaximumAverage,
    LapPercentMaximum,
    Elapsed,
    Sunrise,
    Sunset,
    ComparedToVirtualPartner,
    Maximum24h,
    Minimum24h,
    Minimum,
    First,
    Second,
    Third,
    Shifter,
    LastSport,
    Moving,
    Stopped,
    EstimatedTotal,
    Zone9,
    Zone8,
    Zone7,
    Zone6,
    Zone5,
    Zone4,
    Zone3,
    Zone2,
    Zone1,
    UnknownVariant,
}
impl ExdQualifiers {
    pub fn from(content: u8) -> ExdQualifiers {
        match content {
            0 => ExdQualifiers::NoQualifier,
            1 => ExdQualifiers::Instantaneous,
            2 => ExdQualifiers::Average,
            3 => ExdQualifiers::Lap,
            4 => ExdQualifiers::Maximum,
            5 => ExdQualifiers::MaximumAverage,
            6 => ExdQualifiers::MaximumLap,
            7 => ExdQualifiers::LastLap,
            8 => ExdQualifiers::AverageLap,
            9 => ExdQualifiers::ToDestination,
            10 => ExdQualifiers::ToGo,
            11 => ExdQualifiers::ToNext,
            12 => ExdQualifiers::NextCoursePoint,
            13 => ExdQualifiers::Total,
            14 => ExdQualifiers::ThreeSecondAverage,
            15 => ExdQualifiers::TenSecondAverage,
            16 => ExdQualifiers::ThirtySecondAverage,
            17 => ExdQualifiers::PercentMaximum,
            18 => ExdQualifiers::PercentMaximumAverage,
            19 => ExdQualifiers::LapPercentMaximum,
            20 => ExdQualifiers::Elapsed,
            21 => ExdQualifiers::Sunrise,
            22 => ExdQualifiers::Sunset,
            23 => ExdQualifiers::ComparedToVirtualPartner,
            24 => ExdQualifiers::Maximum24h,
            25 => ExdQualifiers::Minimum24h,
            26 => ExdQualifiers::Minimum,
            27 => ExdQualifiers::First,
            28 => ExdQualifiers::Second,
            29 => ExdQualifiers::Third,
            30 => ExdQualifiers::Shifter,
            31 => ExdQualifiers::LastSport,
            32 => ExdQualifiers::Moving,
            33 => ExdQualifiers::Stopped,
            34 => ExdQualifiers::EstimatedTotal,
            242 => ExdQualifiers::Zone9,
            243 => ExdQualifiers::Zone8,
            244 => ExdQualifiers::Zone7,
            245 => ExdQualifiers::Zone6,
            246 => ExdQualifiers::Zone5,
            247 => ExdQualifiers::Zone4,
            248 => ExdQualifiers::Zone3,
            249 => ExdQualifiers::Zone2,
            250 => ExdQualifiers::Zone1,
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
    BikeLightBatteryStatus,
    BeamAngleStatus,
    BateryLevel,
    LightNetworkMode,
    NumberLightsConnected,
    Cadence,
    Distance,
    EstimatedTimeOfArrival,
    Heading,
    Time,
    BatteryLevel,
    TrainerResistance,
    TrainerTargetPower,
    TimeSeated,
    TimeStanding,
    Elevation,
    Grade,
    Ascent,
    Descent,
    VerticalSpeed,
    Di2BatteryLevel,
    FrontGear,
    RearGear,
    GearRatio,
    HeartRate,
    HeartRateZone,
    TimeInHeartRateZone,
    HeartRateReserve,
    Calories,
    GpsAccuracy,
    GpsSignalStrength,
    Temperature,
    TimeOfDay,
    Balance,
    PedalSmoothness,
    Power,
    FunctionalThresholdPower,
    IntensityFactor,
    Work,
    PowerRatio,
    NormalizedPower,
    TrainingStressScore,
    TimeOnZone,
    Speed,
    Laps,
    Reps,
    WorkoutStep,
    CourseDistance,
    NavigationDistance,
    CourseEstimatedTimeOfArrival,
    NavigationEstimatedTimeOfArrival,
    CourseTime,
    NavigationTime,
    CourseHeading,
    NavigationHeading,
    PowerZone,
    TorqueEffectiveness,
    TimerTime,
    PowerWeightRatio,
    LeftPlatformCenterOffset,
    RightPlatformCenterOffset,
    LeftPowerPhaseStartAngle,
    RightPowerPhaseStartAngle,
    LeftPowerPhaseFinishAngle,
    RightPowerPhaseFinishAngle,
    Gears,
    Pace,
    TrainingEffect,
    VerticalOscillation,
    VerticalRatio,
    GroundContactTime,
    LeftGroundContactTimeBalance,
    RightGroundContactTimeBalance,
    StrideLength,
    RunningCadence,
    PerformanceCondition,
    CourseType,
    TimeInPowerZone,
    NavigationTurn,
    CourseLocation,
    NavigationLocation,
    Compass,
    GearCombo,
    MuscleOxygen,
    Icon,
    CompassHeading,
    GpsHeading,
    GpsElevation,
    AnaerobicTrainingEffect,
    Course,
    OffCourse,
    GlideRatio,
    VerticalDistance,
    Vmg,
    AmbientPressure,
    Pressure,
    Vam,
    UnknownVariant,
}
impl ExdDescriptors {
    pub fn from(content: u8) -> ExdDescriptors {
        match content {
            0 => ExdDescriptors::BikeLightBatteryStatus,
            1 => ExdDescriptors::BeamAngleStatus,
            2 => ExdDescriptors::BateryLevel,
            3 => ExdDescriptors::LightNetworkMode,
            4 => ExdDescriptors::NumberLightsConnected,
            5 => ExdDescriptors::Cadence,
            6 => ExdDescriptors::Distance,
            7 => ExdDescriptors::EstimatedTimeOfArrival,
            8 => ExdDescriptors::Heading,
            9 => ExdDescriptors::Time,
            10 => ExdDescriptors::BatteryLevel,
            11 => ExdDescriptors::TrainerResistance,
            12 => ExdDescriptors::TrainerTargetPower,
            13 => ExdDescriptors::TimeSeated,
            14 => ExdDescriptors::TimeStanding,
            15 => ExdDescriptors::Elevation,
            16 => ExdDescriptors::Grade,
            17 => ExdDescriptors::Ascent,
            18 => ExdDescriptors::Descent,
            19 => ExdDescriptors::VerticalSpeed,
            20 => ExdDescriptors::Di2BatteryLevel,
            21 => ExdDescriptors::FrontGear,
            22 => ExdDescriptors::RearGear,
            23 => ExdDescriptors::GearRatio,
            24 => ExdDescriptors::HeartRate,
            25 => ExdDescriptors::HeartRateZone,
            26 => ExdDescriptors::TimeInHeartRateZone,
            27 => ExdDescriptors::HeartRateReserve,
            28 => ExdDescriptors::Calories,
            29 => ExdDescriptors::GpsAccuracy,
            30 => ExdDescriptors::GpsSignalStrength,
            31 => ExdDescriptors::Temperature,
            32 => ExdDescriptors::TimeOfDay,
            33 => ExdDescriptors::Balance,
            34 => ExdDescriptors::PedalSmoothness,
            35 => ExdDescriptors::Power,
            36 => ExdDescriptors::FunctionalThresholdPower,
            37 => ExdDescriptors::IntensityFactor,
            38 => ExdDescriptors::Work,
            39 => ExdDescriptors::PowerRatio,
            40 => ExdDescriptors::NormalizedPower,
            41 => ExdDescriptors::TrainingStressScore,
            42 => ExdDescriptors::TimeOnZone,
            43 => ExdDescriptors::Speed,
            44 => ExdDescriptors::Laps,
            45 => ExdDescriptors::Reps,
            46 => ExdDescriptors::WorkoutStep,
            47 => ExdDescriptors::CourseDistance,
            48 => ExdDescriptors::NavigationDistance,
            49 => ExdDescriptors::CourseEstimatedTimeOfArrival,
            50 => ExdDescriptors::NavigationEstimatedTimeOfArrival,
            51 => ExdDescriptors::CourseTime,
            52 => ExdDescriptors::NavigationTime,
            53 => ExdDescriptors::CourseHeading,
            54 => ExdDescriptors::NavigationHeading,
            55 => ExdDescriptors::PowerZone,
            56 => ExdDescriptors::TorqueEffectiveness,
            57 => ExdDescriptors::TimerTime,
            58 => ExdDescriptors::PowerWeightRatio,
            59 => ExdDescriptors::LeftPlatformCenterOffset,
            60 => ExdDescriptors::RightPlatformCenterOffset,
            61 => ExdDescriptors::LeftPowerPhaseStartAngle,
            62 => ExdDescriptors::RightPowerPhaseStartAngle,
            63 => ExdDescriptors::LeftPowerPhaseFinishAngle,
            64 => ExdDescriptors::RightPowerPhaseFinishAngle,
            65 => ExdDescriptors::Gears,
            66 => ExdDescriptors::Pace,
            67 => ExdDescriptors::TrainingEffect,
            68 => ExdDescriptors::VerticalOscillation,
            69 => ExdDescriptors::VerticalRatio,
            70 => ExdDescriptors::GroundContactTime,
            71 => ExdDescriptors::LeftGroundContactTimeBalance,
            72 => ExdDescriptors::RightGroundContactTimeBalance,
            73 => ExdDescriptors::StrideLength,
            74 => ExdDescriptors::RunningCadence,
            75 => ExdDescriptors::PerformanceCondition,
            76 => ExdDescriptors::CourseType,
            77 => ExdDescriptors::TimeInPowerZone,
            78 => ExdDescriptors::NavigationTurn,
            79 => ExdDescriptors::CourseLocation,
            80 => ExdDescriptors::NavigationLocation,
            81 => ExdDescriptors::Compass,
            82 => ExdDescriptors::GearCombo,
            83 => ExdDescriptors::MuscleOxygen,
            84 => ExdDescriptors::Icon,
            85 => ExdDescriptors::CompassHeading,
            86 => ExdDescriptors::GpsHeading,
            87 => ExdDescriptors::GpsElevation,
            88 => ExdDescriptors::AnaerobicTrainingEffect,
            89 => ExdDescriptors::Course,
            90 => ExdDescriptors::OffCourse,
            91 => ExdDescriptors::GlideRatio,
            92 => ExdDescriptors::VerticalDistance,
            93 => ExdDescriptors::Vmg,
            94 => ExdDescriptors::AmbientPressure,
            95 => ExdDescriptors::Pressure,
            96 => ExdDescriptors::Vam,
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
    None,
    Running,
    Cycling,
    Swimming,
    Walking,
    Elliptical,
    Sedentary,
    UnknownVariant,
}
impl AutoActivityDetect {
    pub fn from(content: u32) -> AutoActivityDetect {
        match content {
            0 => AutoActivityDetect::None,
            1 => AutoActivityDetect::Running,
            2 => AutoActivityDetect::Cycling,
            4 => AutoActivityDetect::Swimming,
            8 => AutoActivityDetect::Walking,
            32 => AutoActivityDetect::Elliptical,
            1024 => AutoActivityDetect::Sedentary,
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
    UnknownVariant,
}
impl FitBaseType {
    pub fn from(content: u8) -> FitBaseType {
        match content {
            0 => FitBaseType::Enum,
            1 => FitBaseType::Sint8,
            2 => FitBaseType::Uint8,
            131 => FitBaseType::Sint16,
            132 => FitBaseType::Uint16,
            133 => FitBaseType::Sint32,
            134 => FitBaseType::Uint32,
            7 => FitBaseType::String,
            136 => FitBaseType::Float32,
            137 => FitBaseType::Float64,
            10 => FitBaseType::Uint8z,
            139 => FitBaseType::Uint16z,
            140 => FitBaseType::Uint32z,
            13 => FitBaseType::Byte,
            142 => FitBaseType::Sint64,
            143 => FitBaseType::Uint64,
            144 => FitBaseType::Uint64z,
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
    Other,
    Kilogram,
    Pound,
    UnknownVariant,
}
impl FitBaseUnit {
    pub fn from(content: u16) -> FitBaseUnit {
        match content {
            0 => FitBaseUnit::Other,
            1 => FitBaseUnit::Kilogram,
            2 => FitBaseUnit::Pound,
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
    Rest,
    Active,
    UnknownVariant,
}
impl SetType {
    pub fn from(content: u8) -> SetType {
        match content {
            0 => SetType::Rest,
            1 => SetType::Active,
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
    BenchPress,
    CalfRaise,
    Cardio,
    Carry,
    Chop,
    Core,
    Crunch,
    Curl,
    Deadlift,
    Flye,
    HipRaise,
    HipStability,
    HipSwing,
    Hyperextension,
    LateralRaise,
    LegCurl,
    LegRaise,
    Lunge,
    OlympicLift,
    Plank,
    Plyo,
    PullUp,
    PushUp,
    Row,
    ShoulderPress,
    ShoulderStability,
    Shrug,
    SitUp,
    Squat,
    TotalBody,
    TricepsExtension,
    WarmUp,
    Run,
    Bike,
    CardioSensors,
    Move,
    Pose,
    BandedExercises,
    BattleRope,
    Elliptical,
    FloorClimb,
    IndoorBike,
    IndoorRow,
    Ladder,
    Sandbag,
    Sled,
    SledgeHammer,
    StairStepper,
    Suspension,
    Tire,
    RunIndoor,
    BikeOutdoor,
    Unknown,
    UnknownVariant,
}
impl ExerciseCategory {
    pub fn from(content: u16) -> ExerciseCategory {
        match content {
            0 => ExerciseCategory::BenchPress,
            1 => ExerciseCategory::CalfRaise,
            2 => ExerciseCategory::Cardio,
            3 => ExerciseCategory::Carry,
            4 => ExerciseCategory::Chop,
            5 => ExerciseCategory::Core,
            6 => ExerciseCategory::Crunch,
            7 => ExerciseCategory::Curl,
            8 => ExerciseCategory::Deadlift,
            9 => ExerciseCategory::Flye,
            10 => ExerciseCategory::HipRaise,
            11 => ExerciseCategory::HipStability,
            12 => ExerciseCategory::HipSwing,
            13 => ExerciseCategory::Hyperextension,
            14 => ExerciseCategory::LateralRaise,
            15 => ExerciseCategory::LegCurl,
            16 => ExerciseCategory::LegRaise,
            17 => ExerciseCategory::Lunge,
            18 => ExerciseCategory::OlympicLift,
            19 => ExerciseCategory::Plank,
            20 => ExerciseCategory::Plyo,
            21 => ExerciseCategory::PullUp,
            22 => ExerciseCategory::PushUp,
            23 => ExerciseCategory::Row,
            24 => ExerciseCategory::ShoulderPress,
            25 => ExerciseCategory::ShoulderStability,
            26 => ExerciseCategory::Shrug,
            27 => ExerciseCategory::SitUp,
            28 => ExerciseCategory::Squat,
            29 => ExerciseCategory::TotalBody,
            30 => ExerciseCategory::TricepsExtension,
            31 => ExerciseCategory::WarmUp,
            32 => ExerciseCategory::Run,
            33 => ExerciseCategory::Bike,
            34 => ExerciseCategory::CardioSensors,
            35 => ExerciseCategory::Move,
            36 => ExerciseCategory::Pose,
            37 => ExerciseCategory::BandedExercises,
            38 => ExerciseCategory::BattleRope,
            39 => ExerciseCategory::Elliptical,
            40 => ExerciseCategory::FloorClimb,
            41 => ExerciseCategory::IndoorBike,
            42 => ExerciseCategory::IndoorRow,
            43 => ExerciseCategory::Ladder,
            44 => ExerciseCategory::Sandbag,
            45 => ExerciseCategory::Sled,
            46 => ExerciseCategory::SledgeHammer,
            47 => ExerciseCategory::StairStepper,
            49 => ExerciseCategory::Suspension,
            50 => ExerciseCategory::Tire,
            52 => ExerciseCategory::RunIndoor,
            53 => ExerciseCategory::BikeOutdoor,
            65534 => ExerciseCategory::Unknown,
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
    Fresh,
    Salt,
    En13319,
    Custom,
    UnknownVariant,
}
impl WaterType {
    pub fn from(content: u8) -> WaterType {
        match content {
            0 => WaterType::Fresh,
            1 => WaterType::Salt,
            2 => WaterType::En13319,
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
    Disabled,
    Enabled,
    BackupOnly,
    UnknownVariant,
}
impl DiveGasStatus {
    pub fn from(content: u8) -> DiveGasStatus {
        match content {
            0 => DiveGasStatus::Disabled,
            1 => DiveGasStatus::Enabled,
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
    Depth,
    Time,
    Speed,
    UnknownVariant,
}
impl DiveAlarmType {
    pub fn from(content: u8) -> DiveAlarmType {
        match content {
            0 => DiveAlarmType::Depth,
            1 => DiveAlarmType::Time,
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
    Unmeasurable,
    Awake,
    Light,
    Deep,
    Rem,
    UnknownVariant,
}
impl SleepLevel {
    pub fn from(content: u8) -> SleepLevel {
        match content {
            0 => SleepLevel::Unmeasurable,
            1 => SleepLevel::Awake,
            2 => SleepLevel::Light,
            3 => SleepLevel::Deep,
            4 => SleepLevel::Rem,
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
    OffWrist,
    SpotCheck,
    ContinuousCheck,
    Periodic,
    UnknownVariant,
}
impl Spo2MeasurementType {
    pub fn from(content: u8) -> Spo2MeasurementType {
        match content {
            0 => Spo2MeasurementType::OffWrist,
            1 => Spo2MeasurementType::SpotCheck,
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
    Manual,
    Automatic,
    UnknownVariant,
}
impl CcrSetpointSwitchMode {
    pub fn from(content: u8) -> CcrSetpointSwitchMode {
        match content {
            0 => CcrSetpointSwitchMode::Manual,
            1 => CcrSetpointSwitchMode::Automatic,
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
    Arrow,
    RifleCartridge,
    PistolCartridge,
    Shotshell,
    AirRiflePellet,
    Other,
    UnknownVariant,
}
impl ProjectileType {
    pub fn from(content: u8) -> ProjectileType {
        match content {
            0 => ProjectileType::Arrow,
            1 => ProjectileType::RifleCartridge,
            2 => ProjectileType::PistolCartridge,
            3 => ProjectileType::Shotshell,
            4 => ProjectileType::AirRiflePellet,
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
    AscentSplit,
    DescentSplit,
    IntervalActive,
    IntervalRest,
    IntervalWarmup,
    IntervalCooldown,
    IntervalRecovery,
    IntervalOther,
    ClimbActive,
    ClimbRest,
    SurfActive,
    RunActive,
    RunRest,
    WorkoutRound,
    RwdRun,
    RwdWalk,
    WindsurfActive,
    RwdStand,
    Transition,
    SkiLiftSplit,
    SkiRunSplit,
    UnknownVariant,
}
impl SplitType {
    pub fn from(content: u8) -> SplitType {
        match content {
            1 => SplitType::AscentSplit,
            2 => SplitType::DescentSplit,
            3 => SplitType::IntervalActive,
            4 => SplitType::IntervalRest,
            5 => SplitType::IntervalWarmup,
            6 => SplitType::IntervalCooldown,
            7 => SplitType::IntervalRecovery,
            8 => SplitType::IntervalOther,
            9 => SplitType::ClimbActive,
            10 => SplitType::ClimbRest,
            11 => SplitType::SurfActive,
            12 => SplitType::RunActive,
            13 => SplitType::RunRest,
            14 => SplitType::WorkoutRound,
            17 => SplitType::RwdRun,
            18 => SplitType::RwdWalk,
            21 => SplitType::WindsurfActive,
            22 => SplitType::RwdStand,
            23 => SplitType::Transition,
            28 => SplitType::SkiLiftSplit,
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
    Approach,
    Start,
    Complete,
    UnknownVariant,
}
impl ClimbProEvent {
    pub fn from(content: u8) -> ClimbProEvent {
        match content {
            0 => ClimbProEvent::Approach,
            1 => ClimbProEvent::Start,
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
    PressureSac,
    VolumeSac,
    Rmv,
    UnknownVariant,
}
impl GasConsumptionRateType {
    pub fn from(content: u8) -> GasConsumptionRateType {
        match content {
            0 => GasConsumptionRateType::PressureSac,
            1 => GasConsumptionRateType::VolumeSac,
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
    High,
    Medium,
    Low,
    UnknownVariant,
}
impl TapSensitivity {
    pub fn from(content: u8) -> TapSensitivity {
        match content {
            0 => TapSensitivity::High,
            1 => TapSensitivity::Medium,
            2 => TapSensitivity::Low,
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
    ThreatUnknown,
    ThreatNone,
    ThreatApproaching,
    ThreatApproachingFast,
    UnknownVariant,
}
impl RadarThreatLevelType {
    pub fn from(content: u8) -> RadarThreatLevelType {
        match content {
            0 => RadarThreatLevelType::ThreatUnknown,
            1 => RadarThreatLevelType::ThreatNone,
            2 => RadarThreatLevelType::ThreatApproaching,
            3 => RadarThreatLevelType::ThreatApproachingFast,
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
    OnboardGps,
    ConnectedGps,
    Cadence,
    UnknownVariant,
}
impl MaxMetSpeedSource {
    pub fn from(content: u8) -> MaxMetSpeedSource {
        match content {
            0 => MaxMetSpeedSource::OnboardGps,
            1 => MaxMetSpeedSource::ConnectedGps,
            2 => MaxMetSpeedSource::Cadence,
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
    None,
    Poor,
    Low,
    Unbalanced,
    Balanced,
    UnknownVariant,
}
impl HrvStatus {
    pub fn from(content: u8) -> HrvStatus {
        match content {
            0 => HrvStatus::None,
            1 => HrvStatus::Poor,
            2 => HrvStatus::Low,
            3 => HrvStatus::Unbalanced,
            4 => HrvStatus::Balanced,
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
    Standard,
    Flat24Hours,
    UnknownVariant,
}
impl NoFlyTimeMode {
    pub fn from(content: u8) -> NoFlyTimeMode {
        match content {
            0 => NoFlyTimeMode::Standard,
            1 => NoFlyTimeMode::Flat24Hours,
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
