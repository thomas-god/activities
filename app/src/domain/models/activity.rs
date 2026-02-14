use std::{
    collections::HashMap,
    fmt::{self},
    hash::Hash,
    str::FromStr,
};

use chrono::{DateTime, FixedOffset};
use derive_more::{AsRef, Constructor, Display, From, Into};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::models::UserId;

///////////////////////////////////////////////////////////////////
/// ACTIVITY
///////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct Activity {
    id: ActivityId,
    user: UserId,
    name: Option<ActivityName>,
    start_time: ActivityStartTime,
    sport: Sport,
    statistics: ActivityStatistics,
    rpe: Option<ActivityRpe>,
    workout_type: Option<WorkoutType>,
    nutrition: Option<ActivityNutrition>,
    feedback: Option<ActivityFeedback>,
}

#[allow(clippy::too_many_arguments)]
/// An [Activity] is an entity representing a single sport activity or training session.
impl Activity {
    pub fn new(
        id: ActivityId,
        user: UserId,
        name: Option<ActivityName>,
        start_time: ActivityStartTime,
        sport: Sport,
        statistics: ActivityStatistics,
        rpe: Option<ActivityRpe>,
        workout_type: Option<WorkoutType>,
        nutrition: Option<ActivityNutrition>,
        feedback: Option<ActivityFeedback>,
    ) -> Self {
        Self {
            id,
            user,
            name,
            start_time,
            sport,
            statistics,
            rpe,
            workout_type,
            nutrition,
            feedback,
        }
    }

    /// An [Activity]'s natural key if a key generated from its defining fields. Two activities with
    /// identical natural keys should be considered identical/duplicate regardless of their
    /// technical [Activity::id].
    pub fn natural_key(&self) -> ActivityNaturalKey {
        let duration = self
            .statistics
            .get(&ActivityStatistic::Duration)
            .unwrap_or(&0.);
        ActivityNaturalKey(format!(
            "{}:{}:{}:{}",
            self.user, self.sport, self.start_time, duration
        ))
    }

    pub fn id(&self) -> &ActivityId {
        &self.id
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn name(&self) -> Option<&ActivityName> {
        self.name.as_ref()
    }

    pub fn start_time(&self) -> &ActivityStartTime {
        &self.start_time
    }

    pub fn sport(&self) -> &Sport {
        &self.sport
    }

    pub fn statistics(&self) -> &ActivityStatistics {
        &self.statistics
    }

    pub fn rpe(&self) -> &Option<ActivityRpe> {
        &self.rpe
    }

    pub fn workout_type(&self) -> &Option<WorkoutType> {
        &self.workout_type
    }

    pub fn nutrition(&self) -> &Option<ActivityNutrition> {
        &self.nutrition
    }

    pub fn feedback(&self) -> &Option<ActivityFeedback> {
        &self.feedback
    }
}

#[derive(Clone, Debug, Constructor)]
pub struct ActivityWithTimeseries {
    activity: Activity,
    timeseries: ActivityTimeseries,
}

impl ActivityWithTimeseries {
    pub fn activity(&self) -> &Activity {
        &self.activity
    }

    pub fn id(&self) -> &ActivityId {
        self.activity.id()
    }

    pub fn natural_key(&self) -> ActivityNaturalKey {
        self.activity.natural_key()
    }

    pub fn user(&self) -> &UserId {
        self.activity.user()
    }

    pub fn name(&self) -> Option<&ActivityName> {
        self.activity.name()
    }

    pub fn start_time(&self) -> &ActivityStartTime {
        self.activity.start_time()
    }

    pub fn sport(&self) -> &Sport {
        self.activity.sport()
    }

    pub fn statistics(&self) -> &ActivityStatistics {
        self.activity.statistics()
    }

    pub fn rpe(&self) -> &Option<ActivityRpe> {
        &self.activity.rpe
    }

    pub fn workout_type(&self) -> &Option<WorkoutType> {
        &self.activity.workout_type
    }

    pub fn nutrition(&self) -> &Option<ActivityNutrition> {
        &self.activity.nutrition
    }

    pub fn feedback(&self) -> &Option<ActivityFeedback> {
        &self.activity.feedback
    }

    pub fn timeseries(&self) -> &ActivityTimeseries {
        &self.timeseries
    }
}

/// Technical ID of an [Activity].
#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Hash)]
pub struct ActivityId(String);

impl ActivityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl Default for ActivityId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Constructor)]
pub struct ActivityName(String);

impl fmt::Display for ActivityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ActivityName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Rate of Perceived Exertion (RPE) - a value from 1 to 10
#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ActivityRpe {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
}

impl ActivityRpe {
    pub fn range(&self) -> ActivityRpeRange {
        ActivityRpeRange::from(self)
    }
}

impl ActivityRpe {
    pub fn value(&self) -> u8 {
        *self as u8
    }
}

impl From<ActivityRpe> for u8 {
    fn from(rpe: ActivityRpe) -> Self {
        rpe as u8
    }
}

impl TryFrom<u8> for ActivityRpe {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ActivityRpe::One),
            2 => Ok(ActivityRpe::Two),
            3 => Ok(ActivityRpe::Three),
            4 => Ok(ActivityRpe::Four),
            5 => Ok(ActivityRpe::Five),
            6 => Ok(ActivityRpe::Six),
            7 => Ok(ActivityRpe::Seven),
            8 => Ok(ActivityRpe::Eight),
            9 => Ok(ActivityRpe::Nine),
            10 => Ok(ActivityRpe::Ten),
            _ => Err(format!(
                "Invalid RPE value: {}. Must be between 1 and 10",
                value
            )),
        }
    }
}

impl fmt::Display for ActivityRpe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivityRpeRange {
    Easy,
    Moderate,
    Hard,
    VeryHard,
    Maximum,
}

impl From<&ActivityRpe> for ActivityRpeRange {
    fn from(value: &ActivityRpe) -> Self {
        match *value {
            ActivityRpe::One | ActivityRpe::Two | ActivityRpe::Three => Self::Easy,
            ActivityRpe::Four | ActivityRpe::Five | ActivityRpe::Six => Self::Moderate,
            ActivityRpe::Seven | ActivityRpe::Eight => Self::Hard,
            ActivityRpe::Nine => Self::VeryHard,
            ActivityRpe::Ten => Self::Maximum,
        }
    }
}

impl fmt::Display for ActivityRpeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            Self::Easy => "easy",
            Self::Moderate => "moderate",
            Self::Hard => "hard",
            Self::VeryHard => "very_hard",
            Self::Maximum => "maximum",
        };
        write!(f, "{}", s)
    }
}

/// Workout type categorizes the nature of a training session.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkoutType {
    Easy,
    Tempo,
    Intervals,
    LongRun,
    Race,
    CrossTraining,
}

impl FromStr for WorkoutType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "easy" => Ok(WorkoutType::Easy),
            "tempo" => Ok(WorkoutType::Tempo),
            "intervals" => Ok(WorkoutType::Intervals),
            "long_run" | "longrun" => Ok(WorkoutType::LongRun),
            "race" => Ok(WorkoutType::Race),
            "cross_training" => Ok(WorkoutType::CrossTraining),
            _ => Err(format!(
                "Invalid training type: '{}'. Must be one of: easy, tempo, intervals, long_run, race",
                s
            )),
        }
    }
}

impl fmt::Display for WorkoutType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            WorkoutType::Easy => "easy",
            WorkoutType::Tempo => "tempo",
            WorkoutType::Intervals => "intervals",
            WorkoutType::LongRun => "long_run",
            WorkoutType::Race => "race",
            WorkoutType::CrossTraining => "cross_training",
        };
        write!(f, "{}", s)
    }
}

/// Bonk status indicates whether the athlete experienced a bonk (energy depletion) during the activity.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BonkStatus {
    /// No bonk occurred
    None,
    /// Bonk occurred - athlete experienced energy depletion
    Bonked,
}

impl FromStr for BonkStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(BonkStatus::None),
            "bonked" => Ok(BonkStatus::Bonked),
            _ => Err(format!(
                "Invalid bonk status: '{}'. Must be one of: none, bonked",
                s
            )),
        }
    }
}

impl fmt::Display for BonkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BonkStatus::None => "none",
            BonkStatus::Bonked => "bonked",
        };
        write!(f, "{}", s)
    }
}

/// Nutrition and hydration tracking for an activity.
/// Includes bonk status and optional details about nutrition/hydration intake.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Constructor, Serialize, Deserialize)]
pub struct ActivityNutrition {
    bonk_status: BonkStatus,
    details: Option<String>,
}

impl ActivityNutrition {
    pub fn bonk_status(&self) -> BonkStatus {
        self.bonk_status
    }

    pub fn details(&self) -> Option<&str> {
        self.details.as_deref()
    }
}

/// Free-form text feedback about an activity.
/// Can include subjective impressions, training notes, or any other commentary.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActivityFeedback(String);

impl fmt::Display for ActivityFeedback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ActivityFeedback {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for ActivityFeedback {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl ActivityFeedback {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActivityNaturalKey(String);

impl From<&str> for ActivityNaturalKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into, Copy)]
pub struct ActivityStartTime(DateTime<FixedOffset>);

impl ActivityStartTime {
    pub fn new(datetime: DateTime<FixedOffset>) -> Self {
        Self(datetime)
    }

    pub fn from_timestamp(timestamp: usize) -> Option<Self> {
        DateTime::from_timestamp(timestamp as i64, 0).map(|dt| Self(dt.fixed_offset()))
    }

    pub fn date(&self) -> &DateTime<FixedOffset> {
        &self.0
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Display, Serialize, Deserialize)]
pub enum Sport {
    Running,
    TrailRunning,
    IndoorRunning,
    TrackRunning,

    Walking,
    Hiking,
    Mountaineering,
    IndoorWalking,
    Snowshoeing,

    Cycling,
    IndoorCycling,
    MountainBiking,
    Cyclocross,
    TrackCycling,
    EBiking,
    GravelCycling,

    Rowing,
    IndoorRowing,

    Swimming,
    OpenWaterSwimming,

    StandUpPaddleboarding,
    Surfing,
    Wakeboarding,
    WaterSkiing,
    Windsurfing,
    Kitesurfing,
    Wakesurfing,
    Sailing,
    Snorkeling,

    Whitewater,
    Paddling,
    Kayaking,
    Rafting,

    AlpineSki,
    CrossCountrySkiing,
    Snowboarding,

    InlineSkating,

    Hiit,
    CardioTraining,
    StrengthTraining,
    Yoga,
    Pilates,

    Climbing,
    IndoorClimbing,
    Bouldering,

    Soccer,
    Baseball,
    Cricket,
    AmericanFootball,
    Basketball,
    Rugby,
    Hockey,
    Lacrosse,
    Volleyball,

    Racket,
    Tennis,
    Pickleball,
    Padel,
    Squash,
    Badminton,
    Racquetball,
    TableTennis,

    Boxing,
    MixedMartialArts,
    Golf,

    Other,
}

pub struct InvalidSport {}

impl FromStr for Sport {
    type Err = InvalidSport;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Running" => Ok(Self::Running),
            "TrailRunning" => Ok(Self::TrailRunning),
            "IndoorRunning" => Ok(Self::IndoorRunning),
            "TrackRunning" => Ok(Self::TrackRunning),

            "Walking" => Ok(Self::Walking),
            "Hiking" => Ok(Self::Hiking),
            "Mountaineering" => Ok(Self::Mountaineering),
            "IndoorWalking" => Ok(Self::IndoorWalking),
            "Snowshoeing" => Ok(Self::Snowshoeing),

            "Cycling" => Ok(Self::Cycling),
            "IndoorCycling" => Ok(Self::IndoorCycling),
            "MountainBiking" => Ok(Self::MountainBiking),
            "Cyclocross" => Ok(Self::Cyclocross),
            "TrackCycling" => Ok(Self::TrackCycling),
            "EBiking" => Ok(Self::EBiking),
            "GravelCycling" => Ok(Self::GravelCycling),

            "Rowing" => Ok(Self::Rowing),
            "IndoorRowing" => Ok(Self::IndoorRowing),

            "Swimming" => Ok(Self::Swimming),
            "OpenWaterSwimming" => Ok(Self::OpenWaterSwimming),

            "StandUpPaddleboarding" => Ok(Self::StandUpPaddleboarding),
            "Surfing" => Ok(Self::Surfing),
            "Wakeboarding" => Ok(Self::Wakeboarding),
            "WaterSkiing" => Ok(Self::WaterSkiing),
            "Windsurfing" => Ok(Self::Windsurfing),
            "Kitesurfing" => Ok(Self::Kitesurfing),
            "Wakesurfing" => Ok(Self::Wakesurfing),
            "Sailing" => Ok(Self::Sailing),
            "Snorkeling" => Ok(Self::Snorkeling),

            "Whitewater" => Ok(Self::Whitewater),
            "Paddling" => Ok(Self::Paddling),
            "Kayaking" => Ok(Self::Kayaking),
            "Rafting" => Ok(Self::Rafting),

            "AlpineSki" => Ok(Self::AlpineSki),
            "CrossCountrySkiing" => Ok(Self::CrossCountrySkiing),
            "Snowboarding" => Ok(Self::Snowboarding),

            "InlineSkating" => Ok(Self::InlineSkating),

            "Hiit" => Ok(Self::Hiit),
            "CardioTraining" => Ok(Self::CardioTraining),
            "StrengthTraining" => Ok(Self::StrengthTraining),
            "Yoga" => Ok(Self::Yoga),
            "Pilates" => Ok(Self::Pilates),

            "Climbing" => Ok(Self::Climbing),
            "IndoorClimbing" => Ok(Self::IndoorClimbing),
            "Bouldering" => Ok(Self::Bouldering),

            "Soccer" => Ok(Self::Soccer),
            "Baseball" => Ok(Self::Baseball),
            "Basketball" => Ok(Self::Basketball),
            "Rugby" => Ok(Self::Rugby),
            "Hockey" => Ok(Self::Hockey),
            "Lacrosse" => Ok(Self::Lacrosse),
            "Volleyball" => Ok(Self::Volleyball),
            "Cricket" => Ok(Self::Cricket),
            "AmericanFootball" => Ok(Self::AmericanFootball),

            "Racket" => Ok(Self::Racket),
            "Tennis" => Ok(Self::Tennis),
            "Pickleball" => Ok(Self::Pickleball),
            "Padel" => Ok(Self::Padel),
            "Squash" => Ok(Self::Squash),
            "Badminton" => Ok(Self::Badminton),
            "Racquetball" => Ok(Self::Racquetball),
            "TableTennis" => Ok(Self::TableTennis),

            "Boxing" => Ok(Self::Boxing),
            "MixedMartialArts" => Ok(Self::MixedMartialArts),
            "Golf" => Ok(Self::Golf),

            "Other" => Ok(Self::Other),
            _ => Err(InvalidSport {}),
        }
    }
}

impl Sport {
    pub fn category(&self) -> Option<SportCategory> {
        match self {
            Self::Running | Self::TrailRunning | Self::IndoorRunning | Self::TrackRunning => {
                Some(SportCategory::Running)
            }

            Self::Walking
            | Self::Hiking
            | Self::Mountaineering
            | Self::IndoorWalking
            | Self::Snowshoeing => Some(SportCategory::Walking),

            Self::Cycling
            | Self::IndoorCycling
            | Self::MountainBiking
            | Self::Cyclocross
            | Self::TrackCycling
            | Self::EBiking
            | Self::GravelCycling => Some(SportCategory::Cycling),

            Self::Rowing | Self::IndoorRowing => Some(SportCategory::Rowing),

            Self::Swimming | Self::OpenWaterSwimming => Some(SportCategory::Swimming),

            Self::StandUpPaddleboarding
            | Self::Surfing
            | Self::Wakeboarding
            | Self::WaterSkiing
            | Self::Windsurfing
            | Self::Kitesurfing
            | Self::Wakesurfing
            | Self::Sailing
            | Self::Snorkeling => Some(SportCategory::WaterSports),

            Self::Whitewater | Self::Paddling | Self::Kayaking | Self::Rafting => {
                Some(SportCategory::WaterSports)
            }

            Self::AlpineSki | Self::CrossCountrySkiing | Self::Snowboarding => {
                Some(SportCategory::Ski)
            }

            Self::InlineSkating => None,

            Self::Hiit
            | Self::CardioTraining
            | Self::StrengthTraining
            | Self::Yoga
            | Self::Pilates => Some(SportCategory::Cardio),

            Self::Climbing | Self::IndoorClimbing | Self::Bouldering => {
                Some(SportCategory::Climbing)
            }

            Self::Soccer
            | Self::Baseball
            | Self::Basketball
            | Self::Rugby
            | Self::Hockey
            | Self::Lacrosse
            | Self::Volleyball
            | Self::AmericanFootball
            | Self::Cricket => Some(SportCategory::TeamSports),

            Self::Racket
            | Self::Tennis
            | Self::Pickleball
            | Self::Padel
            | Self::Squash
            | Self::Badminton
            | Self::Racquetball
            | Self::TableTennis => Some(SportCategory::Racket),

            Self::Boxing => None,
            Self::MixedMartialArts => None,
            Self::Golf => None,

            Self::Other => None,
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Display, Serialize, Deserialize)]
pub enum SportCategory {
    Running,
    Cycling,
    Swimming,
    Walking,
    Rowing,
    WaterSports,
    Ski,
    Cardio,
    Climbing,
    TeamSports,
    Racket,
}

#[derive(Clone, Debug, Constructor, Default, Serialize, Deserialize, PartialEq)]
pub struct ActivityStatistics(HashMap<ActivityStatistic, f64>);

impl ActivityStatistics {
    pub fn get(&self, stat: &ActivityStatistic) -> Option<&f64> {
        self.0.get(stat)
    }

    pub fn items(&self) -> HashMap<String, f64> {
        HashMap::from_iter(
            self.0
                .iter()
                .map(|(stat, value)| (stat.to_string(), *value)),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum ActivityStatistic {
    Duration,
    Calories,
    Elevation,
    Distance,
    NormalizedPower,
}

impl ToUnit for ActivityStatistic {
    fn unit(&self) -> Unit {
        match self {
            Self::Duration => Unit::Second,
            Self::Calories => Unit::KiloCalorie,
            Self::Elevation => Unit::Meter,
            Self::Distance => Unit::Meter,
            Self::NormalizedPower => Unit::Watt,
        }
    }
}

/// Trait to represent the associated physical unit (e.g., meters, watt) of some value.
pub trait ToUnit {
    fn unit(&self) -> Unit;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    KiloCalorie,
    Meter,
    Kilometer,
    MeterPerSecond,
    KilometerPerHour,
    SecondPerMeter,
    SecondPerKilometer,
    Watt,
    BeatPerMinute,
    RevolutionPerMinute,
    Second,
    NumberOfActivities,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = match self {
            Self::KiloCalorie => "kcal",
            Self::Meter => "m",
            Self::Kilometer => "km",
            Self::MeterPerSecond => "m/s",
            Self::KilometerPerHour => "km/h",
            Self::SecondPerMeter => "s/m",
            Self::SecondPerKilometer => "s/km",
            Self::Watt => "W",
            Self::BeatPerMinute => "bpm",
            Self::RevolutionPerMinute => "rpm",
            Self::Second => "s",
            Self::NumberOfActivities => "activities",
        };

        write!(f, "{}", unit)
    }
}

///////////////////////////////////////////////////////////////////
// TIMESERIES
///////////////////////////////////////////////////////////////////

/// An [ActivityTimeseries] is a coherent set of time dependant [TimeseriesMetric] (plural)
/// from the same [Activity].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActivityTimeseries {
    time: TimeseriesTime,
    active_time: TimeseriesActiveTime,
    metrics: Vec<Timeseries>,
    laps: Vec<Lap>,
}

#[derive(Debug, Clone, Error)]
pub enum NewTimeseriesError {
    #[error("Different lengths for time/metrics")]
    InvalidLengths,
}

impl ActivityTimeseries {
    pub fn new(
        time: TimeseriesTime,
        active_time: TimeseriesActiveTime,
        laps: Vec<Lap>,
        metrics: Vec<Timeseries>,
    ) -> Result<Self, NewTimeseriesError> {
        if time.len() != active_time.len() {
            return Err(NewTimeseriesError::InvalidLengths);
        }

        if metrics.iter().any(|metric| metric.len() != time.len()) {
            return Err(NewTimeseriesError::InvalidLengths);
        }

        Ok(Self {
            time,
            active_time,
            laps,
            metrics,
        })
    }

    pub fn time(&self) -> &TimeseriesTime {
        &self.time
    }

    pub fn active_time(&self) -> &TimeseriesActiveTime {
        &self.active_time
    }

    pub fn laps(&self) -> &[Lap] {
        &self.laps
    }

    pub fn metrics(&self) -> &[Timeseries] {
        &self.metrics
    }
}

/// [TimeseriesTime] represents the relative timestamp of a timeseries, starting from the
/// [Activity::start_time]. This time is strictly increasing, i.e. event when the activity is paused.
#[derive(Debug, Clone, PartialEq, Constructor, AsRef, Default)]
pub struct TimeseriesTime(Vec<usize>);

impl TimeseriesTime {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn values(&self) -> &[usize] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveTime {
    Running(usize),
    Paused,
}

impl ActiveTime {
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }

    pub fn value(&self) -> Option<usize> {
        match self {
            Self::Paused => None,
            Self::Running(dt) => Some(*dt),
        }
    }
}

/// [TimeseriesTime] represents the active time of a timeseries, i.e. it does not increase
/// when the activity is paused.
#[derive(Debug, Clone, PartialEq, Constructor, Default)]
pub struct TimeseriesActiveTime(Vec<ActiveTime>);

impl TimeseriesActiveTime {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn values(&self) -> &[ActiveTime] {
        &self.0
    }
}

#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct Lap {
    start: usize,
    end: usize,
}

impl Lap {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct Timeseries {
    metric: TimeseriesMetric,
    values: Vec<Option<TimeseriesValue>>,
}

impl Timeseries {
    pub fn metric(&self) -> &TimeseriesMetric {
        &self.metric
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn values(&self) -> &[Option<TimeseriesValue>] {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum TimeseriesMetric {
    Speed,
    Power,
    HeartRate,
    Distance,
    Cadence,
    Altitude,
    Pace,
}

impl ToUnit for TimeseriesMetric {
    fn unit(&self) -> Unit {
        match self {
            Self::Distance => Unit::Meter,
            Self::Power => Unit::Watt,
            Self::HeartRate => Unit::BeatPerMinute,
            Self::Speed => Unit::MeterPerSecond,
            Self::Altitude => Unit::Meter,
            Self::Cadence => Unit::RevolutionPerMinute,
            Self::Pace => Unit::SecondPerMeter,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeseriesValue {
    Int(usize),
    Float(f64),
}

impl From<&TimeseriesValue> for f64 {
    fn from(value: &TimeseriesValue) -> Self {
        match value {
            TimeseriesValue::Int(val) => *val as f64,
            TimeseriesValue::Float(val) => *val,
        }
    }
}

impl TimeseriesValue {
    pub fn inverse(&self) -> Option<TimeseriesValue> {
        match self {
            TimeseriesValue::Float(val) => {
                if *val == 0. {
                    return None;
                }
                return Some(Self::Float(1. / val));
            }
            TimeseriesValue::Int(val) => {
                if *val == 0 {
                    return None;
                }
                return Some(Self::Float(1. / *val as f64));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum TimeseriesAggregate {
    Min,
    Max,
    Average,
    Sum,
}

impl TimeseriesAggregate {
    pub fn value_from_timeseries(
        &self,
        metric: &TimeseriesMetric,
        activity: &ActivityWithTimeseries,
    ) -> Option<f64> {
        let values: Vec<f64> = activity.timeseries().metrics().iter().find_map(|m| {
            if m.metric() == metric {
                Some(
                    m.values()
                        .iter()
                        .filter_map(|val| val.as_ref().map(f64::from))
                        .collect(),
                )
            } else {
                None
            }
        })?;
        if values.is_empty() {
            return None;
        }
        let length = values.len();
        match self {
            Self::Min => values.into_iter().reduce(f64::min),
            Self::Max => values.into_iter().reduce(f64::max),
            Self::Average => values
                .into_iter()
                .reduce(|acc, e| acc + e)
                .map(|val| val / length as f64),
            Self::Sum => values.into_iter().reduce(|acc, e| acc + e),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_different_activities_different_natural_keys() {
        let first_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
            None,
            None,
            None,
            None,
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Running,
            ActivityStatistics::default(),
            None,
            None,
            None,
            None,
        );

        assert_ne!(first_activity.natural_key(), second_activity.natural_key());
    }

    #[test]
    fn test_similar_activities_same_natural_keys() {
        let first_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
            None,
            None,
            None,
            None,
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
            None,
            None,
            None,
            None,
        );

        assert_eq!(first_activity.natural_key(), second_activity.natural_key());
    }

    #[test]
    fn test_same_activity_different_user_natural_keys_not_equal() {
        let first_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
            None,
            None,
            None,
            None,
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            "another_user".to_string().into(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
            None,
            None,
            None,
            None,
        );

        assert_ne!(first_activity.natural_key(), second_activity.natural_key());
    }

    #[test]
    fn test_rpe_valid_values() {
        assert!(ActivityRpe::try_from(1).is_ok());
        assert!(ActivityRpe::try_from(5).is_ok());
        assert!(ActivityRpe::try_from(10).is_ok());
    }

    #[test]
    fn test_rpe_invalid_values() {
        assert!(ActivityRpe::try_from(0).is_err());
        assert!(ActivityRpe::try_from(11).is_err());
        assert!(ActivityRpe::try_from(255).is_err());
    }

    #[test]
    fn test_rpe_value_getter() {
        let rpe = ActivityRpe::try_from(7).unwrap();
        assert_eq!(rpe.value(), 7);
    }

    #[test]
    fn test_rpe_variants() {
        assert_eq!(ActivityRpe::One.value(), 1);
        assert_eq!(ActivityRpe::Five.value(), 5);
        assert_eq!(ActivityRpe::Ten.value(), 10);
    }

    #[test]
    fn test_rpe_ordering() {
        assert!(ActivityRpe::One < ActivityRpe::Five);
        assert!(ActivityRpe::Five < ActivityRpe::Ten);
        assert_eq!(ActivityRpe::Seven, ActivityRpe::Seven);
    }

    #[test]
    fn test_bonk_status_from_str() {
        assert_eq!(BonkStatus::from_str("none").unwrap(), BonkStatus::None);
        assert_eq!(BonkStatus::from_str("bonked").unwrap(), BonkStatus::Bonked);
        assert_eq!(BonkStatus::from_str("NONE").unwrap(), BonkStatus::None);
        assert_eq!(BonkStatus::from_str("BONKED").unwrap(), BonkStatus::Bonked);
        assert!(BonkStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_bonk_status_display() {
        assert_eq!(BonkStatus::None.to_string(), "none");
        assert_eq!(BonkStatus::Bonked.to_string(), "bonked");
    }
}

#[cfg(test)]
mod test_timeseries {

    use super::*;

    #[test]
    fn test_new_timeseries_ok() {
        let time = TimeseriesTime::new(vec![1, 2, 3]);
        let active_time = TimeseriesActiveTime::new(vec![
            ActiveTime::Running(1),
            ActiveTime::Running(2),
            ActiveTime::Running(3),
        ]);
        let laps = vec![];
        let metrics = vec![Timeseries::new(
            TimeseriesMetric::Power,
            vec![
                Some(TimeseriesValue::Float(1.)),
                Some(TimeseriesValue::Float(1.)),
                Some(TimeseriesValue::Float(1.)),
            ],
        )];

        assert!(ActivityTimeseries::new(time, active_time, laps, metrics).is_ok());
    }

    #[test]
    fn test_new_timeseries_invalid_active_time_len() {
        let time = TimeseriesTime::new(vec![1, 2, 3]);
        let active_time =
            TimeseriesActiveTime::new(vec![ActiveTime::Running(1), ActiveTime::Running(2)]);

        let laps = vec![];
        let metrics = vec![Timeseries::new(
            TimeseriesMetric::Power,
            vec![
                Some(TimeseriesValue::Float(1.)),
                Some(TimeseriesValue::Float(1.)),
                Some(TimeseriesValue::Float(1.)),
            ],
        )];

        assert!(ActivityTimeseries::new(time, active_time, laps, metrics).is_err());
    }

    #[test]
    fn test_new_timeseries_invalid_metric_len() {
        let time = TimeseriesTime::new(vec![1, 2, 3]);
        let active_time = TimeseriesActiveTime::new(vec![
            ActiveTime::Running(1),
            ActiveTime::Running(2),
            ActiveTime::Running(3),
        ]);
        let laps = vec![];
        let metrics = vec![Timeseries::new(
            TimeseriesMetric::Power,
            vec![
                Some(TimeseriesValue::Float(1.)),
                Some(TimeseriesValue::Float(1.)),
            ],
        )];

        assert!(ActivityTimeseries::new(time, active_time, laps, metrics).is_err());
    }

    #[test]
    fn test_timeseries_value_inverse_float() {
        let value = TimeseriesValue::Float(2.0);
        let inverse = value.inverse().unwrap();
        assert_eq!(inverse, TimeseriesValue::Float(0.5));
    }

    #[test]
    fn test_timeseries_value_inverse_float_zero() {
        let value = TimeseriesValue::Float(0.0);
        let inverse = value.inverse();
        assert!(inverse.is_none());
    }

    #[test]
    fn test_timeseries_value_inverse_int() {
        let value = TimeseriesValue::Int(4);
        let inverse = value.inverse().unwrap();
        assert_eq!(inverse, TimeseriesValue::Float(0.25));
    }

    #[test]
    fn test_timeseries_value_inverse_int_zero() {
        let value = TimeseriesValue::Int(0);
        let inverse = value.inverse();
        assert!(inverse.is_none());
    }

    #[test]
    fn test_timeseries_value_inverse_large_float() {
        let value = TimeseriesValue::Float(1000.0);
        let inverse = value.inverse().unwrap();
        assert_eq!(inverse, TimeseriesValue::Float(0.001));
    }

    #[test]
    fn test_timeseries_value_inverse_small_float() {
        let value = TimeseriesValue::Float(0.25);
        let inverse = value.inverse().unwrap();
        assert_eq!(inverse, TimeseriesValue::Float(4.0));
    }

    fn default_activity() -> ActivityWithTimeseries {
        ActivityWithTimeseries::new(
            Activity::new(
                ActivityId::default(),
                UserId::test_default(),
                None,
                ActivityStartTime::new(
                    "2025-09-03T00:00:00Z"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                ),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::from([(ActivityStatistic::Calories, 123.3)])),
                None,
                None,
                None,
                None,
            ),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![0, 1, 2]),
                TimeseriesActiveTime::new(vec![
                    ActiveTime::Running(0),
                    ActiveTime::Running(1),
                    ActiveTime::Running(2),
                ]),
                vec![],
                vec![Timeseries::new(
                    TimeseriesMetric::Power,
                    vec![
                        Some(TimeseriesValue::Int(10)),
                        Some(TimeseriesValue::Int(20)),
                        Some(TimeseriesValue::Int(30)),
                    ],
                )],
            )
            .unwrap(),
        )
    }

    #[test]
    fn test_extract_aggregated_activity_metric_no_metric_found() {
        let metric = TimeseriesMetric::Speed;
        let aggregate = TimeseriesAggregate::Min;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_metric_is_empty() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Average;
        let activity = ActivityWithTimeseries::new(
            Activity::new(
                ActivityId::default(),
                UserId::test_default(),
                None,
                ActivityStartTime::new(
                    "2025-09-03T00:00:00Z"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                ),
                Sport::Cycling,
                ActivityStatistics::default(),
                None,
                None,
                None,
                None,
            ),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![]),
                TimeseriesActiveTime::new(vec![]),
                vec![],
                vec![Timeseries::new(TimeseriesMetric::Power, vec![])],
            )
            .unwrap(),
        );

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_min_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Min;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 10.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_max_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Max;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 30.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_average_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Average;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 20.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_total_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Sum;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 60.)
    }
}
