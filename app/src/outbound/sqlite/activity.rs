use std::str::FromStr;

use anyhow::anyhow;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::domain::{
    models::{
        UserId,
        activity::{
            Activity, ActivityId, ActivityName, ActivityNaturalKey, ActivityStartTime,
            ActivityStatistics, ActivityWithTimeseries, Sport,
        },
    },
    ports::{
        ActivityRepository, GetActivityError, ListActivitiesError, SaveActivityError,
        SimilarActivityError,
    },
};

type ActivityRow = (
    ActivityId,
    UserId,
    Option<ActivityName>,
    ActivityStartTime,
    Sport,
    ActivityStatistics,
);

#[derive(Debug, Clone)]
pub struct SqliteActivityRepository {
    pool: SqlitePool,
}

impl SqliteActivityRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migration here for now
        sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS t_activities (
            id TEXT UNIQUE,
            user_id TEXT,
            name TEXT NULLABLE,
            start_time TEXT,
            sport TEXT,
            statistics BLOB,
            natural_key TEXT
        );

        CREATE INDEX IF NOT EXISTS t_activities_natural_key_idx
        ON t_activities(natural_key);"#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}

impl ActivityRepository for SqliteActivityRepository {
    async fn delete_activity(&self, activity: &ActivityId) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM t_activities WHERE id = ?1")
            .bind(activity)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|err| anyhow!("Unanble to delete activity {}. {err}", activity))
    }

    async fn get_activity(&self, id: &ActivityId) -> Result<Option<Activity>, anyhow::Error> {
        match sqlx::query_as::<_, ActivityRow>(
            "SELECT id, user_id, name, start_time, sport, statistics
            FROM t_activities
            WHERE id = ?1
            LIMIT 1;",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        {
            Ok((id, user_id, name, start_time, sport, statistics)) => Ok(Some(Activity::new(
                id, user_id, name, start_time, sport, statistics,
            ))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(anyhow!(err)),
        }
    }

    async fn get_activity_with_timeseries(
        &self,
        _id: &ActivityId,
    ) -> Result<Option<ActivityWithTimeseries>, GetActivityError> {
        todo!()
    }

    async fn list_activities(&self, user: &UserId) -> Result<Vec<Activity>, ListActivitiesError> {
        sqlx::query_as::<_, ActivityRow>(
            "SELECT id, user_id, name, start_time, sport, statistics
            FROM t_activities
            WHERE user_id = ?1;",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| ListActivitiesError::Unknown(anyhow!(err)))
        .map(|rows| {
            rows.into_iter()
                .map(|(id, user_id, name, start_time, sport, statistics)| {
                    Activity::new(id, user_id, name, start_time, sport, statistics)
                })
                .collect()
        })
    }

    async fn list_activities_with_timeseries(
        &self,
        _user: &UserId,
    ) -> Result<Vec<ActivityWithTimeseries>, ListActivitiesError> {
        todo!()
    }

    async fn modify_activity_name(
        &self,
        id: &ActivityId,
        name: Option<ActivityName>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query("UPDATE t_activities SET name = ?1 WHERE id = ?2;")
            .bind(name)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|err| anyhow!(err))
            .map(|_| ())
    }

    async fn save_activity(
        &self,
        activity: &ActivityWithTimeseries,
    ) -> Result<(), SaveActivityError> {
        sqlx::query(
            "INSERT INTO t_activities VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7
        );",
        )
        .bind(activity.id())
        .bind(activity.user())
        .bind(activity.name())
        .bind(activity.start_time().date())
        .bind(activity.sport())
        .bind(activity.statistics())
        .bind(activity.natural_key())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|err| {
            SaveActivityError::Unknown(anyhow!("Unanble to save activity {}. {err}", activity.id()))
        })
    }

    async fn similar_activity_exists(
        &self,
        natural_key: &ActivityNaturalKey,
    ) -> Result<bool, SimilarActivityError> {
        match sqlx::query("SELECT natural_key FROM t_activities WHERE natural_key = ?1;")
            .bind(natural_key)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(row) => Ok(row.is_some()),
            Err(sqlx::Error::RowNotFound) => Ok(false),
            Err(err) => {
                tracing::warn!("hello?");
                dbg!(&err);
                Err(SimilarActivityError::Unknown(anyhow!(err)))
            }
        }
    }
}

#[cfg(test)]
mod test_sqlite_activity_repository {

    use std::collections::HashMap;

    use rand::random_range;
    use tempfile::NamedTempFile;

    use crate::domain::models::{
        UserId,
        activity::{
            ActivityStartTime, ActivityStatistic, ActivityStatistics, ActivityTimeseries, Sport,
            Timeseries, TimeseriesMetric, TimeseriesTime, TimeseriesValue,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_init_table() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        sqlx::query("select count(*) from t_activities;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
    }

    fn build_activity() -> Activity {
        Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(random_range(100..1200)).unwrap(),
            Sport::Cycling,
            ActivityStatistics::new(HashMap::from([(ActivityStatistic::Calories, 123.3)])),
        )
    }

    fn build_activity_with_timeseries() -> ActivityWithTimeseries {
        ActivityWithTimeseries::new(
            build_activity(),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![0, 1, 2, 3]),
                vec![Timeseries::new(
                    TimeseriesMetric::Speed,
                    vec![
                        Some(TimeseriesValue::Float(1.3)),
                        Some(TimeseriesValue::Float(1.45)),
                        Some(TimeseriesValue::Float(1.15)),
                        Some(TimeseriesValue::Float(2.45)),
                    ],
                )],
            ),
        )
    }

    #[tokio::test]
    async fn test_save_activity() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();

        repository
            .save_activity(&activity)
            .await
            .expect("Should have succeed");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_activities;")
                .fetch_one(&repository.pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_does_not_save_duplicated_activity_id() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();

        repository
            .save_activity(&activity)
            .await
            .expect("Should have succeed");

        repository
            .save_activity(&activity)
            .await
            .expect_err("Should have failed");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_activities;")
                .fetch_one(&repository.pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_delete_activity() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();

        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_activities;")
                .fetch_one(&repository.pool)
                .await
                .unwrap(),
            1
        );

        repository
            .delete_activity(activity.id())
            .await
            .expect("Deletion should have succeeded");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_activities;")
                .fetch_one(&repository.pool)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_delete_activity_does_not_exist_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();

        repository
            .delete_activity(activity.id())
            .await
            .expect("Should have returned ok");
    }

    #[tokio::test]
    async fn test_get_activity() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();

        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_activities;")
                .fetch_one(&repository.pool)
                .await
                .unwrap(),
            1
        );

        let res = repository
            .get_activity(activity.id())
            .await
            .expect("Get should have succeeded")
            .expect("Should not be None");

        assert_eq!(res.id(), activity.id());
        assert_eq!(res.user(), activity.user());
        assert_eq!(res.name(), activity.name());
        assert_eq!(res.start_time(), activity.start_time());
        assert_eq!(res.sport(), activity.sport());
        assert_eq!(res.statistics(), activity.statistics());
    }

    #[tokio::test]
    async fn test_get_activity_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();

        let res = repository
            .get_activity(activity.id())
            .await
            .expect("Get should have succeeded");

        assert!(res.is_none());
    }

    #[tokio::test]
    async fn test_list_activities() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_activities(&UserId::test_default())
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 2);
    }

    #[tokio::test]
    async fn test_list_activities_ignore_other_users() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_activities(&UserId::from("another_user"))
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 0);
    }

    #[tokio::test]
    async fn test_modify_activity_name() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "select count(*) from t_activities where name = 'a new name';"
            )
            .fetch_one(&repository.pool)
            .await
            .unwrap(),
            0
        );

        repository
            .modify_activity_name(
                activity.id(),
                Some(ActivityName::new("a new name".to_string())),
            )
            .await
            .expect("Should not have err");

        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "select count(*) from t_activities where name = 'a new name';"
            )
            .fetch_one(&repository.pool)
            .await
            .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_natural_key_exists() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        assert!(
            repository
                .similar_activity_exists(&activity.natural_key())
                .await
                .expect("Should not have err")
        );
    }

    #[tokio::test]
    async fn test_natural_key_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        assert!(
            !repository
                .similar_activity_exists(&ActivityNaturalKey::from("another_key"))
                .await
                .expect("Should not have err")
        );
    }
}
