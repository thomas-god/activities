use std::sync::Arc;

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::domain::{
    models::{
        UserId,
        training::{
            ComputeMetricRequirement, TrainingMetric, TrainingMetricDefinition, TrainingMetricId,
            TrainingMetricValues, TrainingNote, TrainingNoteContent, TrainingNoteDate,
            TrainingNoteId, TrainingNoteTitle, TrainingPeriodId,
        },
    },
    ports::{
        ActivityRepository, ComputeTrainingMetricValuesError, CreateTrainingMetricError,
        CreateTrainingMetricRequest, CreateTrainingNoteError, CreateTrainingNoteRequest,
        CreateTrainingPeriodError, CreateTrainingPeriodRequest, DateRange,
        DeleteTrainingMetricError, DeleteTrainingMetricRequest, DeleteTrainingNoteError,
        DeleteTrainingPeriodError, DeleteTrainingPeriodRequest, GetTrainingMetricValuesError,
        GetTrainingNoteError, ITrainingService, ListActivitiesFilters, TrainingRepository,
        UpdateTrainingNoteError, UpdateTrainingPeriodDatesError, UpdateTrainingPeriodDatesRequest,
        UpdateTrainingPeriodNameError, UpdateTrainingPeriodNameRequest,
        UpdateTrainingPeriodNoteError, UpdateTrainingPeriodNoteRequest,
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
            req.user().clone(),
            req.source().clone(),
            req.granularity().clone(),
            req.aggregate().clone(),
            req.filters().clone(),
            req.group_by().clone(),
        );
        let training_metric = TrainingMetric::new(id.clone(), definition);
        self.training_repository
            .save_training_metric_definition(training_metric.clone())
            .await?;

        Ok(id)
    }

    /// Compute training metric values from a [TrainingMetricDefinition] and a [DateRange] using
    /// activities within the [DateRange].
    async fn compute_training_metric_values(
        &self,
        definition: &TrainingMetricDefinition,
        date_range: &DateRange,
    ) -> Result<TrainingMetricValues, ComputeTrainingMetricValuesError> {
        let values = match definition.source_requirement() {
            ComputeMetricRequirement::ActivityWithTimeseries => {
                let activities = self
                    .activity_repository
                    .lock()
                    .await
                    .list_activities_with_timeseries(
                        definition.user(),
                        &ListActivitiesFilters::empty().set_date_range(Some(date_range.clone())),
                    )
                    .await
                    .map_err(|err| ComputeTrainingMetricValuesError::Unknown(err.into()))?;

                definition.compute_values_from_timeseries(&activities)
            }
            ComputeMetricRequirement::Activity => {
                let activities = self
                    .activity_repository
                    .lock()
                    .await
                    .list_activities(
                        definition.user(),
                        &ListActivitiesFilters::empty().set_date_range(Some(date_range.clone())),
                    )
                    .await
                    .map_err(|err| ComputeTrainingMetricValuesError::Unknown(err.into()))?;

                definition.compute_values(&activities)
            }
        };

        Ok(values)
    }

    /// Get training metric values for all registered training metrics for a given user.
    async fn get_training_metrics_values(
        &self,
        user: &UserId,
        date_range: &Option<DateRange>,
    ) -> Vec<(TrainingMetric, TrainingMetricValues)> {
        let Ok(metrics) = self.training_repository.get_metrics(user).await else {
            return vec![];
        };

        let mut res = vec![];
        for metric in metrics {
            // Align the date range to the metric's granularity to ensure complete bins
            // For example, if granularity is Weekly and date_range starts on Wednesday,
            // we align it to the Monday of that week to include the full week's data
            let aligned_date_range = date_range.as_ref().map(|range| {
                let bins = metric.definition().granularity().bins(range);
                if let (Some(first), Some(last)) = (bins.first(), bins.last()) {
                    DateRange::new(*first.start(), *last.end())
                } else {
                    range.clone()
                }
            });

            let values = match &aligned_date_range {
                Some(range) => self
                    .compute_training_metric_values(metric.definition(), range)
                    .await
                    .unwrap_or_default(),
                None => {
                    // If no date range specified, compute over entire user history
                    let range = self
                        .activity_repository
                        .lock()
                        .await
                        .get_user_history_date_range(user)
                        .await;
                    match range {
                        Ok(Some(history_range)) => {
                            let bins = metric
                                .definition()
                                .granularity()
                                .bins_from_datetime(&history_range);
                            if let (Some(first), Some(last)) = (bins.first(), bins.last()) {
                                let full_range = DateRange::new(*first.start(), *last.end());
                                self.compute_training_metric_values(
                                    metric.definition(),
                                    &full_range,
                                )
                                .await
                                .unwrap_or_default()
                            } else {
                                TrainingMetricValues::default()
                            }
                        }
                        _ => TrainingMetricValues::default(),
                    }
                }
            };
            res.push((metric.clone(), values))
        }

        res
    }

    /// Get training metric values for a given registered training metric.
    async fn get_training_metric_values(
        &self,
        user: &UserId,
        metric_id: &TrainingMetricId,
        date_range: &DateRange,
    ) -> Result<TrainingMetricValues, GetTrainingMetricValuesError> {
        let definition = self
            .training_repository
            .get_definition(metric_id)
            .await
            .map_err(|err| GetTrainingMetricValuesError::Unknown(err.into()))?;

        match definition {
            Some(def) if def.user() == user => self
                .compute_training_metric_values(&def, date_range)
                .await
                .map_err(GetTrainingMetricValuesError::from),
            Some(_) => Err(GetTrainingMetricValuesError::Unknown(anyhow::anyhow!(
                "Training metric does not belong to user"
            ))),
            None => Err(GetTrainingMetricValuesError::TrainingMetricDoesNotExists(
                metric_id.clone(),
            )),
        }
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
        let filters =
            ListActivitiesFilters::empty().set_date_range(Some(period.range_default_tomorrow()));

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

    async fn delete_training_period(
        &self,
        req: DeleteTrainingPeriodRequest,
    ) -> Result<(), DeleteTrainingPeriodError> {
        // Get the period to verify it exists and check ownership
        let Some(period) = self
            .training_repository
            .get_training_period(req.user(), req.period_id())
            .await
        else {
            return Err(DeleteTrainingPeriodError::PeriodDoesNotExist(
                req.period_id().clone(),
            ));
        };

        // Verify user owns the period
        if period.user() != req.user() {
            return Err(DeleteTrainingPeriodError::UserDoesNotOwnPeriod(
                req.user().clone(),
                req.period_id().clone(),
            ));
        }

        // Delete the period
        self.training_repository
            .delete_training_period(req.period_id())
            .await
            .map_err(DeleteTrainingPeriodError::Unknown)?;

        Ok(())
    }

    async fn update_training_period_name(
        &self,
        req: UpdateTrainingPeriodNameRequest,
    ) -> Result<(), UpdateTrainingPeriodNameError> {
        // Get the period to verify it exists and check ownership
        let Some(period) = self
            .training_repository
            .get_training_period(req.user(), req.period_id())
            .await
        else {
            return Err(UpdateTrainingPeriodNameError::PeriodDoesNotExist(
                req.period_id().clone(),
            ));
        };

        // Verify user owns the period
        if period.user() != req.user() {
            return Err(UpdateTrainingPeriodNameError::UserDoesNotOwnPeriod(
                req.user().clone(),
                req.period_id().clone(),
            ));
        }

        // Update the period name
        self.training_repository
            .update_training_period_name(req.period_id(), req.name().to_string())
            .await
            .map_err(UpdateTrainingPeriodNameError::Unknown)?;

        Ok(())
    }

    async fn update_training_period_note(
        &self,
        req: UpdateTrainingPeriodNoteRequest,
    ) -> Result<(), UpdateTrainingPeriodNoteError> {
        // Get the period to verify it exists and check ownership
        let Some(period) = self
            .training_repository
            .get_training_period(req.user(), req.period_id())
            .await
        else {
            return Err(UpdateTrainingPeriodNoteError::PeriodDoesNotExist(
                req.period_id().clone(),
            ));
        };

        // Verify user owns the period
        if period.user() != req.user() {
            return Err(UpdateTrainingPeriodNoteError::UserDoesNotOwnPeriod(
                req.user().clone(),
                req.period_id().clone(),
            ));
        }

        // Update the period note
        self.training_repository
            .update_training_period_note(req.period_id(), req.note().clone())
            .await
            .map_err(UpdateTrainingPeriodNoteError::Unknown)?;

        Ok(())
    }

    async fn update_training_period_dates(
        &self,
        req: UpdateTrainingPeriodDatesRequest,
    ) -> Result<(), UpdateTrainingPeriodDatesError> {
        // Get the period to verify it exists and check ownership
        let Some(period) = self
            .training_repository
            .get_training_period(req.user(), req.period_id())
            .await
        else {
            return Err(UpdateTrainingPeriodDatesError::PeriodDoesNotExist(
                req.period_id().clone(),
            ));
        };

        // Verify user owns the period
        if period.user() != req.user() {
            return Err(UpdateTrainingPeriodDatesError::UserDoesNotOwnPeriod(
                req.user().clone(),
                req.period_id().clone(),
            ));
        }

        // Validate dates
        if let Some(end_date) = req.end()
            && req.start() > end_date
        {
            return Err(UpdateTrainingPeriodDatesError::EndDateBeforeStartDate);
        }

        // Update the period dates
        self.training_repository
            .update_training_period_dates(req.period_id(), *req.start(), req.end().clone())
            .await
            .map_err(UpdateTrainingPeriodDatesError::Unknown)?;

        Ok(())
    }

    async fn create_training_note(
        &self,
        req: CreateTrainingNoteRequest,
    ) -> Result<TrainingNoteId, CreateTrainingNoteError> {
        use crate::domain::models::training::{TrainingNote, TrainingNoteId};

        // Create the note
        let note_id = TrainingNoteId::new();
        let note = TrainingNote::new(
            note_id.clone(),
            req.user().clone(),
            req.title().clone(),
            req.content().clone(),
            req.date().clone(),
            chrono::Utc::now().into(),
        );

        // Save to repository
        self.training_repository
            .save_training_note(note)
            .await
            .map_err(|err| CreateTrainingNoteError::Unknown(err.into()))?;

        Ok(note_id)
    }

    async fn get_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
    ) -> Result<Option<TrainingNote>, GetTrainingNoteError> {
        let note = self.training_repository.get_training_note(note_id).await?;

        // Verify the note belongs to the user
        if let Some(ref n) = note
            && n.user() != user
        {
            return Ok(None);
        }

        Ok(note)
    }

    async fn get_training_notes(
        &self,
        user: &UserId,
    ) -> Result<Vec<TrainingNote>, GetTrainingNoteError> {
        self.training_repository.get_training_notes(user).await
    }

    async fn update_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
        title: Option<TrainingNoteTitle>,
        content: TrainingNoteContent,
        date: TrainingNoteDate,
    ) -> Result<(), UpdateTrainingNoteError> {
        // Verify the note exists and belongs to the user
        let note = self
            .training_repository
            .get_training_note(note_id)
            .await
            .map_err(|err| UpdateTrainingNoteError::Unknown(err.into()))?;

        match note {
            Some(n) if n.user() == user => {
                self.training_repository
                    .update_training_note(note_id, title, content, date)
                    .await?;
                Ok(())
            }
            _ => Err(UpdateTrainingNoteError::Unknown(anyhow::anyhow!(
                "Training note not found or unauthorized"
            ))),
        }
    }

    async fn delete_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
    ) -> Result<(), DeleteTrainingNoteError> {
        // Verify the note exists and belongs to the user
        let note = self
            .training_repository
            .get_training_note(note_id)
            .await
            .map_err(|err| DeleteTrainingNoteError::Unknown(err.into()))?;

        match note {
            Some(n) if n.user() == user => {
                self.training_repository
                    .delete_training_note(note_id)
                    .await?;
                Ok(())
            }
            _ => Err(DeleteTrainingNoteError::Unknown(anyhow::anyhow!(
                "Training note not found or unauthorized"
            ))),
        }
    }
}

///////////////////////////////////////////////////////////////////
// MOCK IMPLEMENTATIONS FOR TESTING
///////////////////////////////////////////////////////////////////

#[cfg(test)]
pub mod test_utils {

    use chrono::NaiveDate;
    use mockall::mock;

    use crate::domain::{
        models::training::{
            TrainingNote, TrainingNoteContent, TrainingPeriod, TrainingPeriodWithActivities,
        },
        ports::{
            CreateTrainingPeriodError, CreateTrainingPeriodRequest, DeleteMetricError,
            DeleteTrainingNoteError, DeleteTrainingPeriodError, DeleteTrainingPeriodRequest,
            GetDefinitionError, GetTrainingMetricsDefinitionsError, GetTrainingNoteError,
            SaveTrainingMetricError, SaveTrainingNoteError, SaveTrainingPeriodError,
            UpdateTrainingNoteError,
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

            async fn get_training_metrics_values(
                &self,
                user: &UserId,
                date_range: &Option<DateRange>,
            ) -> Vec<(TrainingMetric, TrainingMetricValues)>;

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

            async fn delete_training_period(
                &self,
                req: DeleteTrainingPeriodRequest,
            ) -> Result<(), DeleteTrainingPeriodError>;

            async fn update_training_period_name(
                &self,
                req: UpdateTrainingPeriodNameRequest,
            ) -> Result<(), UpdateTrainingPeriodNameError>;

            async fn update_training_period_note(
                &self,
                req: UpdateTrainingPeriodNoteRequest,
            ) -> Result<(), UpdateTrainingPeriodNoteError>;

            async fn update_training_period_dates(
                &self,
                req: UpdateTrainingPeriodDatesRequest,
            ) -> Result<(), UpdateTrainingPeriodDatesError>;

            async fn create_training_note(
                &self,
                req: CreateTrainingNoteRequest,
            ) -> Result<TrainingNoteId, CreateTrainingNoteError>;

            async fn get_training_note(
                &self,
                user: &UserId,
                note_id: &TrainingNoteId,
            ) -> Result<Option<TrainingNote>, GetTrainingNoteError>;

            async fn get_training_notes(
                &self,
                user: &UserId,
            ) -> Result<Vec<TrainingNote>, GetTrainingNoteError>;

            async fn update_training_note(
                &self,
                user: &UserId,
                note_id: &TrainingNoteId,
                title: Option<TrainingNoteTitle>,
                content: TrainingNoteContent,
                date: TrainingNoteDate,
            ) -> Result<(), UpdateTrainingNoteError>;

            async fn delete_training_note(
                &self,
                user: &UserId,
                note_id: &TrainingNoteId,
            ) -> Result<(), DeleteTrainingNoteError>;

            async fn get_training_metric_values(
                &self,
                user: &UserId,
                metric_id: &TrainingMetricId,
                date_range: &DateRange,
            ) -> Result<TrainingMetricValues, GetTrainingMetricValuesError>;

            async fn compute_training_metric_values(
                &self,
                definition: &TrainingMetricDefinition,
                date_range: &DateRange,
            ) -> Result<TrainingMetricValues, ComputeTrainingMetricValuesError>;
        }
    }

    impl MockTrainingService {
        pub fn test_default() -> Self {
            let mut mock = Self::new();

            mock.expect_create_metric()
                .returning(|_| Ok(TrainingMetricId::default()));
            mock.expect_get_training_metrics_values()
                .returning(|_, _| vec![]);
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
            async fn save_training_metric_definition(
                &self,
                metric: TrainingMetric,
            ) -> Result<(), SaveTrainingMetricError>;

            async fn get_metrics(
                &self,
                user: &UserId,
            ) -> Result<Vec<TrainingMetric>, GetTrainingMetricsDefinitionsError>;

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

            async fn delete_training_period(
                &self,
                period_id: &TrainingPeriodId,
            ) -> Result<(), anyhow::Error>;

            async fn update_training_period_name(
                &self,
                period_id: &TrainingPeriodId,
                name: String,
            ) -> Result<(), anyhow::Error>;

            async fn update_training_period_note(
                &self,
                period_id: &TrainingPeriodId,
                note: Option<String>,
            ) -> Result<(), anyhow::Error>;

            async fn update_training_period_dates(
                &self,
                period_id: &TrainingPeriodId,
                start: NaiveDate,
                end: Option<NaiveDate>,
            ) -> Result<(), anyhow::Error>;

            async fn save_training_note(
                &self,
                note: TrainingNote,
            ) -> Result<(), SaveTrainingNoteError>;

            async fn get_training_note(
                &self,
                note_id: &TrainingNoteId,
            ) -> Result<Option<TrainingNote>, GetTrainingNoteError>;

            async fn get_training_notes(
                &self,
                user: &UserId,
            ) -> Result<Vec<TrainingNote>, GetTrainingNoteError>;

            async fn update_training_note(
                &self,
                note_id: &TrainingNoteId,
                title: Option<TrainingNoteTitle>,
                content: TrainingNoteContent,
                date: TrainingNoteDate,
            ) -> Result<(), UpdateTrainingNoteError>;

            async fn delete_training_note(
                &self,
                note_id: &TrainingNoteId,
            ) -> Result<(), DeleteTrainingNoteError>;
        }
    }
}

#[cfg(test)]
mod tests_training_metrics_service {

    use anyhow::anyhow;
    use chrono::{DateTime, FixedOffset};
    use tokio::sync::Mutex;

    use super::*;
    use crate::domain::models::activity::{
        Activity, ActivityId, ActivityStartTime, ActivityStatistics, Sport,
    };

    use crate::domain::ports::DateRange;
    use crate::domain::{
        models::{
            activity::ActivityStatistic,
            training::{
                ActivityMetricSource, TrainingMetricAggregate, TrainingMetricDefinition,
                TrainingMetricFilters, TrainingMetricGranularity, TrainingMetricGroupBy,
                TrainingMetricId,
            },
        },
        ports::{DateTimeRange, GetTrainingMetricsDefinitionsError, SaveTrainingMetricError},
        services::{
            activity::test_utils::MockActivityRepository,
            training::test_utils::MockTrainingRepository,
        },
    };
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    #[tokio::test]
    async fn test_create_metric_ok() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_save_training_metric_definition()
            .returning(|_| Ok(()));
        let activities = MockActivityRepository::new();

        let service = TrainingService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty(),
            TrainingMetricGroupBy::none(),
            None,
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
            .expect_save_training_metric_definition()
            .returning(|_| Err(SaveTrainingMetricError::Unknown(anyhow!("error"))));
        let activities = MockActivityRepository::new();
        let service = TrainingService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty(),
            TrainingMetricGroupBy::none(),
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
        repository.expect_get_metrics().returning(|_| {
            Err(GetTrainingMetricsDefinitionsError::Unknown(anyhow!(
                "an error"
            )))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let res = service
            .get_training_metrics_values(&UserId::test_default(), &None)
            .await;
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_def_without_values() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_metrics().returning(|_| {
            Ok(vec![TrainingMetric::new(
                TrainingMetricId::from("test"),
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Average,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )])
        });

        let mut activity_repository = MockActivityRepository::default();
        // When no date range is specified, it should query the user's history
        activity_repository
            .expect_get_user_history_date_range()
            .returning(|_| Ok(None)); // No history, so no values
        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(repository, activity_repository);

        let res = service
            .get_training_metrics_values(&UserId::test_default(), &None)
            .await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetric::new(
                TrainingMetricId::from("test"),
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Average,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                )
            )
        );
        assert!(value.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_map_def_with_its_values() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_metrics().returning(|_| {
            Ok(vec![TrainingMetric::new(
                TrainingMetricId::from("test"),
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )])
        });

        let mut activity_repository = MockActivityRepository::default();
        // When no date range is specified, query user history
        activity_repository
            .expect_get_user_history_date_range()
            .returning(|_| {
                Ok(Some(DateTimeRange::new(
                    "2025-09-24T00:00:00+00:00"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                    Some(
                        "2025-09-24T23:59:59+00:00"
                            .parse::<DateTime<FixedOffset>>()
                            .unwrap(),
                    ),
                )))
            });
        // Then it should list activities in that range
        activity_repository
            .expect_list_activities()
            .returning(|_, _| {
                // Use default stats - the test just verifies that values are computed from activities
                let stats = ActivityStatistics::default();
                Ok(vec![Activity::new(
                    ActivityId::from("test"),
                    UserId::test_default(),
                    None,
                    ActivityStartTime::new(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                        )
                        .and_utc()
                        .fixed_offset(),
                    ),
                    Sport::Running,
                    stats,
                    None,
                    None,
                    None,
                    None,
                )])
            });

        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(repository, activity_repository);

        let res = service
            .get_training_metrics_values(&UserId::test_default(), &None)
            .await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetric::new(
                TrainingMetricId::from("test"),
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                )
            )
        );
        // With default stats (no calories), should have empty values
        assert!(value.is_empty());
    }

    #[tokio::test]
    async fn test_training_service_get_metrics_with_date_range() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_metrics().returning(|_| {
            Ok(vec![TrainingMetric::new(
                TrainingMetricId::from("test"),
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Average,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )])
        });

        let mut activity_repository = MockActivityRepository::default();
        // Should list activities in the date range
        activity_repository
            .expect_list_activities()
            .withf(|_, filters| {
                // The date range should be aligned to full days (Sep 24-26)
                filters.date_range().is_some()
                    && filters.date_range().as_ref().unwrap().start()
                        == &NaiveDate::from_ymd_opt(2025, 9, 24).unwrap()
                    && filters.date_range().as_ref().unwrap().end()
                        == &NaiveDate::from_ymd_opt(2025, 9, 26).unwrap()
            })
            .returning(|_, _| {
                let stats = ActivityStatistics::default();
                Ok(vec![Activity::new(
                    ActivityId::from("test"),
                    UserId::test_default(),
                    None,
                    ActivityStartTime::new(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                        )
                        .and_utc()
                        .fixed_offset(),
                    ),
                    Sport::Running,
                    stats,
                    None,
                    None,
                    None,
                    None,
                )])
            });

        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(repository, activity_repository);

        let date_range = Some(DateRange::new(
            NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
            NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
        ));

        let res = service
            .get_training_metrics_values(&UserId::test_default(), &date_range)
            .await;

        assert_eq!(res.len(), 1);
        let (_, values) = res.first().unwrap();
        // With default stats, values will be empty but the test verifies the flow works
        assert!(values.is_empty());
    }

    #[tokio::test]
    async fn test_training_service_get_metrics_aligns_date_range_to_granularity() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_metrics().returning(|_| {
            Ok(vec![TrainingMetric::new(
                TrainingMetricId::from("test"),
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                    TrainingMetricGranularity::Weekly, // Weekly granularity
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )])
        });

        let mut activity_repository = MockActivityRepository::default();
        // The date range will be aligned to week boundaries (Monday)
        // Input: Wed Sep 24 to Thu Sep 25, 2025
        // Should be aligned to: Mon Sep 22 to Mon Sep 29, 2025
        activity_repository
            .expect_list_activities()
            .withf(|_, filters| {
                let Some(range) = filters.date_range() else {
                    return false;
                };
                // Verify that the range starts on Monday (Sep 22) and ends on following Monday (Sep 29)
                *range.start() == NaiveDate::from_ymd_opt(2025, 9, 22).unwrap()
                    && *range.end() == NaiveDate::from_ymd_opt(2025, 9, 29).unwrap()
            })
            .returning(|_, _| {
                let stats = ActivityStatistics::default();
                Ok(vec![Activity::new(
                    ActivityId::from("test"),
                    UserId::test_default(),
                    None,
                    ActivityStartTime::new(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                        )
                        .and_utc()
                        .fixed_offset(),
                    ),
                    Sport::Running,
                    stats,
                    None,
                    None,
                    None,
                    None,
                )])
            });

        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(repository, activity_repository);

        // Input date range: Wednesday to Thursday (mid-week)
        let date_range = Some(DateRange::new(
            NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(), // Wednesday
            NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(), // Thursday
        ));

        let res = service
            .get_training_metrics_values(&UserId::test_default(), &date_range)
            .await;

        assert_eq!(res.len(), 1);
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
                "other_user".to_string().into(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
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
                "user".to_string().into(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
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

    // // Helper function to create a test activity with given timestamp and statistics
    // fn create_test_activity(
    //     timestamp: usize,
    //     calories: Option<f64>,
    //     distance: Option<f64>,
    // ) -> ActivityWithTimeseries {
    //     create_test_activity_with_sport(timestamp, calories, distance, Sport::Running)
    // }

    // // Helper function to create a test activity with given timestamp, statistics, and sport
    // fn create_test_activity_with_sport(
    //     timestamp: usize,
    //     calories: Option<f64>,
    //     distance: Option<f64>,
    //     sport: Sport,
    // ) -> ActivityWithTimeseries {
    //     let mut stats_map = HashMap::new();
    //     if let Some(cal) = calories {
    //         stats_map.insert(ActivityStatistic::Calories, cal);
    //     }
    //     if let Some(dist) = distance {
    //         stats_map.insert(ActivityStatistic::Distance, dist);
    //     }

    //     ActivityWithTimeseries::new(
    //         Activity::new(
    //             ActivityId::new(),
    //             UserId::test_default(),
    //             None,
    //             ActivityStartTime::from_timestamp(timestamp).unwrap(),
    //             sport,
    //             ActivityStatistics::new(stats_map),
    //             None,
    //             None,
    //             None,
    //             None,
    //         ),
    //         ActivityTimeseries::default(),
    //     )
    // }
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
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
    async fn test_get_training_period_with_activities_open_ended_includes_today() {
        use crate::domain::models::activity::{
            Activity, ActivityId, ActivityStartTime, ActivityStatistics, Sport,
        };
        use chrono::{Days, Utc};

        let today = Utc::now().date_naive();
        let yesterday = today - Days::new(1);

        // Create activities: yesterday and today
        // Note: Tomorrow's activity wouldn't be returned by the repository
        // because the date range filter is exclusive of the end date
        let activities = vec![
            // Yesterday - should be included
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    yesterday
                        .and_hms_opt(10, 0, 0)
                        .unwrap()
                        .and_utc()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Running,
                ActivityStatistics::default(),
                None,
                None,
                None,
                None,
            ),
            // Today - should be included (this is the bug we're fixing)
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(
                    today
                        .and_hms_opt(14, 30, 0)
                        .unwrap()
                        .and_utc()
                        .timestamp()
                        .try_into()
                        .unwrap(),
                )
                .unwrap(),
                Sport::Cycling,
                ActivityStatistics::default(),
                None,
                None,
                None,
                None,
            ),
        ];

        // Create an open-ended period (no end date) starting 7 days ago
        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            today - Days::new(7),
            None, // Open-ended
            "Open Period".to_string(),
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

        // Should include yesterday and today's activities
        // (Before the fix, today's activities would be excluded)
        assert_eq!(
            period_with_activities.activities().len(),
            2,
            "Open-ended period should include today's activities"
        );

        // Verify the activities are yesterday and today
        let activity_sports: Vec<_> = period_with_activities
            .activities()
            .iter()
            .map(|a| a.sport())
            .collect();
        assert!(activity_sports.contains(&&Sport::Running)); // yesterday
        assert!(activity_sports.contains(&&Sport::Cycling)); // today
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

    #[tokio::test]
    async fn test_delete_training_period_ok() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
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
        training_repository
            .expect_delete_training_period()
            .times(1)
            .returning(|_| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = DeleteTrainingPeriodRequest::new(user_id, period_id);
        let result = service.delete_training_period(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_training_period_not_found() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| None);

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = DeleteTrainingPeriodRequest::new(user_id.clone(), period_id.clone());
        let result = service.delete_training_period(req).await;

        assert!(result.is_err());
        match result {
            Err(DeleteTrainingPeriodError::PeriodDoesNotExist(id)) => {
                assert_eq!(id, period_id);
            }
            _ => panic!("Expected PeriodDoesNotExist error"),
        }
    }

    #[tokio::test]
    async fn test_delete_training_period_user_does_not_own() {
        let period_id = TrainingPeriodId::new();
        let owner_user_id = UserId::from("owner");
        let other_user_id = UserId::from("other");

        let period = TrainingPeriod::new(
            period_id.clone(),
            owner_user_id.clone(),
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

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = DeleteTrainingPeriodRequest::new(other_user_id.clone(), period_id.clone());
        let result = service.delete_training_period(req).await;

        assert!(result.is_err());
        match result {
            Err(DeleteTrainingPeriodError::UserDoesNotOwnPeriod(user, period)) => {
                assert_eq!(user, other_user_id);
                assert_eq!(period, period_id);
            }
            _ => panic!("Expected UserDoesNotOwnPeriod error"),
        }
    }

    #[tokio::test]
    async fn test_delete_training_period_repository_error() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
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
        training_repository
            .expect_delete_training_period()
            .times(1)
            .returning(|_| Err(anyhow!("database error")));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = DeleteTrainingPeriodRequest::new(user_id, period_id);
        let result = service.delete_training_period(req).await;

        assert!(result.is_err());
        match result {
            Err(DeleteTrainingPeriodError::Unknown(_)) => {}
            _ => panic!("Expected Unknown error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_name_ok() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_name = "Updated Period Name".to_string();

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            Some("2025-10-21".parse::<NaiveDate>().unwrap()),
            "Old Period Name".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(move |_, _| Some(period));

        let period_id_clone = period_id.clone();
        training_repository
            .expect_update_training_period_name()
            .times(1)
            .withf(move |id, name| id == &period_id_clone && name == "Updated Period Name")
            .returning(|_, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodNameRequest::new(user_id, period_id, new_name);
        let result = service.update_training_period_name(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_name_not_found() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_name = "Updated Period Name".to_string();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(|_, _| None);

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodNameRequest::new(user_id, period_id.clone(), new_name);
        let result = service.update_training_period_name(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodNameError::PeriodDoesNotExist(id)) => {
                assert_eq!(id, period_id);
            }
            _ => panic!("Expected PeriodDoesNotExist error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_name_user_does_not_own() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let other_user_id = UserId::from("other_user".to_string());
        let new_name = "Updated Period Name".to_string();

        let period = TrainingPeriod::new(
            period_id.clone(),
            other_user_id.clone(),
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

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req =
            UpdateTrainingPeriodNameRequest::new(user_id.clone(), period_id.clone(), new_name);
        let result = service.update_training_period_name(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodNameError::UserDoesNotOwnPeriod(uid, pid)) => {
                assert_eq!(uid, user_id);
                assert_eq!(pid, period_id);
            }
            _ => panic!("Expected UserDoesNotOwnPeriod error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_name_repository_error() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_name = "Updated Period Name".to_string();

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
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
        training_repository
            .expect_update_training_period_name()
            .times(1)
            .returning(|_, _| Err(anyhow!("database error")));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodNameRequest::new(user_id, period_id, new_name);
        let result = service.update_training_period_name(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodNameError::Unknown(_)) => {}
            _ => panic!("Expected Unknown error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_note_ok() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_note = Some("Updated note content".to_string());

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            Some("2025-10-21".parse::<NaiveDate>().unwrap()),
            "Test Period".to_string(),
            TrainingPeriodSports::new(None),
            Some("Old note".to_string()),
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(move |_, _| Some(period));

        let period_id_clone = period_id.clone();
        let new_note_clone = new_note.clone();
        training_repository
            .expect_update_training_period_note()
            .times(1)
            .withf(move |id, note| id == &period_id_clone && note == &new_note_clone)
            .returning(|_, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodNoteRequest::new(user_id, period_id, new_note);
        let result = service.update_training_period_note(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_note_clear_note() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_note = None;

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            Some("2025-10-21".parse::<NaiveDate>().unwrap()),
            "Test Period".to_string(),
            TrainingPeriodSports::new(None),
            Some("Old note".to_string()),
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(move |_, _| Some(period));

        let period_id_clone = period_id.clone();
        training_repository
            .expect_update_training_period_note()
            .times(1)
            .withf(move |id, note| id == &period_id_clone && note.is_none())
            .returning(|_, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodNoteRequest::new(user_id, period_id, new_note);
        let result = service.update_training_period_note(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_note_not_found() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_note = Some("Updated note".to_string());

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(|_, _| None);

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodNoteRequest::new(user_id, period_id.clone(), new_note);
        let result = service.update_training_period_note(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodNoteError::PeriodDoesNotExist(id)) => {
                assert_eq!(id, period_id);
            }
            _ => panic!("Expected PeriodDoesNotExist error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_note_user_does_not_own() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let other_user_id = UserId::from("other_user".to_string());
        let new_note = Some("Updated note".to_string());

        let period = TrainingPeriod::new(
            period_id.clone(),
            other_user_id.clone(),
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

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req =
            UpdateTrainingPeriodNoteRequest::new(user_id.clone(), period_id.clone(), new_note);
        let result = service.update_training_period_note(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodNoteError::UserDoesNotOwnPeriod(uid, pid)) => {
                assert_eq!(uid, user_id);
                assert_eq!(pid, period_id);
            }
            _ => panic!("Expected UserDoesNotOwnPeriod error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_note_repository_error() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_note = Some("Updated note".to_string());

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
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
        training_repository
            .expect_update_training_period_note()
            .times(1)
            .returning(|_, _| Err(anyhow!("database error")));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodNoteRequest::new(user_id, period_id, new_note);
        let result = service.update_training_period_note(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodNoteError::Unknown(_)) => {}
            _ => panic!("Expected Unknown error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_dates_ok() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_start = "2025-11-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-11-30".parse::<NaiveDate>().unwrap());

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
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

        let period_id_clone = period_id.clone();
        let new_start_clone = new_start;
        let new_end_clone = new_end.clone();
        training_repository
            .expect_update_training_period_dates()
            .times(1)
            .withf(move |id, start, end| {
                id == &period_id_clone && start == &new_start_clone && end == &new_end_clone
            })
            .returning(|_, _, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodDatesRequest::new(user_id, period_id, new_start, new_end);
        let result = service.update_training_period_dates(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_dates_clear_end_date() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_start = "2025-11-01".parse::<NaiveDate>().unwrap();
        let new_end = None;

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
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

        let period_id_clone = period_id.clone();
        let new_start_clone = new_start;
        training_repository
            .expect_update_training_period_dates()
            .times(1)
            .withf(move |id, start, end| {
                id == &period_id_clone && start == &new_start_clone && end.is_none()
            })
            .returning(|_, _, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodDatesRequest::new(user_id, period_id, new_start, new_end);
        let result = service.update_training_period_dates(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_dates_end_before_start() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_start = "2025-11-30".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-11-01".parse::<NaiveDate>().unwrap());

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
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

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodDatesRequest::new(user_id, period_id, new_start, new_end);
        let result = service.update_training_period_dates(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodDatesError::EndDateBeforeStartDate) => {}
            _ => panic!("Expected EndDateBeforeStartDate error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_dates_not_found() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_start = "2025-11-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-11-30".parse::<NaiveDate>().unwrap());

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .return_once(|_, _| None);

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req =
            UpdateTrainingPeriodDatesRequest::new(user_id, period_id.clone(), new_start, new_end);
        let result = service.update_training_period_dates(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodDatesError::PeriodDoesNotExist(id)) => {
                assert_eq!(id, period_id);
            }
            _ => panic!("Expected PeriodDoesNotExist error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_dates_user_does_not_own() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let other_user_id = UserId::from("other_user".to_string());
        let new_start = "2025-11-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-11-30".parse::<NaiveDate>().unwrap());

        let period = TrainingPeriod::new(
            period_id.clone(),
            other_user_id.clone(),
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

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodDatesRequest::new(
            user_id.clone(),
            period_id.clone(),
            new_start,
            new_end,
        );
        let result = service.update_training_period_dates(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodDatesError::UserDoesNotOwnPeriod(uid, pid)) => {
                assert_eq!(uid, user_id);
                assert_eq!(pid, period_id);
            }
            _ => panic!("Expected UserDoesNotOwnPeriod error"),
        }
    }

    #[tokio::test]
    async fn test_update_training_period_dates_repository_error() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_start = "2025-11-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-11-30".parse::<NaiveDate>().unwrap());

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
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
        training_repository
            .expect_update_training_period_dates()
            .times(1)
            .returning(|_, _, _| Err(anyhow!("database error")));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = UpdateTrainingPeriodDatesRequest::new(user_id, period_id, new_start, new_end);
        let result = service.update_training_period_dates(req).await;

        assert!(result.is_err());
        match result {
            Err(UpdateTrainingPeriodDatesError::Unknown(_)) => {}
            _ => panic!("Expected Unknown error"),
        }
    }
}

#[cfg(test)]
mod test_training_service_training_note {
    use super::*;
    use crate::domain::models::training::TrainingNoteContent;
    use crate::domain::ports::{
        CreateTrainingNoteError, CreateTrainingNoteRequest, SaveTrainingNoteError,
    };
    use crate::domain::services::activity::test_utils::MockActivityRepository;
    use crate::domain::services::training::test_utils::MockTrainingRepository;
    use anyhow::anyhow;

    #[tokio::test]
    async fn test_create_training_note_ok() {
        let user_id = UserId::from("user1");
        let title = None;
        let content = TrainingNoteContent::from("This is a test note");
        let date = TrainingNoteDate::today();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_save_training_note()
            .times(1)
            .returning(|_| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = CreateTrainingNoteRequest::new(user_id, title, content, date);
        let result = service.create_training_note(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_training_note_repository_error() {
        let user_id = UserId::from("user1");
        let title = None;
        let content = TrainingNoteContent::from("Note that fails to save");
        let date = TrainingNoteDate::today();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_save_training_note()
            .times(1)
            .returning(|_| Err(SaveTrainingNoteError::Unknown(anyhow!("database error"))));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let req = CreateTrainingNoteRequest::new(user_id, title, content, date);
        let result = service.create_training_note(req).await;

        assert!(result.is_err());
        match result {
            Err(CreateTrainingNoteError::Unknown(_)) => {}
            _ => panic!("Expected Unknown error"),
        }
    }
}

#[cfg(test)]
mod test_training_service_metric_values {
    use chrono::NaiveDate;

    use super::*;
    use crate::domain::models::activity::{
        Activity, ActivityId, ActivityStartTime, ActivityStatistic, ActivityStatistics, Sport,
    };
    use crate::domain::models::training::{
        ActivityMetricSource, TrainingMetricAggregate, TrainingMetricFilters,
        TrainingMetricGranularity,
    };
    use crate::domain::ports::GetTrainingMetricValuesError;
    use crate::domain::services::activity::test_utils::MockActivityRepository;
    use crate::domain::services::training::test_utils::MockTrainingRepository;
    use std::collections::HashMap;

    // #[tokio::test]
    // async fn test_get_training_metric_values_ok() {
    //     let user_id = UserId::from("user1");
    //     let metric_id = TrainingMetricId::new();
    //     let date_range = DateRange::new(
    //         "2024-01-01".parse::<NaiveDate>().unwrap(),
    //         "2024-12-31".parse::<NaiveDate>().unwrap(),
    //     );

    //     let definition = TrainingMetricDefinition::new(
    //         user_id.clone(),
    //         ActivityMetricSource::Statistic(ActivityStatistic::Distance),
    //         TrainingMetricGranularity::Weekly,
    //         TrainingMetricAggregate::Sum,
    //         TrainingMetricFilters::empty(),
    //         None,
    //     );

    //     let values = TrainingMetricValues::new(HashMap::new());

    //     let mut training_repository = MockTrainingRepository::new();
    //     training_repository
    //         .expect_get_definition()
    //         .times(1)
    //         .return_once(move |_| Ok(Some(definition)));
    //     training_repository
    //         .expect_get_metric_values()
    //         .times(1)
    //         .return_once(move |_, _| Ok(values));

    //     let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
    //     let service = TrainingService::new(training_repository, activity_repository);

    //     let result = service
    //         .get_training_metric_values(&user_id, &metric_id, &date_range)
    //         .await;

    //     assert!(result.is_ok());
    // }

    #[tokio::test]
    async fn test_get_training_metric_values_metric_not_found() {
        let user_id = UserId::from("user1");
        let metric_id = TrainingMetricId::new();
        let date_range = DateRange::new(
            "2024-01-01".parse::<NaiveDate>().unwrap(),
            "2024-12-31".parse::<NaiveDate>().unwrap(),
        );

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_definition()
            .times(1)
            .return_once(move |_| Ok(None));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let result = service
            .get_training_metric_values(&user_id, &metric_id, &date_range)
            .await;

        assert!(result.is_err());
        match result {
            Err(GetTrainingMetricValuesError::TrainingMetricDoesNotExists(_)) => {}
            _ => panic!("Expected TrainingMetricDoesNotExists error"),
        }
    }

    #[tokio::test]
    async fn test_get_training_metric_values_wrong_user() {
        let user_id = UserId::from("user1");
        let other_user_id = UserId::from("user2");
        let metric_id = TrainingMetricId::new();
        let date_range = DateRange::new(
            "2024-01-01".parse::<NaiveDate>().unwrap(),
            "2024-12-31".parse::<NaiveDate>().unwrap(),
        );

        let definition = TrainingMetricDefinition::new(
            other_user_id,
            ActivityMetricSource::Statistic(ActivityStatistic::Distance),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_definition()
            .times(1)
            .return_once(move |_| Ok(Some(definition)));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(training_repository, activity_repository);

        let result = service
            .get_training_metric_values(&user_id, &metric_id, &date_range)
            .await;

        assert!(result.is_err());
        match result {
            Err(GetTrainingMetricValuesError::Unknown(_)) => {}
            _ => panic!("Expected Unknown error"),
        }
    }

    #[tokio::test]
    async fn test_compute_training_metric_values_ok() {
        let user_id = UserId::from("user1");
        let date_range = DateRange::new(
            "2024-01-01".parse::<NaiveDate>().unwrap(),
            "2024-01-31".parse::<NaiveDate>().unwrap(),
        );

        let definition = TrainingMetricDefinition::new(
            user_id,
            ActivityMetricSource::Statistic(ActivityStatistic::Distance),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        let mut activity_repository = MockActivityRepository::default();
        activity_repository
            .expect_list_activities()
            .times(1)
            .returning(|_, _| Ok(vec![]));

        let training_repository = MockTrainingRepository::new();
        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(training_repository, activity_repository);

        let result = service
            .compute_training_metric_values(&definition, &date_range)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_compute_training_metric_values_with_activities() {
        let user_id = UserId::from("user1");
        let date_range = DateRange::new(
            "2024-01-01".parse::<NaiveDate>().unwrap(),
            "2024-01-31".parse::<NaiveDate>().unwrap(),
        );

        let definition = TrainingMetricDefinition::new(
            user_id.clone(),
            ActivityMetricSource::Statistic(ActivityStatistic::Distance),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        // Create some test activities
        let mut stats_map = HashMap::new();
        stats_map.insert(ActivityStatistic::Distance, 10000.0);

        let activity = Activity::new(
            ActivityId::new(),
            user_id.clone(),
            None,
            ActivityStartTime::from_timestamp(1705315200).unwrap(), // 2024-01-15T10:00:00Z
            Sport::Running,
            ActivityStatistics::new(stats_map),
            None,
            None,
            None,
            None,
        );

        let mut activity_repository = MockActivityRepository::default();
        activity_repository
            .expect_list_activities()
            .times(1)
            .returning(move |_, _| Ok(vec![activity.clone()]));

        let training_repository = MockTrainingRepository::new();
        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(training_repository, activity_repository);

        let result = service
            .compute_training_metric_values(&definition, &date_range)
            .await;

        assert!(result.is_ok());
        let values = result.unwrap();
        // With weekly granularity and sum aggregate, we should have one bin with the activity distance
        assert!(!values.is_empty());
    }
}
