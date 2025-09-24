use std::collections::HashMap;

use chrono::DateTime;
use sqlx::{Database, encode::IsNull, error::BoxDynError};

use crate::domain::models::{
    UserId,
    activity::{
        ActivityId, ActivityName, ActivityNaturalKey, ActivityStartTime, ActivityStatistic,
        ActivityStatistics, Sport,
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
        let s = match self {
            Self::Running => "running",
            Self::Cycling => "cycling",
            Self::Other => "other",
        };
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(s.into()));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for Sport {
    fn decode(value: <sqlx::Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        match s {
            "running" => Ok(Sport::Running),
            "cycling" => Ok(Sport::Cycling),
            "other" => Ok(Sport::Other),
            _ => Err(format!("Unknown Sport: {}", s).into()),
        }
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
