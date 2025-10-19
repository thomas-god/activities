use std::sync::Arc;

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::domain::{
    models::{
        UserId,
        training::{
            ComputeMetricRequirement, TrainingMetricDefinition, TrainingMetricId,
            TrainingMetricValues, TrainingPeriodId,
        },
    },
    ports::{
        ActivityRepository, CreateTrainingMetricError, CreateTrainingMetricRequest,
        CreateTrainingPeriodError, CreateTrainingPeriodRequest, DateRange,
        DeleteTrainingMetricError, DeleteTrainingMetricRequest, ITrainingService,
        ListActivitiesFilters, TrainingRepository, UpdateMetricsValuesRequest,
    },
};

///////////////////////////////////////////////////////////////////
/// TRAINING SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Constructor)]
pub struct TrainingService<TR, AR>
where
    TR: TrainingRepository,
    AR: ActivityRepository,
{
    training_repository: TR,
    activity_repository: Arc<Mutex<AR>>,
}

impl<TR, AR> TrainingService<TR, AR>
where
    TR: TrainingRepository,
    AR: ActivityRepository,
{
    async fn compute_metric_values(
        &self,
        definition: &TrainingMetricDefinition,
        bin: &DateRange,
        user: &UserId,
    ) {
        let values = match definition.source_requirement() {
            ComputeMetricRequirement::ActivityWithTimeseries => {
                let Ok(activities) = self
                    .activity_repository
                    .lock()
                    .await
                    .list_activities_with_timeseries(
                        user,
                        &ListActivitiesFilters::empty().set_date_range(Some(bin.clone())),
                    )
                    .await
                else {
                    return;
                };

                definition.compute_values_from_timeseries(&activities)
            }
            ComputeMetricRequirement::Activity => {
                let Ok(activities) = self
                    .activity_repository
                    .lock()
                    .await
                    .list_activities(
                        user,
                        &ListActivitiesFilters::empty().set_date_range(Some(bin.clone())),
                    )
                    .await
                else {
                    return;
                };

                definition.compute_values(&activities)
            }
        };

        for (key, value) in values.into_iter() {
            let _ = self
                .training_repository
                .update_metric_values(definition.id(), (key, value))
                .await;
        }
    }
}

impl<TMR, AR> ITrainingService for TrainingService<TMR, AR>
where
    TMR: TrainingRepository,
    AR: ActivityRepository,
{
    /// Create a new training metric and compute its values on the user's activity history.
    /// If [CreateTrainingMetricRequest::initial_date_range] is not None, this function computes the
    /// metric value on that range before returning. The rest of the history is processed in the
    /// background and may take some time depending on the history size.
    async fn create_metric(
        &self,
        req: CreateTrainingMetricRequest,
    ) -> Result<TrainingMetricId, CreateTrainingMetricError> {
        let id = TrainingMetricId::new();
        let definition = TrainingMetricDefinition::new(
            id.clone(),
            req.user().clone(),
            req.source().clone(),
            req.granularity().clone(),
            req.aggregate().clone(),
            req.filters().clone(),
        );
        self.training_repository
            .save_definition(definition.clone())
            .await?;

        let mut initial_bins = Vec::new();
        if let Some(initial_range) = req.initial_date_range() {
            initial_bins = definition.granularity().bins(initial_range);
            for bin in initial_bins.iter() {
                self.compute_metric_values(&definition, bin, req.user())
                    .await;
            }
        }

        let this = self.clone();
        tokio::spawn(async move {
            let _ = this
                .compute_initial_values(req, definition, initial_bins)
                .await;
        });

        Ok(id)
    }

    async fn update_metrics_values(&self, req: UpdateMetricsValuesRequest) -> Result<(), ()> {
        let definitions = self
            .training_repository
            .get_definitions(req.user())
            .await
            .unwrap();

        for definition in definitions {
            for activity in req.new_activities() {
                let Some(metric) = definition
                    .source()
                    .metric_from_activity_with_timeseries(activity)
                else {
                    continue;
                };
                let bin_key = definition
                    .granularity()
                    .datetime_key(activity.start_time().date());
                let new_value = match self
                    .training_repository
                    .get_metric_value(definition.id(), &bin_key)
                    .await
                {
                    Ok(Some(previous_value)) => {
                        let Some(new_value) = definition
                            .aggregate()
                            .update_value(&previous_value, &metric)
                        else {
                            continue;
                        };
                        new_value
                    }
                    Ok(None) => {
                        let Some(new_value) = definition.aggregate().initial_value(&metric) else {
                            continue;
                        };
                        new_value
                    }
                    Err(_err) => continue,
                };
                let _ = self
                    .training_repository
                    .update_metric_values(definition.id(), (bin_key, new_value))
                    .await;
            }
        }

        Ok(())
    }

    async fn get_training_metrics(
        &self,
        user: &UserId,
    ) -> Vec<(TrainingMetricDefinition, TrainingMetricValues)> {
        let Ok(definitions) = self.training_repository.get_definitions(user).await else {
            return vec![];
        };

        let mut res = vec![];
        for definition in definitions {
            let values = self
                .training_repository
                .get_metric_values(definition.id())
                .await
                .unwrap_or_default();
            res.push((definition.clone(), values.clone()))
        }

        res
    }

    async fn delete_metric(
        &self,
        req: DeleteTrainingMetricRequest,
    ) -> Result<(), DeleteTrainingMetricError> {
        let Some(definition) = self
            .training_repository
            .get_definition(req.metric())
            .await?
        else {
            return Err(DeleteTrainingMetricError::MetricDoesNotExist(
                req.metric().clone(),
            ));
        };

        if definition.user() != req.user() {
            return Err(DeleteTrainingMetricError::UserDoesNotOwnTrainingMetric(
                req.user().clone(),
                req.metric().clone(),
            ));
        }

        self.training_repository
            .delete_definition(req.metric())
            .await?;

        Ok(())
    }

    async fn create_training_period(
        &self,
        req: CreateTrainingPeriodRequest,
    ) -> Result<TrainingPeriodId, CreateTrainingPeriodError> {
        let id = TrainingPeriodId::new();
        let period = req
            .to_period(&id)
            .map_err(CreateTrainingPeriodError::InvalidPeriod)?;

        self.training_repository
            .save_training_period(period)
            .await
            .map_err(|err| CreateTrainingPeriodError::Unknown(err.into()))?;

        Ok(id)
    }

    async fn get_training_period(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
    ) -> Option<crate::domain::models::training::TrainingPeriod> {
        self.training_repository
            .get_training_period(user, period)
            .await
    }

    async fn get_training_periods(
        &self,
        user: &UserId,
    ) -> Vec<crate::domain::models::training::TrainingPeriod> {
        self.training_repository.get_training_periods(user).await
    }

    async fn get_training_period_with_activities(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
    ) -> Option<crate::domain::models::training::TrainingPeriodWithActivities> {
        use crate::domain::models::training::TrainingPeriodWithActivities;

        let period = self
            .training_repository
            .get_training_period(user, period_id)
            .await?;

        // Query activities within the period's date range
        let filters = ListActivitiesFilters::empty().set_date_range(Some(period.range()));

        let Ok(activities) = self
            .activity_repository
            .lock()
            .await
            .list_activities(user, &filters)
            .await
        else {
            return None;
        };

        // Filter activities by the period's sport filters
        let matching_activities: Vec<_> = activities
            .into_iter()
            .filter(|activity| period.matches(activity))
            .collect();

        Some(TrainingPeriodWithActivities::new(
            period,
            matching_activities,
        ))
    }
}

impl<TMR, AR> TrainingService<TMR, AR>
where
    TMR: TrainingRepository,
    AR: ActivityRepository,
{
    async fn compute_initial_values(
        &self,
        req: CreateTrainingMetricRequest,
        definition: TrainingMetricDefinition,
        initial_bins: Vec<DateRange>,
    ) {
        // Compute initial values over the user history
        let Ok(Some(history_range)) = self
            .activity_repository
            .lock()
            .await
            .get_user_history_date_range(req.user())
            .await
        else {
            return;
        };

        let bins = definition.granularity().bins_from_datetime(&history_range);

        // Iterate in reverse so that the most recent bins' values are available first
        for bin in bins.iter().rev() {
            if initial_bins.contains(bin) {
                continue;
            }
            self.compute_metric_values(&definition, bin, req.user())
                .await;
        }
    }
}

///////////////////////////////////////////////////////////////////
// MOCK IMPLEMENTATIONS FOR TESTING
///////////////////////////////////////////////////////////////////

#[cfg(test)]
pub mod test_utils {

    use mockall::mock;

    use crate::domain::{
        models::training::{TrainingMetricValue, TrainingPeriod, TrainingPeriodWithActivities},
        ports::{
            CreateTrainingPeriodError, CreateTrainingPeriodRequest, DeleteMetricError,
            GetDefinitionError, GetTrainingMetricValueError, GetTrainingMetricsDefinitionsError,
            SaveTrainingMetricError, SaveTrainingPeriodError, UpdateMetricError,
        },
    };

    use super::*;

    mock! {
        pub TrainingService {}

        impl Clone for TrainingService {
            fn clone(&self) -> Self;
        }

        impl ITrainingService for TrainingService {

            async fn create_metric(
                &self,
                req: CreateTrainingMetricRequest
            ) -> Result<TrainingMetricId, CreateTrainingMetricError>;

            async fn update_metrics_values(
                &self,
                req: UpdateMetricsValuesRequest,
            ) -> Result<(), ()>;

            async fn get_training_metrics(
                &self,
                user: &UserId,
            ) -> Vec<(TrainingMetricDefinition, TrainingMetricValues)>;

            async fn delete_metric(
                &self,
                req: DeleteTrainingMetricRequest,
            ) -> Result<(), DeleteTrainingMetricError>;

            async fn create_training_period(
                &self,
                req: CreateTrainingPeriodRequest,
            ) -> Result<TrainingPeriodId, CreateTrainingPeriodError>;

            async fn get_training_periods(
                &self,
                user: &UserId,
            ) -> Vec<TrainingPeriod>;

            async fn get_training_period(
                &self,
                user: &UserId,
                period: &TrainingPeriodId,
            ) -> Option<TrainingPeriod>;

            async fn get_training_period_with_activities(
                &self,
                user: &UserId,
                period: &TrainingPeriodId,
            ) -> Option<TrainingPeriodWithActivities>;
        }
    }

    impl MockTrainingService {
        pub fn test_default() -> Self {
            let mut mock = Self::new();

            mock.expect_create_metric()
                .returning(|_| Ok(TrainingMetricId::default()));
            mock.expect_update_metrics_values().returning(|_| Ok(()));
            mock.expect_get_training_metrics().returning(|_| vec![]);
            mock.expect_delete_metric().returning(|_| Ok(()));

            mock
        }
    }

    mock! {
        pub TrainingRepository {}

        impl Clone for TrainingRepository {
            fn clone(&self) -> Self;
        }

        impl TrainingRepository for TrainingRepository {
            async fn save_definition(
                &self,
                definition: TrainingMetricDefinition,
            ) -> Result<(), SaveTrainingMetricError>;

            async fn get_definitions(
                &self,
                user: &UserId,
            ) -> Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError>;

            async fn update_metric_values(
                &self,
                id: &TrainingMetricId,
                values: (String, TrainingMetricValue),
            ) -> Result<(), UpdateMetricError>;

            async fn get_metric_value(
                &self,
                id: &TrainingMetricId,
                bin_key: &str
            ) -> Result<Option<TrainingMetricValue>, GetTrainingMetricValueError>;

            async fn get_metric_values(
                &self,
                id: &TrainingMetricId,
            ) -> Result<TrainingMetricValues, GetTrainingMetricValueError>;

            async fn get_definition(
                &self,
                metric: &TrainingMetricId,
            ) -> Result<Option<TrainingMetricDefinition>, GetDefinitionError>;

            async fn delete_definition(
                &self,
                metric: &TrainingMetricId,
            ) -> Result<(), DeleteMetricError>;

            async fn save_training_period(
                &self,
                period: TrainingPeriod,
            ) -> Result<(), SaveTrainingPeriodError>;

            async fn get_training_periods(
                &self,
                user: &UserId,
            ) -> Vec<TrainingPeriod>;

            async fn get_training_period(
                &self,
                user: &UserId,
                period: &TrainingPeriodId,
            ) -> Option<TrainingPeriod>;
        }
    }
}

#[cfg(test)]
mod tests_training_metrics_service {
    use std::{collections::HashMap, sync::Arc};

    use anyhow::anyhow;
    use chrono::{DateTime, Days, FixedOffset, Utc};
    use tokio::sync::Mutex;

    use crate::domain::{
        models::{
            activity::{
                Activity, ActivityId, ActivityStartTime, ActivityStatistic, ActivityStatistics,
                ActivityTimeseries, ActivityWithTimeseries, Sport, TimeseriesActiveTime,
                TimeseriesTime,
            },
            training::{
                ActivityMetricSource, TrainingMetricAggregate, TrainingMetricDefinition,
                TrainingMetricFilters, TrainingMetricGranularity, TrainingMetricId,
                TrainingMetricValue, TrainingMetricValues,
            },
        },
        ports::{DateTimeRange, GetTrainingMetricsDefinitionsError, SaveTrainingMetricError},
        services::{
            activity::test_utils::MockActivityRepository,
            training::test_utils::MockTrainingRepository,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_create_metric_ok() {
        let mut repository = MockTrainingRepository::new();
        let background_repository = MockTrainingRepository::new();
        repository.expect_save_definition().returning(|_| Ok(()));
        repository
            .expect_clone()
            .return_once(move || background_repository);

        let mut activities = MockActivityRepository::new();
        let mut background_activities = MockActivityRepository::new();
        background_activities
            .expect_get_user_history_date_range()
            .returning(|_| {
                Ok(Some(DateTimeRange::new(
                    "2025-09-05T00:00:00+02:00"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                    Some(
                        "2025-09-05T00:00:00+02:00"
                            .parse::<DateTime<FixedOffset>>()
                            .unwrap(),
                    ),
                )))
            });
        background_activities
            .expect_list_activities_with_timeseries()
            .returning(|_, _| Ok(vec![]));
        activities
            .expect_clone()
            .return_once(move || background_activities);
        let service = TrainingService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty(),
            None,
        );

        let _ = service
            .create_metric(req)
            .await
            .expect("Should have return ok");
    }

    #[tokio::test]
    async fn test_create_metric_compute_initial_values() {
        let mut repository = MockTrainingRepository::new();
        let mut background_repository = MockTrainingRepository::new();
        repository.expect_save_definition().returning(|_| Ok(()));
        background_repository
            .expect_update_metric_values()
            .times(1)
            .returning(|_, _| Ok(()));
        repository
            .expect_clone()
            .return_once(move || background_repository);

        let mut activities = MockActivityRepository::new();
        let mut background_activities = MockActivityRepository::new();
        background_activities
            .expect_get_user_history_date_range()
            .returning(|_| {
                Ok(Some(DateTimeRange::new(
                    "2025-09-05T00:00:00+02:00"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                    Some(
                        "2025-09-05T00:00:00+02:00"
                            .parse::<DateTime<FixedOffset>>()
                            .unwrap(),
                    ),
                )))
            });
        background_activities
            .expect_list_activities_with_timeseries()
            .returning(|_, _| {
                Ok(vec![ActivityWithTimeseries::new(
                    Activity::new(
                        ActivityId::new(),
                        UserId::test_default(),
                        None,
                        ActivityStartTime::from_timestamp(1200).unwrap(),
                        Sport::Cycling,
                        ActivityStatistics::new(HashMap::from([(
                            ActivityStatistic::Calories,
                            12.,
                        )])),
                    ),
                    ActivityTimeseries::new(
                        TimeseriesTime::new(vec![]),
                        TimeseriesActiveTime::new(vec![]),
                        vec![],
                        vec![],
                    )
                    .unwrap(),
                )])
            });
        activities
            .expect_clone()
            .return_once(move || background_activities);
        let service = TrainingService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty(),
            None,
        );

        let _ = service
            .create_metric(req)
            .await
            .expect("Should have return ok");
    }

    #[tokio::test]
    async fn test_create_metric_compute_initial_values_with_initial_date_range() {
        let now = Utc::now();
        let mut repository = MockTrainingRepository::new();
        let mut background_repository = MockTrainingRepository::new();
        repository.expect_save_definition().returning(|_| Ok(()));
        repository
            .expect_update_metric_values()
            .times(2) // 2 day fron initial range
            .returning(|_, _| Ok(()));
        background_repository
            .expect_update_metric_values()
            .times(1) // 3 days in user history - 2 days fron initial range
            .returning(|_, _| Ok(()));
        repository
            .expect_clone()
            .return_once(move || background_repository);

        let mut activities = MockActivityRepository::new();
        let cloned_now = now.clone();
        activities
            .expect_get_user_history_date_range()
            .returning(move |_| {
                Ok(Some(DateTimeRange::new(
                    cloned_now
                        .checked_sub_days(Days::new(1))
                        .unwrap()
                        .fixed_offset(),
                    Some(
                        cloned_now
                            .checked_add_days(Days::new(1))
                            .unwrap()
                            .fixed_offset(),
                    ),
                )))
            });
        activities.expect_list_activities().returning(|_, _| {
            Ok(vec![Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(1200).unwrap(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::from([(ActivityStatistic::Calories, 12.)])),
            )])
        });
        let service = TrainingService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty(),
            Some(DateRange::new(
                now.date_naive(),
                now.date_naive().checked_add_days(Days::new(1)).unwrap(),
            )),
        );

        let _ = service
            .create_metric(req)
            .await
            .expect("Should have return ok");
    }

    #[tokio::test]
    async fn test_create_metric_fails_to_save_definition() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_save_definition()
            .returning(|_| Err(SaveTrainingMetricError::Unknown(anyhow!("error"))));
        repository.expect_update_metric_values().times(0);
        let mut activities = MockActivityRepository::new();
        activities.expect_list_activities_with_timeseries().times(0);
        let service = TrainingService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty(),
            None,
        );

        let _ = service
            .create_metric(req)
            .await
            .expect_err("Should have return an err");
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_when_get_definitions_err() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Err(GetTrainingMetricsDefinitionsError::Unknown(anyhow!(
                "an error"
            )))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let res = service.get_training_metrics(&UserId::test_default()).await;
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_def_without_values() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
            )])
        });
        repository
            .expect_get_metric_values()
            .returning(|_| Ok(TrainingMetricValues::new(HashMap::new())));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let res = service.get_training_metrics(&UserId::test_default()).await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
            )
        );
        assert!(value.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_map_def_with_its_values() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
            )])
        });
        repository.expect_get_metric_values().returning(|_| {
            Ok(TrainingMetricValues::new(HashMap::from([(
                "toto".to_string(),
                TrainingMetricValue::Max(0.3),
            )])))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let res = service.get_training_metrics(&UserId::test_default()).await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
            )
        );
        assert_eq!(*value.get("toto").unwrap(), TrainingMetricValue::Max(0.3));
    }

    #[tokio::test]
    async fn test_training_service_delete_metric_does_not_exist() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definition().returning(|_| Ok(None));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let req = DeleteTrainingMetricRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
        );

        let res = service.delete_metric(req).await;

        let Err(DeleteTrainingMetricError::MetricDoesNotExist(metric)) = res else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(metric, TrainingMetricId::from("test"));
    }

    #[tokio::test]
    async fn test_training_service_delete_metric_wrong_user() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definition().returning(|_| {
            Ok(Some(TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                "other_user".to_string().into(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
            )))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let req = DeleteTrainingMetricRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
        );

        let res = service.delete_metric(req).await;

        let Err(DeleteTrainingMetricError::UserDoesNotOwnTrainingMetric(user, metric)) = res else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(user, "user".to_string().into());
        assert_eq!(metric, TrainingMetricId::from("test"));
    }

    #[tokio::test]
    async fn test_training_service_delete_metric() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definition().returning(|_| {
            Ok(Some(TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                "user".to_string().into(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
            )))
        });
        repository
            .expect_delete_definition()
            .times(1)
            .withf(|id| id == &TrainingMetricId::from("test"))
            .returning(|_| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let req = DeleteTrainingMetricRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
        );

        let res = service.delete_metric(req).await;

        assert!(res.is_ok());
    }
}

#[cfg(test)]
mod test_training_service_period {
    use anyhow::anyhow;
    use chrono::NaiveDate;

    use crate::domain::{
        models::training::{TrainingPeriod, TrainingPeriodSports},
        ports::{CreateTrainingPeriodRequest, ListActivitiesError, SaveTrainingPeriodError},
        services::{
            activity::test_utils::MockActivityRepository,
            training::test_utils::MockTrainingRepository,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_training_service_create_training_period_ok() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_save_training_period()
            .times(1)
            .returning(|_| Ok(()));
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let req = CreateTrainingPeriodRequest::new(
            UserId::test_default(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            None,
            "test_period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        );

        let res = service.create_training_period(req).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_training_service_create_training_period_save_period_err() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_save_training_period()
            .times(1)
            .returning(|_| Err(SaveTrainingPeriodError::Unknown(anyhow!("repo error"))));
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let req = CreateTrainingPeriodRequest::new(
            UserId::test_default(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            None,
            "test_period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        );

        let res = service.create_training_period(req).await;

        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_not_found() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| None);
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let result = service
            .get_training_period_with_activities(&UserId::test_default(), &TrainingPeriodId::new())
            .await;

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_no_sport_filter() {
        use crate::domain::models::activity::{
            Activity, ActivityId, ActivityStartTime, ActivityStatistics, Sport,
        };

        // Create test activities with different sports and dates
        let activities = vec![
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-18T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Running,
                ActivityStatistics::default(),
            ),
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-19T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Cycling,
                ActivityStatistics::default(),
            ),
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-20T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Swimming,
                ActivityStatistics::default(),
            ),
        ];

        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            Some("2025-10-21".parse::<NaiveDate>().unwrap()),
            "Test Period".to_string(),
            TrainingPeriodSports::new(None), // No sport filter = all sports
            None,
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(move |_, _| Some(period));

        let mut activity_repository = MockActivityRepository::new();
        let activities_clone = activities.clone();
        activity_repository
            .expect_list_activities()
            .times(1)
            .returning(move |_, _| Ok(activities_clone.clone()));

        let service = TrainingService::new(
            training_repository,
            Arc::new(Mutex::new(activity_repository)),
        );

        let result = service
            .get_training_period_with_activities(&UserId::test_default(), &TrainingPeriodId::new())
            .await;

        assert!(result.is_some());
        let period_with_activities = result.unwrap();
        assert_eq!(period_with_activities.activities().len(), 3);
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_with_sport_filter() {
        use crate::domain::models::activity::{
            Activity, ActivityId, ActivityStartTime, ActivityStatistics, Sport,
        };
        use crate::domain::models::training::SportFilter;

        // Create test activities with different sports
        let activities = vec![
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-18T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Running,
                ActivityStatistics::default(),
            ),
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-19T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Cycling,
                ActivityStatistics::default(),
            ),
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-20T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Swimming,
                ActivityStatistics::default(),
            ),
        ];

        // Period with only Running filter
        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            Some("2025-10-21".parse::<NaiveDate>().unwrap()),
            "Running Period".to_string(),
            TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::Running)])),
            None,
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(move |_, _| Some(period));

        let mut activity_repository = MockActivityRepository::new();
        let activities_clone = activities.clone();
        activity_repository
            .expect_list_activities()
            .times(1)
            .returning(move |_, _| Ok(activities_clone.clone()));

        let service = TrainingService::new(
            training_repository,
            Arc::new(Mutex::new(activity_repository)),
        );

        let result = service
            .get_training_period_with_activities(&UserId::test_default(), &TrainingPeriodId::new())
            .await;

        assert!(result.is_some());
        let period_with_activities = result.unwrap();
        // Should only include Running activity
        assert_eq!(period_with_activities.activities().len(), 1);
        assert_eq!(
            period_with_activities.activities()[0].sport(),
            &Sport::Running
        );
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_with_category_filter() {
        use crate::domain::models::activity::{
            Activity, ActivityId, ActivityStartTime, ActivityStatistics, Sport, SportCategory,
        };
        use crate::domain::models::training::SportFilter;

        // Create test activities with different sports in the Running category
        let activities = vec![
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-18T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Running,
                ActivityStatistics::default(),
            ),
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-19T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::TrailRunning,
                ActivityStatistics::default(),
            ),
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-20T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Cycling,
                ActivityStatistics::default(),
            ),
        ];

        // Period with Running category filter (should match Running and TrailRunning)
        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            Some("2025-10-21".parse::<NaiveDate>().unwrap()),
            "Running Category Period".to_string(),
            TrainingPeriodSports::new(Some(vec![SportFilter::SportCategory(
                SportCategory::Running,
            )])),
            None,
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(move |_, _| Some(period));

        let mut activity_repository = MockActivityRepository::new();
        let activities_clone = activities.clone();
        activity_repository
            .expect_list_activities()
            .times(1)
            .returning(move |_, _| Ok(activities_clone.clone()));

        let service = TrainingService::new(
            training_repository,
            Arc::new(Mutex::new(activity_repository)),
        );

        let result = service
            .get_training_period_with_activities(&UserId::test_default(), &TrainingPeriodId::new())
            .await;

        assert!(result.is_some());
        let period_with_activities = result.unwrap();
        // Should include Running and TrailRunning, but not Cycling
        assert_eq!(period_with_activities.activities().len(), 2);
        assert!(
            period_with_activities
                .activities()
                .iter()
                .any(|a| a.sport() == &Sport::Running)
        );
        assert!(
            period_with_activities
                .activities()
                .iter()
                .any(|a| a.sport() == &Sport::TrailRunning)
        );
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_date_filtering() {
        use crate::domain::models::activity::{
            Activity, ActivityId, ActivityStartTime, ActivityStatistics, Sport,
        };

        // Create activities with dates both inside and outside the period
        let activities = vec![
            // Before period
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-16T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Running,
                ActivityStatistics::default(),
            ),
            // Inside period
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-18T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Running,
                ActivityStatistics::default(),
            ),
            // After period
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    "2025-10-22T10:00:00Z"
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Running,
                ActivityStatistics::default(),
            ),
        ];

        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            Some("2025-10-21".parse::<NaiveDate>().unwrap()),
            "Test Period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(move |_, _| Some(period));

        let mut activity_repository = MockActivityRepository::new();
        let activities_clone = activities.clone();
        activity_repository
            .expect_list_activities()
            .times(1)
            .returning(move |_, _| Ok(activities_clone.clone()));

        let service = TrainingService::new(
            training_repository,
            Arc::new(Mutex::new(activity_repository)),
        );

        let result = service
            .get_training_period_with_activities(&UserId::test_default(), &TrainingPeriodId::new())
            .await;

        assert!(result.is_some());
        let period_with_activities = result.unwrap();
        // Should only include the activity inside the period (2025-10-18)
        assert_eq!(period_with_activities.activities().len(), 1);
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_repository_error() {
        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            Some("2025-10-21".parse::<NaiveDate>().unwrap()),
            "Test Period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(move |_, _| Some(period));

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_list_activities()
            .times(1)
            .returning(|_, _| Err(ListActivitiesError::Unknown(anyhow!("database error"))));

        let service = TrainingService::new(
            training_repository,
            Arc::new(Mutex::new(activity_repository)),
        );

        let result = service
            .get_training_period_with_activities(&UserId::test_default(), &TrainingPeriodId::new())
            .await;

        // Should return None when repository fails
        assert!(result.is_none());
    }
}
