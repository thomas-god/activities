use std::str::FromStr;

use anyhow::anyhow;
use chrono::{DateTime, FixedOffset};
use sqlx::{Sqlite, SqlitePool, sqlite::SqliteConnectOptions};

use crate::{
    domain::{
        models::{
            UserId,
            activity::{
                Activity, ActivityId, ActivityName, ActivityNaturalKey, ActivityStartTime,
                ActivityStatistics, ActivityWithTimeseries, Sport,
            },
        },
        ports::{
            ActivityRepository, DateTimeRange, ListActivitiesError, ListActivitiesFilters,
            RawDataRepository, SaveActivityError, SimilarActivityError,
        },
    },
    inbound::parser::ParseFile,
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
pub struct SqliteActivityRepository<R, FP> {
    pool: SqlitePool,
    raw_data_repository: R,
    file_parser: FP,
}

impl<R, FP> SqliteActivityRepository<R, FP> {
    pub async fn new(
        url: &str,
        raw_data_repository: R,
        file_parser: FP,
    ) -> Result<Self, sqlx::Error> {
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

        Ok(Self {
            pool,
            raw_data_repository,
            file_parser,
        })
    }
}

impl<R, FP> SqliteActivityRepository<R, FP>
where
    R: RawDataRepository,
    FP: ParseFile,
{
    async fn load_timeseries(
        &self,
        id: &ActivityId,
        activity: Activity,
    ) -> Result<ActivityWithTimeseries, anyhow::Error> {
        let raw_data = match self.raw_data_repository.get_raw_data(id).await {
            Ok(raw_data) => raw_data,
            Err(err) => return Err(anyhow!(err)),
        };

        let extension = raw_data
            .extension()
            .try_into()
            .map_err(|_| anyhow!("Unsupported file format: {}", raw_data.extension()))?;

        let parsed_content = match self
            .file_parser
            .try_bytes_into_domain(&extension, raw_data.raw_content())
        {
            Ok(parsed_content) => parsed_content,
            Err(err) => return Err(anyhow!(err)),
        };

        Ok(ActivityWithTimeseries::new(
            activity,
            parsed_content.timeseries().clone(),
        ))
    }
}

impl<R, FP> ActivityRepository for SqliteActivityRepository<R, FP>
where
    R: RawDataRepository,
    FP: ParseFile,
{
    async fn delete_activity(&self, activity: &ActivityId) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM t_activities WHERE id = ?1")
            .bind(activity)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|err| anyhow!("Unable to delete activity {}. {err}", activity))
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
        id: &ActivityId,
    ) -> Result<Option<ActivityWithTimeseries>, anyhow::Error> {
        let activity = match self.get_activity(id).await {
            Ok(Some(activity)) => activity,
            Ok(None) => return Ok(None),
            Err(err) => return Err(anyhow!(err)),
        };

        let activity_with_timeseries = match self.load_timeseries(id, activity).await {
            Ok(value) => value,
            Err(err) => return Err(err),
        };

        Ok(Some(activity_with_timeseries))
    }

    async fn list_activities(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<Activity>, ListActivitiesError> {
        let mut builder = sqlx::QueryBuilder::<'_, Sqlite>::new(
            "SELECT id, user_id, name, start_time, sport, statistics
            FROM t_activities ",
        );
        builder.push("WHERE user_id = ").push_bind(user);

        if let Some(date_range) = filters.date_range() {
            builder
                .push(" AND start_time >= ")
                .push_bind(date_range.start());
            builder
                .push(" AND start_time < ")
                .push_bind(date_range.end());
        }

        builder.push("ORDER BY start_time DESC ");

        if let Some(limit) = *filters.limit() {
            builder.push("LIMIT ").push_bind(limit as i64);
        }

        let query = builder.build_query_as::<'_, ActivityRow>();

        query
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
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<ActivityWithTimeseries>, ListActivitiesError> {
        let activities = self.list_activities(user, filters).await?;

        let mut res = vec![];
        for activity in activities.into_iter() {
            let Ok(activity_with_timeseries) =
                self.load_timeseries(&activity.id().clone(), activity).await
            else {
                continue;
            };
            res.push(activity_with_timeseries);
        }
        Ok(res)
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

    async fn get_user_history_date_range(
        &self,
        user: &UserId,
    ) -> Result<Option<crate::domain::ports::DateTimeRange>, anyhow::Error> {
        // Option<DateTime<FixedOffset>> because MIN/MAX(...) return NULL if the set is empty
        match sqlx::query_as::<_, (Option<DateTime<FixedOffset>>, Option<DateTime<FixedOffset>>)>(
            "
        SELECT MIN(start_time), MAX(start_time)
        FROM t_activities
        WHERE user_id = ?1;",
        )
        .bind(user)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some((Some(start), Some(end)))) => Ok(Some(DateTimeRange::new(start, Some(end)))),
            Ok(Some(_)) => Ok(None),
            Ok(None) => Ok(None),
            Err(err) => Err(anyhow!(
                "Unable to get history date range for user {}. {err}",
                user
            )),
        }
    }
}

#[cfg(test)]
mod test_sqlite_activity_repository {

    use std::collections::HashMap;

    use chrono::NaiveDate;
    use rand::random_range;
    use tempfile::NamedTempFile;

    use crate::{
        domain::{
            models::{
                UserId,
                activity::{
                    ActiveTime, ActivityStartTime, ActivityStatistic, ActivityStatistics,
                    ActivityTimeseries, Sport, Timeseries, TimeseriesActiveTime, TimeseriesMetric,
                    TimeseriesTime, TimeseriesValue,
                },
            },
            ports::{DateRange, GetRawDataError, RawContent, test_utils::MockRawDataRepository},
        },
        inbound::parser::{ParseBytesError, ParsedFileContent, test_utils::MockFileParser},
    };

    use super::*;

    #[tokio::test]
    async fn test_init_table() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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

    fn build_activity_starting_at(start: &DateTime<FixedOffset>) -> Activity {
        Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::new(*start),
            Sport::Cycling,
            ActivityStatistics::new(HashMap::from([(ActivityStatistic::Calories, 123.3)])),
        )
    }

    fn build_activity_with_timeseries() -> ActivityWithTimeseries {
        ActivityWithTimeseries::new(
            build_activity(),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![0, 1, 2, 3]),
                TimeseriesActiveTime::new(vec![
                    ActiveTime::Running(0),
                    ActiveTime::Running(1),
                    ActiveTime::Running(2),
                    ActiveTime::Running(3),
                ]),
                vec![Timeseries::new(
                    TimeseriesMetric::Speed,
                    vec![
                        Some(TimeseriesValue::Float(1.3)),
                        Some(TimeseriesValue::Float(1.45)),
                        Some(TimeseriesValue::Float(1.15)),
                        Some(TimeseriesValue::Float(2.45)),
                    ],
                )],
            )
            .unwrap(),
        )
    }

    fn build_activity_with_timeseries_starting_at(
        start: &DateTime<FixedOffset>,
    ) -> ActivityWithTimeseries {
        ActivityWithTimeseries::new(
            build_activity_starting_at(start),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![0, 1, 2, 3]),
                TimeseriesActiveTime::new(vec![
                    ActiveTime::Running(0),
                    ActiveTime::Running(1),
                    ActiveTime::Running(2),
                    ActiveTime::Running(3),
                ]),
                vec![Timeseries::new(
                    TimeseriesMetric::Speed,
                    vec![
                        Some(TimeseriesValue::Float(1.3)),
                        Some(TimeseriesValue::Float(1.45)),
                        Some(TimeseriesValue::Float(1.15)),
                        Some(TimeseriesValue::Float(2.45)),
                    ],
                )],
            )
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn test_save_activity() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
            .list_activities(&UserId::test_default(), &ListActivitiesFilters::empty())
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 2);
    }

    #[tokio::test]
    async fn test_list_activities_with_limit() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
            .list_activities(
                &UserId::test_default(),
                &ListActivitiesFilters::empty().set_limit(Some(1)),
            )
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_list_activities_with_date_range() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity_with_timeseries_starting_at(
            &"2025-09-29T12:34:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
        );
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let activity = build_activity_with_timeseries_starting_at(
            &"2025-10-03T12:34:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
        );
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_activities(
                &UserId::test_default(),
                &ListActivitiesFilters::empty().set_date_range(Some(DateRange::new(
                    "2025-09-10".parse::<NaiveDate>().unwrap(),
                    "2025-10-01".parse::<NaiveDate>().unwrap(),
                ))),
            )
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_list_activities_with_date_range_timezone() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity_with_timeseries_starting_at(
            &"2025-09-10T08:34:00-10:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
        );
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_activities(
                &UserId::test_default(),
                &ListActivitiesFilters::empty().set_date_range(Some(DateRange::new(
                    "2025-09-10".parse::<NaiveDate>().unwrap(),
                    "2025-09-11".parse::<NaiveDate>().unwrap(),
                ))),
            )
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_list_activities_ignore_other_users() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
            .list_activities(
                &UserId::from("another_user"),
                &ListActivitiesFilters::empty(),
            )
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 0);
    }

    #[tokio::test]
    async fn test_modify_activity_name() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
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

    fn build_parsed_file_content() -> ParsedFileContent {
        ParsedFileContent::new(
            Sport::Cycling,
            ActivityStartTime::from_timestamp(120).unwrap(),
            ActivityStatistics::new(HashMap::new()),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![0, 1, 2, 3]),
                TimeseriesActiveTime::new(vec![
                    ActiveTime::Running(0),
                    ActiveTime::Running(1),
                    ActiveTime::Running(2),
                    ActiveTime::Running(3),
                ]),
                vec![Timeseries::new(
                    TimeseriesMetric::Altitude,
                    vec![
                        Some(TimeseriesValue::Float(12.3)),
                        Some(TimeseriesValue::Float(12.3)),
                        Some(TimeseriesValue::Float(12.3)),
                        Some(TimeseriesValue::Float(12.3)),
                    ],
                )],
            )
            .unwrap(),
            "fit".to_string(),
            vec![],
        )
    }

    #[tokio::test]
    async fn test_get_activity_with_timeseries_ok() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .times(1)
            .returning(|_, __| Ok(build_parsed_file_content()));
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
        )
        .await
        .expect("repo should init");

        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .get_activity_with_timeseries(activity.id())
            .await
            .expect("Should have succeeded")
            .expect("Should not be none");

        assert_eq!(
            res.timeseries().metrics().first().unwrap(),
            &Timeseries::new(
                TimeseriesMetric::Altitude,
                vec![
                    Some(TimeseriesValue::Float(12.3)),
                    Some(TimeseriesValue::Float(12.3)),
                    Some(TimeseriesValue::Float(12.3)),
                    Some(TimeseriesValue::Float(12.3)),
                ],
            )
        );
    }

    #[tokio::test]
    async fn test_get_activity_with_timeseries_get_raw_data_fails() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Err(GetRawDataError::Unknown));
        let mut file_parser = MockFileParser::new();
        file_parser.expect_try_bytes_into_domain().times(0);
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
        )
        .await
        .expect("repo should init");

        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        repository
            .get_activity_with_timeseries(activity.id())
            .await
            .expect_err("Should have failed");
    }

    #[tokio::test]
    async fn test_get_activity_with_timeseries_raw_data_parsing_fails() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .times(1)
            .returning(|_, __| Err(ParseBytesError::InvalidContent));

        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
        )
        .await
        .expect("repo should init");

        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        repository
            .get_activity_with_timeseries(activity.id())
            .await
            .expect_err("Should have failed");
    }

    #[tokio::test]
    async fn test_list_activities_with_timeseries_ok() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(2)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .times(2)
            .returning(|_, __| Ok(build_parsed_file_content()));
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
        )
        .await
        .expect("repo should init");

        // Insert 2 activities
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .list_activities_with_timeseries(activity.user(), &ListActivitiesFilters::empty())
            .await
            .expect("Should have succeeded");

        assert_eq!(res.len(), 2);
    }

    #[tokio::test]
    async fn test_list_activities_with_timeseries_with_limit() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .returning(|_, __| Ok(build_parsed_file_content()));
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
        )
        .await
        .expect("repo should init");

        // Insert 2 activities
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .list_activities_with_timeseries(
                activity.user(),
                &ListActivitiesFilters::empty().set_limit(Some(1)),
            )
            .await
            .expect("Should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_list_activities_with_timeseries_ok_ignore_failed_activities() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .return_once(|_| Err(GetRawDataError::Unknown));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .times(1)
            .returning(|_, __| Ok(build_parsed_file_content()));
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
        )
        .await
        .expect("repo should init");

        // Insert 2 activities
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");
        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .list_activities_with_timeseries(activity.user(), &ListActivitiesFilters::empty())
            .await
            .expect("Should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_user_history_date_range_when_no_activities() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
        .await
        .expect("repo should init");

        assert!(
            repository
                .get_user_history_date_range(&UserId::test_default())
                .await
                .expect("Should be Ok")
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_user_history_date_range() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity_with_timeseries();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let another_activity = build_activity_with_timeseries();
        repository
            .save_activity(&another_activity)
            .await
            .expect("Insertion should have succeed");

        let date_range = repository
            .get_user_history_date_range(&UserId::test_default())
            .await
            .expect("Should be Ok")
            .expect("Should be Some");
        let expected_start = activity
            .start_time()
            .date()
            .min(another_activity.start_time().date());
        let expected_end = activity
            .start_time()
            .date()
            .max(another_activity.start_time().date());

        assert_eq!(date_range.start(), expected_start);
        assert_eq!(date_range.end().expect("End should be some"), *expected_end);
    }
}
