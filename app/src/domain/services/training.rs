use std::sync::Arc;

use chrono::NaiveDate;
use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::domain::{
    models::{
        UserId,
        training::{
            ComputeMetricRequirement, TrainingMetricBin, TrainingMetricDefinition,
            TrainingMetricId, TrainingMetricValues, TrainingNote, TrainingNoteContent,
            TrainingNoteDate, TrainingNoteId, TrainingNoteTitle, TrainingPeriodId,
        },
    },
    ports::{
        ActivityRepository, CreateTrainingMetricError, CreateTrainingMetricRequest,
        CreateTrainingNoteError, CreateTrainingNoteRequest, CreateTrainingPeriodError,
        CreateTrainingPeriodRequest, DateRange, DeleteTrainingMetricError,
        DeleteTrainingMetricRequest, DeleteTrainingNoteError, DeleteTrainingPeriodError,
        DeleteTrainingPeriodRequest, GetTrainingNoteError, ITrainingService, ListActivitiesFilters,
        RemoveActivityFromMetricsRequest, TrainingRepository, UpdateMetricsValuesRequest,
        UpdateTrainingNoteError, UpdateTrainingPeriodNameError, UpdateTrainingPeriodNameRequest,
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
                .update_metric_value(definition.id(), (key, value))
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
            req.group_by().clone(),
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
                let Some(activity_metric) = definition
                    .source()
                    .metric_from_activity_with_timeseries(activity)
                else {
                    continue;
                };
                let granule = definition
                    .granularity()
                    .datetime_key(activity.start_time().date());
                let group = definition
                    .group_by()
                    .as_ref()
                    .and_then(|group_by| group_by.extract_group(activity.activity()));
                let bin = TrainingMetricBin::new(granule, group);
                let new_metric_value = match self
                    .training_repository
                    .get_metric_value(definition.id(), &bin)
                    .await
                {
                    Ok(Some(previous_metric_value)) => {
                        let Some(new_metric_value) = definition
                            .aggregate()
                            .update_value(&previous_metric_value, &activity_metric)
                        else {
                            continue;
                        };
                        new_metric_value
                    }
                    Ok(None) => {
                        let Some(new_metric_value) =
                            definition.aggregate().initial_value(&activity_metric)
                        else {
                            continue;
                        };
                        new_metric_value
                    }
                    Err(_err) => continue,
                };
                let _ = self
                    .training_repository
                    .update_metric_value(definition.id(), (bin, new_metric_value))
                    .await;
            }
        }

        Ok(())
    }

    async fn remove_activity_from_metrics(
        &self,
        req: RemoveActivityFromMetricsRequest,
    ) -> Result<(), ()> {
        let definitions = self
            .training_repository
            .get_definitions(req.user())
            .await
            .unwrap();

        let activity = req.deleted_activity();

        // Collect bins affected by the deleted activity that need recomputation
        let mut bins_to_recompute: std::collections::HashSet<(
            TrainingMetricId,
            String, // granule key for date range reconstruction
        )> = std::collections::HashSet::new();

        for definition in &definitions {
            // Determine the bin(s) that need recomputation
            let granule = definition
                .granularity()
                .datetime_key(activity.start_time().date());

            bins_to_recompute.insert((definition.id().clone(), granule));
        }

        // Recompute bins affected by deleted activity
        for (metric_id, granule_key) in bins_to_recompute {
            // Find the definition for this metric
            let Some(definition) = definitions.iter().find(|d| d.id() == &metric_id) else {
                continue;
            };

            // Delete existing values for this bin before recomputing
            // This ensures that if the bin becomes empty, old values are removed
            let _ = self
                .training_repository
                .delete_metric_values_for_bin(&metric_id, &granule_key)
                .await;

            // Parse the granule key to get the start date
            let Ok(start_date) = NaiveDate::parse_from_str(&granule_key, "%Y-%m-%d") else {
                continue;
            };

            // Create a date range for this bin
            let bin_range = definition
                .granularity()
                .bins(&DateRange::new(start_date, start_date));
            if let Some(first_bin) = bin_range.first() {
                self.compute_metric_values(definition, first_bin, req.user())
                    .await;
            }
        }

        Ok(())
    }

    async fn get_training_metrics(
        &self,
        user: &UserId,
        date_range: &Option<DateRange>,
    ) -> Vec<(TrainingMetricDefinition, TrainingMetricValues)> {
        let Ok(definitions) = self.training_repository.get_definitions(user).await else {
            return vec![];
        };

        let mut res = vec![];
        for definition in definitions {
            // Align the date range to the metric's granularity to ensure complete bins
            // For example, if granularity is Weekly and date_range starts on Wednesday,
            // we align it to the Monday of that week to include the full week's data
            let aligned_date_range = date_range.as_ref().map(|range| {
                let bins = definition.granularity().bins(range);
                if let (Some(first), Some(last)) = (bins.first(), bins.last()) {
                    DateRange::new(*first.start(), *last.end())
                } else {
                    range.clone()
                }
            });

            let values = self
                .training_repository
                .get_metric_values(definition.id(), &aligned_date_range)
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
        models::training::{
            TrainingMetricValue, TrainingNote, TrainingNoteContent, TrainingPeriod,
            TrainingPeriodWithActivities,
        },
        ports::{
            CreateTrainingPeriodError, CreateTrainingPeriodRequest, DeleteMetricError,
            DeleteTrainingNoteError, DeleteTrainingPeriodError, DeleteTrainingPeriodRequest,
            GetDefinitionError, GetTrainingMetricValueError, GetTrainingMetricsDefinitionsError,
            GetTrainingNoteError, SaveTrainingMetricError, SaveTrainingNoteError,
            SaveTrainingPeriodError, UpdateMetricError, UpdateTrainingNoteError,
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

            async fn remove_activity_from_metrics(
                &self,
                req: RemoveActivityFromMetricsRequest,
            ) -> Result<(), ()>;

            async fn get_training_metrics(
                &self,
                user: &UserId,
                date_range: &Option<DateRange>,
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
        }
    }

    impl MockTrainingService {
        pub fn test_default() -> Self {
            let mut mock = Self::new();

            mock.expect_create_metric()
                .returning(|_| Ok(TrainingMetricId::default()));
            mock.expect_update_metrics_values().returning(|_| Ok(()));
            mock.expect_get_training_metrics().returning(|_, _| vec![]);
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

            async fn update_metric_value(
                &self,
                id: &TrainingMetricId,
                values: (TrainingMetricBin, TrainingMetricValue),
            ) -> Result<(), UpdateMetricError>;

            async fn delete_metric_values_for_bin(
                &self,
                id: &TrainingMetricId,
                granule: &str,
            ) -> Result<(), anyhow::Error>;

            async fn get_metric_value(
                &self,
                id: &TrainingMetricId,
                bin: &TrainingMetricBin
            ) -> Result<Option<TrainingMetricValue>, GetTrainingMetricValueError>;

            async fn get_metric_values(
                &self,
                id: &TrainingMetricId,
                date_range: &Option<DateRange>,
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
    use std::{collections::HashMap, sync::Arc};

    use anyhow::anyhow;
    use chrono::{DateTime, Days, FixedOffset, NaiveDate, Utc};
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
                TrainingMetricFilters, TrainingMetricGranularity, TrainingMetricGroupBy,
                TrainingMetricId, TrainingMetricValue, TrainingMetricValues,
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
            TrainingMetricGroupBy::none(),
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
            .expect_update_metric_value()
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
                        None,
                        None,
                        None,
                        None,
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
            TrainingMetricGroupBy::none(),
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
            .expect_update_metric_value()
            .times(2) // 2 day fron initial range
            .returning(|_, _| Ok(()));
        background_repository
            .expect_update_metric_value()
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
                None,
                None,
                None,
                None,
            )])
        });
        let service = TrainingService::new(repository, Arc::new(Mutex::new(activities)));

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty(),
            TrainingMetricGroupBy::none(),
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
        repository.expect_update_metric_value().times(0);
        let mut activities = MockActivityRepository::new();
        activities.expect_list_activities_with_timeseries().times(0);
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
        repository.expect_get_definitions().returning(|_| {
            Err(GetTrainingMetricsDefinitionsError::Unknown(anyhow!(
                "an error"
            )))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let res = service
            .get_training_metrics(&UserId::test_default(), &None)
            .await;
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
                TrainingMetricGroupBy::none(),
            )])
        });
        repository
            .expect_get_metric_values()
            .returning(|_, _| Ok(TrainingMetricValues::new(HashMap::new())));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let res = service
            .get_training_metrics(&UserId::test_default(), &None)
            .await;

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
                TrainingMetricGroupBy::none(),
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
                TrainingMetricGroupBy::none(),
            )])
        });
        repository.expect_get_metric_values().returning(|_, _| {
            Ok(TrainingMetricValues::new(HashMap::from([(
                TrainingMetricBin::from_granule("toto"),
                TrainingMetricValue::Max(0.3),
            )])))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let res = service
            .get_training_metrics(&UserId::test_default(), &None)
            .await;

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
                TrainingMetricGroupBy::none(),
            )
        );
        assert_eq!(
            *value.get(&TrainingMetricBin::from_granule("toto")).unwrap(),
            TrainingMetricValue::Max(0.3)
        );
    }

    #[tokio::test]
    async fn test_training_service_get_metrics_with_date_range() {
        use crate::domain::ports::DateRange;
        use chrono::NaiveDate;

        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
            )])
        });

        // Expect the repository method to be called with the date range
        repository
            .expect_get_metric_values()
            .withf(|_, date_range| {
                // Verify that the date range is passed through
                date_range.is_some()
            })
            .returning(|_, _| {
                Ok(TrainingMetricValues::new(HashMap::from([(
                    TrainingMetricBin::from_granule("2025-09-24"),
                    TrainingMetricValue::Max(100.0),
                )])))
            });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let date_range = Some(DateRange::new(
            NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
            NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
        ));

        let res = service
            .get_training_metrics(&UserId::test_default(), &date_range)
            .await;

        assert_eq!(res.len(), 1);
        let (_, values) = res.first().unwrap();
        assert_eq!(values.clone().as_hash_map().len(), 1);
    }

    #[tokio::test]
    async fn test_training_service_get_metrics_aligns_date_range_to_granularity() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Weekly, // Weekly granularity
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
            )])
        });

        // The date range will be aligned to week boundaries (Monday)
        // Input: Wed Sep 24 to Thu Sep 25, 2025
        // Should be aligned to: Mon Sep 22 to Mon Sep 29, 2025
        repository
            .expect_get_metric_values()
            .withf(|_, date_range| {
                let Some(range) = date_range else {
                    return false;
                };
                // Verify that the range starts on Monday (Sep 22)
                *range.start() == NaiveDate::from_ymd_opt(2025, 9, 22).unwrap()
                    && *range.end() == NaiveDate::from_ymd_opt(2025, 9, 29).unwrap()
            })
            .returning(|_, _| Ok(TrainingMetricValues::default()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        // Input date range: Wednesday to Thursday (mid-week)
        let date_range = Some(DateRange::new(
            NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(), // Wednesday
            NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(), // Thursday
        ));

        let res = service
            .get_training_metrics(&UserId::test_default(), &date_range)
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
                TrainingMetricId::from("test"),
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
                TrainingMetricId::from("test"),
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

    // Helper function to create a test activity with given timestamp and statistics
    fn create_test_activity(
        timestamp: usize,
        calories: Option<f64>,
        distance: Option<f64>,
    ) -> ActivityWithTimeseries {
        create_test_activity_with_sport(timestamp, calories, distance, Sport::Running)
    }

    // Helper function to create a test activity with given timestamp, statistics, and sport
    fn create_test_activity_with_sport(
        timestamp: usize,
        calories: Option<f64>,
        distance: Option<f64>,
        sport: Sport,
    ) -> ActivityWithTimeseries {
        let mut stats_map = HashMap::new();
        if let Some(cal) = calories {
            stats_map.insert(ActivityStatistic::Calories, cal);
        }
        if let Some(dist) = distance {
            stats_map.insert(ActivityStatistic::Distance, dist);
        }

        ActivityWithTimeseries::new(
            Activity::new(
                ActivityId::new(),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(timestamp).unwrap(),
                sport,
                ActivityStatistics::new(stats_map),
                None,
                None,
                None,
                None,
            ),
            ActivityTimeseries::default(),
        )
    }

    #[tokio::test]
    async fn test_update_metrics_values_with_no_definitions() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(|_| Ok(vec![]));
        repository.expect_update_metric_value().times(0);

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));
        let req = UpdateMetricsValuesRequest::new(UserId::test_default(), vec![activity]);

        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_with_activity_not_matching_metric_source() {
        let metric_id = TrainingMetricId::from("test");
        let user_id = UserId::test_default();
        let user_id_clone = user_id.clone();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                )])
            });
        repository.expect_update_metric_value().times(0);

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        // Activity has no distance, so does not match the metric definition
        let activity = create_test_activity(1200, Some(1000.0), None);
        let req = UpdateMetricsValuesRequest::new(user_id, vec![activity]);

        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_no_previous_value() {
        let metric_id = TrainingMetricId::from("test");
        let user_id = UserId::test_default();
        let user_id_clone = user_id.clone();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                )])
            });
        repository
            .expect_get_metric_value()
            .times(1)
            .returning(|_, _| Ok(None)); // No previous value
        repository
            .expect_update_metric_value()
            .times(1)
            .withf(|_, (_, value)| {
                // For Sum aggregate, initial value should be the metric value itself (1000 calories)
                matches!(value, TrainingMetricValue::Sum(v) if (*v - 1000.0).abs() < 0.01)
            })
            .returning(|_, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));
        let req = UpdateMetricsValuesRequest::new(user_id, vec![activity]);

        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_updates_existing_value() {
        let metric_id = TrainingMetricId::from("test");
        let user_id = UserId::test_default();
        let user_id_clone = user_id.clone();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                )])
            });
        repository
            .expect_get_metric_value()
            .times(1)
            .returning(|_, _| Ok(Some(TrainingMetricValue::Sum(500.0)))); // Previous value with 500 calories
        repository
            .expect_update_metric_value()
            .times(1)
            .withf(|_, (_, value)| {
                // For Sum aggregate, updated value should be 1500 calories (500 + 1000)
                matches!(value, TrainingMetricValue::Sum(v) if (*v - 1500.0).abs() < 0.01)
            })
            .returning(|_, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));
        let req = UpdateMetricsValuesRequest::new(user_id, vec![activity]);

        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_with_multiple_activities() {
        let metric_id = TrainingMetricId::from("test");
        let user_id = UserId::test_default();
        let user_id_clone = user_id.clone();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                )])
            });
        repository
            .expect_get_metric_value()
            .times(2)
            .returning(|_, _| Ok(None));
        repository
            .expect_update_metric_value()
            .times(2)
            .returning(|_, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let activity1 = create_test_activity(1200, Some(1000.0), Some(5000.0));
        let activity2 = create_test_activity(3000, Some(1000.0), Some(10000.0));
        let req = UpdateMetricsValuesRequest::new(user_id, vec![activity1, activity2]);

        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_with_multiple_definitions() {
        let metric_id1 = TrainingMetricId::from("test1");
        let metric_id2 = TrainingMetricId::from("test2");
        let user_id = UserId::test_default();
        let user_id_clone = user_id.clone();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![
                    TrainingMetricDefinition::new(
                        metric_id1.clone(),
                        user_id_clone.clone(),
                        ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                        TrainingMetricGranularity::Daily,
                        TrainingMetricAggregate::Sum,
                        TrainingMetricFilters::empty(),
                        TrainingMetricGroupBy::none(),
                    ),
                    TrainingMetricDefinition::new(
                        metric_id2.clone(),
                        user_id_clone.clone(),
                        ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                        TrainingMetricGranularity::Daily,
                        TrainingMetricAggregate::Sum,
                        TrainingMetricFilters::empty(),
                        TrainingMetricGroupBy::none(),
                    ),
                ])
            });
        repository
            .expect_get_metric_value()
            .times(2)
            .returning(|_, _| Ok(None));
        repository
            .expect_update_metric_value()
            .times(2)
            .returning(|_, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let activity = create_test_activity(1200, Some(1000.0), Some(5000.0));
        let req = UpdateMetricsValuesRequest::new(user_id, vec![activity]);

        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_continues_on_get_metric_value_error() {
        let metric_id = TrainingMetricId::from("test");
        let user_id = UserId::test_default();
        let user_id_clone = user_id.clone();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                )])
            });
        repository
            .expect_get_metric_value()
            .times(1)
            .returning(|_, _| Err(anyhow!("database error").into()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));
        let req = UpdateMetricsValuesRequest::new(user_id, vec![activity]);

        // Should return Ok despite the error, as it continues processing
        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_continues_on_update_error() {
        let metric_id = TrainingMetricId::from("test");
        let user_id = UserId::test_default();
        let user_id_clone = user_id.clone();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                )])
            });
        repository
            .expect_get_metric_value()
            .times(1)
            .returning(|_, _| Ok(None));
        repository
            .expect_update_metric_value()
            .times(1)
            .returning(|_, _| Err(anyhow!("database error").into()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));
        let req = UpdateMetricsValuesRequest::new(user_id, vec![activity]);

        // Should return Ok despite the error, as it continues processing
        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_with_average_aggregate() {
        let metric_id = TrainingMetricId::from("test");
        let user_id = UserId::test_default();
        let user_id_clone = user_id.clone();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Average,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                )])
            });
        repository
            .expect_get_metric_value()
            .times(1)
            .returning(|_, _| {
                Ok(Some(TrainingMetricValue::Average {
                    value: 500.0,
                    sum: 1000.0,
                    number_of_elements: 2,
                }))
            }); // Average of 2 activities = 500
        repository
            .expect_update_metric_value()
            .times(1)
            .withf(|_, (_, value)| {
                // For Average aggregate with 2 activities at 500 avg (sum 1000) and new activity with 1000 calories
                // New average: (1000 + 1000) / 3 = 2000 / 3 ≈ 666.67
                matches!(
                    value,
                    TrainingMetricValue::Average { value: v, sum: s, number_of_elements: n }
                    if (*v - 666.67).abs() < 0.01 && (*s - 2000.0).abs() < 0.01 && *n == 3
                )
            })
            .returning(|_, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));
        let req = UpdateMetricsValuesRequest::new(user_id, vec![activity]);

        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_with_groupby_sport() {
        // Arrange
        let mut repository = MockTrainingRepository::new();
        let user_id = UserId::test_default();
        let metric_id = TrainingMetricId::new();

        let running_activity =
            create_test_activity_with_sport(1200, Some(500.0), Some(5000.0), Sport::Running);
        let cycling_activity =
            create_test_activity_with_sport(1300, Some(800.0), Some(15000.0), Sport::Cycling);

        let metric_id_clone = metric_id.clone();
        let user_id_clone = user_id.clone();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id_clone.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    Some(TrainingMetricGroupBy::Sport),
                )])
            });

        let metric_id_clone = metric_id.clone();
        repository
            .expect_update_metric_value()
            .withf(move |m, (b, v)| {
                m == &metric_id_clone
                    && b.granule() == "1969-12-29"
                    && b.group() == &Some("Running".to_string())
                    && matches!(v, TrainingMetricValue::Sum(val) if (val - 500.0).abs() < 0.01)
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let metric_id_clone = metric_id.clone();
        repository
            .expect_update_metric_value()
            .withf(move |m, (b, v)| {
                m == &metric_id_clone
                    && b.granule() == "1969-12-29"
                    && b.group() == &Some("Cycling".to_string())
                    && matches!(v, TrainingMetricValue::Sum(val) if (val - 800.0).abs() < 0.01)
            })
            .times(1)
            .returning(|_, _| Ok(()));

        // Mock get_metric_value to return None (no previous value)
        repository
            .expect_get_metric_value()
            .returning(|_, _| Ok(None));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let req =
            UpdateMetricsValuesRequest::new(user_id, vec![running_activity, cycling_activity]);

        // Act & Assert
        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_without_groupby_uses_none_group() {
        // Arrange
        let mut repository = MockTrainingRepository::new();
        let user_id = UserId::test_default();
        let metric_id = TrainingMetricId::new();

        let activity = create_test_activity(1200, Some(500.0), Some(5000.0));

        let metric_id_clone = metric_id.clone();
        let user_id_clone = user_id.clone();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id_clone.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    None, // No group_by
                )])
            });

        repository
            .expect_update_metric_value()
            .withf(move |m, (b, v)| {
                m == &metric_id
                    && b.granule() == "1969-12-29"
                    && b.group() == &None // Verify group is None
                    && matches!(v, TrainingMetricValue::Sum(val) if (val - 500.0).abs() < 0.01)
            })
            .times(1)
            .returning(|_, _| Ok(()));

        // Mock get_metric_value to return None (no previous value)
        repository
            .expect_get_metric_value()
            .returning(|_, _| Ok(None));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let req = UpdateMetricsValuesRequest::new(user_id, vec![activity]);

        // Act & Assert
        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_metrics_values_grouped_same_granule_different_groups() {
        // Arrange
        let mut repository = MockTrainingRepository::new();
        let user_id = UserId::test_default();
        let metric_id = TrainingMetricId::new();

        // Create multiple activities on the same day but different sports
        let running_am =
            create_test_activity_with_sport(1200, Some(400.0), Some(5000.0), Sport::Running);
        let cycling_pm =
            create_test_activity_with_sport(1205, Some(600.0), Some(20000.0), Sport::Cycling);

        let metric_id_clone = metric_id.clone();
        let user_id_clone = user_id.clone();
        repository
            .expect_get_definitions()
            .times(1)
            .returning(move |_| {
                Ok(vec![TrainingMetricDefinition::new(
                    metric_id_clone.clone(),
                    user_id_clone.clone(),
                    ActivityMetricSource::Statistic(ActivityStatistic::Calories),
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    Some(TrainingMetricGroupBy::Sport),
                )])
            });

        let metric_id_clone = metric_id.clone();
        repository
            .expect_update_metric_value()
            .withf(move |m, (b, v)| {
                m == &metric_id_clone
                    && b.granule() == "1970-01-01"
                    && b.group() == &Some("Running".to_string())
                    && matches!(v, TrainingMetricValue::Sum(val) if (val - 400.0).abs() < 0.01)
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let metric_id_clone = metric_id.clone();
        repository
            .expect_update_metric_value()
            .withf(move |m, (b, v)| {
                m == &metric_id_clone
                    && b.granule() == "1970-01-01"
                    && b.group() == &Some("Cycling".to_string())
                    && matches!(v, TrainingMetricValue::Sum(val) if (val - 600.0).abs() < 0.01)
            })
            .times(1)
            .returning(|_, _| Ok(()));

        // Mock get_metric_value to return None (no previous value)
        repository
            .expect_get_metric_value()
            .returning(|_, _| Ok(None));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let req = UpdateMetricsValuesRequest::new(user_id, vec![running_am, cycling_pm]);

        // Act & Assert - verify both groups are updated for the same day
        let result = service.update_metrics_values(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_activity_from_metrics_recomputes_affected_bin() {
        let user_id = UserId::test_default();
        let metric_id = TrainingMetricId::from("test");

        // Create the activity to be removed
        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));

        // Create the definition
        let definition = TrainingMetricDefinition::new(
            metric_id.clone(),
            user_id.clone(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        let mut repository = MockTrainingRepository::new();

        // Mock get_definitions - return our metric definition
        repository
            .expect_get_definitions()
            .returning(move |_| Ok(vec![definition.clone()]));

        // Mock delete_metric_values_for_bin - should be called to clear old values
        repository
            .expect_delete_metric_values_for_bin()
            .times(1)
            .returning(|_, _| Ok(()));

        // Mock list_activities for recomputation
        let mut activity_repository = MockActivityRepository::default();
        activity_repository
            .expect_list_activities()
            .returning(|_, _| Ok(vec![])); // No activities left after deletion

        // Note: update_metric_value won't be called since there are no activities to compute
        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(repository, activity_repository);

        let req = RemoveActivityFromMetricsRequest::new(user_id, activity.activity().clone());

        let result = service.remove_activity_from_metrics(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_activity_from_metrics_handles_multiple_metrics() {
        let user_id = UserId::test_default();
        let metric_id_1 = TrainingMetricId::from("metric1");
        let metric_id_2 = TrainingMetricId::from("metric2");

        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));

        // Create two metric definitions
        let definition_1 = TrainingMetricDefinition::new(
            metric_id_1.clone(),
            user_id.clone(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        let definition_2 = TrainingMetricDefinition::new(
            metric_id_2.clone(),
            user_id.clone(),
            ActivityMetricSource::Statistic(ActivityStatistic::Distance),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        let mut repository = MockTrainingRepository::new();

        repository
            .expect_get_definitions()
            .returning(move |_| Ok(vec![definition_1.clone(), definition_2.clone()]));

        // Mock delete_metric_values_for_bin - should be called twice (once per metric)
        repository
            .expect_delete_metric_values_for_bin()
            .times(2)
            .returning(|_, _| Ok(()));

        let mut activity_repository = MockActivityRepository::default();
        activity_repository
            .expect_list_activities()
            .returning(|_, _| Ok(vec![]));

        // Note: update_metric_value won't be called since there are no activities to compute
        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(repository, activity_repository);

        let req = RemoveActivityFromMetricsRequest::new(user_id, activity.activity().clone());

        let result = service.remove_activity_from_metrics(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_activity_from_metrics_with_weekly_granularity() {
        let user_id = UserId::test_default();
        let metric_id = TrainingMetricId::from("test");

        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));

        // Create a weekly granularity metric
        let definition = TrainingMetricDefinition::new(
            metric_id.clone(),
            user_id.clone(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        let mut repository = MockTrainingRepository::new();

        repository
            .expect_get_definitions()
            .returning(move |_| Ok(vec![definition.clone()]));

        // Mock delete_metric_values_for_bin
        repository
            .expect_delete_metric_values_for_bin()
            .times(1)
            .returning(|_, _| Ok(()));

        let mut activity_repository = MockActivityRepository::default();
        activity_repository
            .expect_list_activities()
            .returning(|_, _| Ok(vec![]));

        // Note: update_metric_value won't be called since there are no activities to compute
        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(repository, activity_repository);

        let req = RemoveActivityFromMetricsRequest::new(user_id, activity.activity().clone());

        let result = service.remove_activity_from_metrics(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_activity_from_metrics_with_no_metrics() {
        let user_id = UserId::test_default();
        let activity = create_test_activity(1200, Some(1000.0), Some(42195.0));

        let mut repository = MockTrainingRepository::new();

        // No metric definitions
        repository
            .expect_get_definitions()
            .returning(|_| Ok(vec![]));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingService::new(repository, activity_repository);

        let req = RemoveActivityFromMetricsRequest::new(user_id, activity.activity().clone());

        let result = service.remove_activity_from_metrics(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_activity_from_metrics_recomputes_with_remaining_activities() {
        let user_id = UserId::test_default();
        let metric_id = TrainingMetricId::from("test");

        let deleted_activity = create_test_activity(1200, Some(1000.0), Some(42195.0));
        let remaining_activity = create_test_activity(600, Some(600.0), Some(5000.0));

        let definition = TrainingMetricDefinition::new(
            metric_id.clone(),
            user_id.clone(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        let mut repository = MockTrainingRepository::new();

        repository
            .expect_get_definitions()
            .returning(move |_| Ok(vec![definition.clone()]));

        // Mock delete_metric_values_for_bin
        repository
            .expect_delete_metric_values_for_bin()
            .times(1)
            .returning(|_, _| Ok(()));

        let mut activity_repository = MockActivityRepository::default();
        // Return the remaining activity when recomputing
        activity_repository
            .expect_list_activities()
            .returning(move |_, _| Ok(vec![remaining_activity.activity().clone()]));

        // Expect the recomputed value to be based only on remaining activity (600.0 calories)
        repository
            .expect_update_metric_value()
            .withf(|_, (_, v)| matches!(v, TrainingMetricValue::Sum(val) if (*val - 600.0).abs() < 0.01))
            .times(1)
            .returning(|_, _| Ok(()));

        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(repository, activity_repository);

        let req =
            RemoveActivityFromMetricsRequest::new(user_id, deleted_activity.activity().clone());

        let result = service.remove_activity_from_metrics(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_activity_from_metrics_clears_bin_when_last_activity_deleted() {
        let user_id = UserId::test_default();
        let metric_id = TrainingMetricId::from("test");

        // The activity to be deleted - it's the only one in its bin
        let deleted_activity = create_test_activity(1200, Some(1000.0), Some(42195.0));

        let definition = TrainingMetricDefinition::new(
            metric_id.clone(),
            user_id.clone(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        let mut repository = MockTrainingRepository::new();

        repository
            .expect_get_definitions()
            .returning(move |_| Ok(vec![definition.clone()]));

        // CRITICAL: Expect delete_metric_values_for_bin to be called before recomputing
        repository
            .expect_delete_metric_values_for_bin()
            .withf(|id, granule| *id == TrainingMetricId::from("test") && granule == "1970-01-01")
            .times(1)
            .returning(|_, _| Ok(()));

        let mut activity_repository = MockActivityRepository::default();
        // Return empty list - no activities left in this bin
        activity_repository
            .expect_list_activities()
            .returning(|_, _| Ok(vec![]));

        // update_metric_value should NOT be called since there are no activities
        repository.expect_update_metric_value().times(0);

        let activity_repository = Arc::new(Mutex::new(activity_repository));
        let service = TrainingService::new(repository, activity_repository);

        let req =
            RemoveActivityFromMetricsRequest::new(user_id, deleted_activity.activity().clone());

        let result = service.remove_activity_from_metrics(req).await;
        assert!(result.is_ok());
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
