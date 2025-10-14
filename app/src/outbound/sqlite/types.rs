use std::collections::HashMap;

use chrono::DateTime;
use sqlx::{Database, encode::IsNull, error::BoxDynError};

use crate::domain::models::{
    UserId,
    activity::{
        ActivityId, ActivityName, ActivityNaturalKey, ActivityStartTime, ActivityStatistic,
        ActivityStatistics, Sport,
    },
    training_metrics::{
        ActivityMetricSource, TrainingMetricAggregate, TrainingMetricFilters,
        TrainingMetricGranularity, TrainingMetricId, TrainingMetricValue,
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
