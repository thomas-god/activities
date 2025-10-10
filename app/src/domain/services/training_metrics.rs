use std::sync::Arc;

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::domain::{
    models::{
        UserId,
        training_metrics::{
            ComputeMetricRequirement, TrainingMetricDefinition, TrainingMetricId,
            TrainingMetricValues,
        },
    },
    ports::{
        ActivityRepository, CreateTrainingMetricError, CreateTrainingMetricRequest, DateRange,
        DeleteTrainingMetricError, DeleteTrainingMetricRequest, ITrainingMetricService,
        ListActivitiesFilters, TrainingMetricsRepository, UpdateMetricsValuesRequest,
    },
};

///////////////////////////////////////////////////////////////////
/// TRAINING METRICS SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Constructor)]
pub struct TrainingMetricService<TMR, AR>
where
    TMR: TrainingMetricsRepository,
    AR: ActivityRepository,
{
    metrics_repository: TMR,
    activity_repository: Arc<Mutex<AR>>,
}

impl<TMR, AR> TrainingMetricService<TMR, AR>
where
    TMR: TrainingMetricsRepository,
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
                .metrics_repository
                .update_metric_values(definition.id(), (key, value))
                .await;
        }
    }
}

impl<TMR, AR> ITrainingMetricService for TrainingMetricService<TMR, AR>
where
    TMR: TrainingMetricsRepository,
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
        );
        self.metrics_repository
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
            .metrics_repository
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
                    .metrics_repository
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
                    .metrics_repository
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
        let Ok(definitions) = self.metrics_repository.get_definitions(user).await else {
            return vec![];
        };

        let mut res = vec![];
        for definition in definitions {
            let values = self
                .metrics_repository
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
        let Some(definition) = self.metrics_repository.get_definition(req.metric()).await? else {
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

        self.metrics_repository
            .delete_definition(req.metric())
            .await?;

        Ok(())
    }
}

impl<TMR, AR> TrainingMetricService<TMR, AR>
where
    TMR: TrainingMetricsRepository,
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
        models::training_metrics::TrainingMetricValue,
        ports::{
            DeleteMetricError, GetDefinitionError, GetTrainingMetricValueError,
            GetTrainingMetricsDefinitionsError, SaveTrainingMetricError, UpdateMetricError,
        },
    };

    use super::*;

    mock! {
        pub TrainingMetricService {}

        impl Clone for TrainingMetricService {
            fn clone(&self) -> Self;
        }

        impl ITrainingMetricService for TrainingMetricService {

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
        }
    }

    impl MockTrainingMetricService {
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
        pub TrainingMetricsRepository {}

        impl Clone for TrainingMetricsRepository {
            fn clone(&self) -> Self;
        }

        impl TrainingMetricsRepository for TrainingMetricsRepository {
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
                ActivityTimeseries, ActivityWithTimeseries, Sport, TimeseriesTime,
            },
            training_metrics::{
                ActivityMetricSource, TrainingMetricAggregate, TrainingMetricDefinition,
                TrainingMetricGranularity, TrainingMetricId, TrainingMetricValue,
                TrainingMetricValues,
            },
        },
        ports::{DateTimeRange, GetTrainingMetricsDefinitionsError, SaveTrainingMetricError},
        services::{
            activity::test_utils::MockActivityRepository,
            training_metrics::test_utils::MockTrainingMetricsRepository,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_create_metric_ok() {
        let mut repository = MockTrainingMetricsRepository::new();
        let background_repository = MockTrainingMetricsRepository::new();
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
        let service = TrainingMetricService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            None,
        );

        let _ = service
            .create_metric(req)
            .await
            .expect("Should have return ok");
    }

    #[tokio::test]
    async fn test_create_metric_compute_initial_values() {
        let mut repository = MockTrainingMetricsRepository::new();
        let mut background_repository = MockTrainingMetricsRepository::new();
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
                    ActivityTimeseries::new(TimeseriesTime::new(vec![]), vec![]),
                )])
            });
        activities
            .expect_clone()
            .return_once(move || background_activities);
        let service = TrainingMetricService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
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
        let mut repository = MockTrainingMetricsRepository::new();
        let mut background_repository = MockTrainingMetricsRepository::new();
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
        let service = TrainingMetricService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
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
        let mut repository = MockTrainingMetricsRepository::new();
        repository
            .expect_save_definition()
            .returning(|_| Err(SaveTrainingMetricError::Unknown(anyhow!("error"))));
        repository.expect_update_metric_values().times(0);
        let mut activities = MockActivityRepository::new();
        activities.expect_list_activities_with_timeseries().times(0);
        let service = TrainingMetricService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            None,
        );

        let _ = service
            .create_metric(req)
            .await
            .expect_err("Should have return an err");
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_when_get_definitions_err() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Err(GetTrainingMetricsDefinitionsError::Unknown(anyhow!(
                "an error"
            )))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let res = service.get_training_metrics(&UserId::test_default()).await;
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_def_without_values() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )])
        });
        repository
            .expect_get_metric_values()
            .returning(|_| Ok(TrainingMetricValues::new(HashMap::new())));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

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
            )
        );
        assert!(value.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_map_def_with_its_values() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )])
        });
        repository.expect_get_metric_values().returning(|_| {
            Ok(TrainingMetricValues::new(HashMap::from([(
                "toto".to_string(),
                TrainingMetricValue::Max(0.3),
            )])))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

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
            )
        );
        assert_eq!(*value.get("toto").unwrap(), TrainingMetricValue::Max(0.3));
    }

    #[tokio::test]
    async fn test_training_service_delete_metric_does_not_exist() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definition().returning(|_| Ok(None));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

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
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definition().returning(|_| {
            Ok(Some(TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                "other_user".to_string().into(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

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
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definition().returning(|_| {
            Ok(Some(TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                "user".to_string().into(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )))
        });
        repository
            .expect_delete_definition()
            .times(1)
            .withf(|id| id == &TrainingMetricId::from("test"))
            .returning(|_| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let req = DeleteTrainingMetricRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
        );

        let res = service.delete_metric(req).await;

        assert!(res.is_ok());
    }
}
