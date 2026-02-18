use std::collections::HashMap;

use chrono::{DateTime, NaiveDate};
use sqlx::{Database, encode::IsNull, error::BoxDynError};

use crate::domain::models::{
    UserId,
    activity::{
        ActivityFeedback, ActivityId, ActivityName, ActivityNaturalKey, ActivityNutrition,
        ActivityRpe, ActivityStartTime, ActivityStatistic, ActivityStatistics, Sport,
        TimeseriesAggregate, TimeseriesMetric, WorkoutType,
    },
    preferences::{Preference, PreferenceKey},
    training::{
        ActivityMetricSource, TrainingMetricAggregate, TrainingMetricFilters,
        TrainingMetricGranularity, TrainingMetricGroupBy, TrainingMetricId, TrainingMetricName,
        TrainingMetricValue, TrainingNoteContent, TrainingNoteDate, TrainingNoteId,
        TrainingNoteTitle, TrainingPeriodId, TrainingPeriodSports,
    },
};

impl sqlx::Type<sqlx::Sqlite> for ActivityId {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ActivityId {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ActivityId {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for UserId {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for UserId {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for UserId {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for ActivityName {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ActivityName {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ActivityName {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for ActivityFeedback {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ActivityFeedback {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ActivityFeedback {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for ActivityNaturalKey {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ActivityNaturalKey {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ActivityNaturalKey {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for ActivityStartTime {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ActivityStartTime {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ActivityStartTime {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        let date = DateTime::parse_from_rfc3339(s)?;
        Ok(Self::from(date))
    }
}

impl sqlx::Type<sqlx::Sqlite> for Sport {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for Sport {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(
            self.to_string().into(),
        ));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for Sport {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        s.parse::<Sport>()
            .map_err(|_| format!("Unknown Sport: {}", s).into())
    }
}

impl sqlx::Type<sqlx::Sqlite> for ActivityStatistics {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <Vec<u8> as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ActivityStatistics {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let json_bytes = serde_json::to_vec(&self).unwrap();
        args.push(sqlx::sqlite::SqliteArgumentValue::Blob(json_bytes.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ActivityStatistics {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <&[u8] as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        let map: HashMap<ActivityStatistic, f64> = serde_json::from_slice(bytes)?;
        Ok(ActivityStatistics::new(map))
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingMetricId {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingMetricId {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingMetricId {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingMetricName {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingMetricName {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingMetricName {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for ActivityMetricSource {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <Vec<u8> as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ActivityMetricSource {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let json_bytes = serde_json::to_vec(&self).unwrap();
        args.push(sqlx::sqlite::SqliteArgumentValue::Blob(json_bytes.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ActivityMetricSource {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <&[u8] as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(serde_json::from_slice(bytes)?)
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingMetricGranularity {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingMetricGranularity {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let s = match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        };
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(s.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingMetricGranularity {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        match s {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            _ => Err(format!("Unknown Granularity: {}", s).into()),
        }
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingMetricAggregate {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingMetricAggregate {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let s = match self {
            Self::Average => "average",
            Self::Max => "max",
            Self::Min => "min",
            Self::Sum => "sum",
            Self::NumberOfActivities => "number_of_activities",
        };
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(s.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingMetricAggregate {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        match s {
            "average" => Ok(Self::Average),
            "max" => Ok(Self::Max),
            "min" => Ok(Self::Min),
            "sum" => Ok(Self::Sum),
            "number_of_activities" => Ok(Self::NumberOfActivities),
            _ => Err(format!("Unknown Aggregate: {}", s).into()),
        }
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingMetricValue {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <Vec<u8> as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingMetricValue {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let json_bytes = serde_json::to_vec(&self).unwrap();
        args.push(sqlx::sqlite::SqliteArgumentValue::Blob(json_bytes.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingMetricValue {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <&[u8] as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(serde_json::from_slice(bytes)?)
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingMetricFilters {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <Vec<u8> as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingMetricFilters {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let json_bytes = serde_json::to_vec(&self).unwrap();
        args.push(sqlx::sqlite::SqliteArgumentValue::Blob(json_bytes.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingMetricFilters {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <&[u8] as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(serde_json::from_slice(bytes)?)
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingPeriodId {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingPeriodId {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingPeriodId {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingPeriodSports {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <Vec<u8> as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingPeriodSports {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let json_bytes = serde_json::to_vec(&self).unwrap();
        args.push(sqlx::sqlite::SqliteArgumentValue::Blob(json_bytes.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingPeriodSports {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <&[u8] as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(serde_json::from_slice(bytes)?)
    }
}

impl sqlx::Type<sqlx::Sqlite> for ActivityRpe {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <u64 as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ActivityRpe {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let value = self.value();
        args.push(sqlx::sqlite::SqliteArgumentValue::Int(value.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ActivityRpe {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <u64 as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::try_from(u8::try_from(s)?)?)
    }
}

impl sqlx::Type<sqlx::Sqlite> for WorkoutType {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for WorkoutType {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for WorkoutType {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(s.parse()?)
    }
}

impl sqlx::Type<sqlx::Sqlite> for ActivityNutrition {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <Vec<u8> as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ActivityNutrition {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let json_bytes = serde_json::to_vec(&self).unwrap();
        args.push(sqlx::sqlite::SqliteArgumentValue::Blob(json_bytes.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ActivityNutrition {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <&[u8] as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(serde_json::from_slice(bytes)?)
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingMetricGroupBy {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingMetricGroupBy {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let s = match self {
            Self::Sport => "sport",
            Self::SportCategory => "sport_category",
            Self::WorkoutType => "workout_type",
            Self::RpeRange => "rpe_range",
            Self::Bonked => "bonked",
        };
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(s.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingMetricGroupBy {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;

        match s {
            "sport" => Ok(Self::Sport),
            "sport_category" => Ok(Self::SportCategory),
            "workout_type" => Ok(Self::WorkoutType),
            "rpe_range" => Ok(Self::RpeRange),
            "bonked" => Ok(Self::Bonked),
            _ => Err(format!("Unknown GroupBy: {}", s).into()),
        }
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingNoteId {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingNoteId {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingNoteId {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingNoteTitle {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingNoteTitle {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingNoteTitle {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for TrainingNoteContent {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TrainingNoteContent {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingNoteContent {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self::from(s))
    }
}

// SQLx trait implementations for database operations
impl sqlx::Type<sqlx::Sqlite> for TrainingNoteDate {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <&str as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TrainingNoteDate {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let date_str = <&str as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
        Ok(TrainingNoteDate::from(date))
    }
}

impl sqlx::Encode<'_, sqlx::Sqlite> for TrainingNoteDate {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'_>>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let date_str = self.as_naive_date().format("%Y-%m-%d").to_string();
        <String as sqlx::Encode<'_, sqlx::Sqlite>>::encode(date_str, buf)
    }
}

impl sqlx::Type<sqlx::Sqlite> for PreferenceKey {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for PreferenceKey {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let text = self.to_string();
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(text.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for PreferenceKey {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

/// Serialize a preference value to string for storage
pub fn serialize_preference_value(preference: &Preference) -> Result<String, BoxDynError> {
    match preference {
        Preference::FavoriteMetric(id) => Ok(id.to_string()),
    }
}

/// Deserialize a preference value from storage
pub fn deserialize_preference(key: &PreferenceKey, value: &str) -> Result<Preference, BoxDynError> {
    match key {
        PreferenceKey::FavoriteMetric => {
            let id = TrainingMetricId::from(value);
            Ok(Preference::FavoriteMetric(id))
        }
    }
}

impl sqlx::Type<sqlx::Sqlite> for TimeseriesMetric {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TimeseriesMetric {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let s = match self {
            Self::Speed => "speed",
            Self::Power => "power",
            Self::HeartRate => "hr",
            Self::Distance => "distance",
            Self::Cadence => "cadence",
            Self::Altitude => "altitude",
            Self::Pace => "pace-v3",
        };
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(s.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TimeseriesMetric {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        match s {
            "speed" => Ok(Self::Speed),
            "power" => Ok(Self::Power),
            "hr" => Ok(Self::HeartRate),
            "distance" => Ok(Self::Distance),
            "cadence" => Ok(Self::Cadence),
            "altitude" => Ok(Self::Altitude),
            "pace" => Ok(Self::Pace),
            _ => Err(format!("Unknown TimeseriesMetric: {}", s).into()),
        }
    }
}

impl sqlx::Type<sqlx::Sqlite> for TimeseriesAggregate {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TimeseriesAggregate {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let s = match self {
            Self::Average => "average",
            Self::Max => "max",
            Self::Min => "min",
        };
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(s.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TimeseriesAggregate {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        match s {
            "average" => Ok(Self::Average),
            "max" => Ok(Self::Max),
            "min" => Ok(Self::Min),
            _ => Err(format!("Unknown TimeseriesAggregate: {}", s).into()),
        }
    }
}
