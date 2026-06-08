use anyhow::anyhow;
use chrono::NaiveDate;
use derive_more::Constructor;

use crate::domain::{
    models::{
        UserId,
        activity::ActivityMetricV2,
        training::{
            TrainingMetric, TrainingMetricDefinition, TrainingMetricId, TrainingMetricScope,
            TrainingMetricValues, TrainingMetricsOrdering, TrainingNote, TrainingNoteContent,
            TrainingNoteDate, TrainingNoteId, TrainingNoteTitle, TrainingPeriodId,
        },
    },
    ports::{
        DateRange,
        activity::{IActivityService, ListActivitiesFilters},
        training::{
            ComputeTrainingMetricValuesError, CopyTrainingMetricError, CopyTrainingMetricRequest,
            CreateTrainingMetricError, CreateTrainingMetricRequest, CreateTrainingNoteError,
            CreateTrainingNoteRequest, CreateTrainingPeriodError, CreateTrainingPeriodRequest,
            DeleteTrainingMetricError, DeleteTrainingMetricRequest, DeleteTrainingNoteError,
            DeleteTrainingPeriodError, DeleteTrainingPeriodRequest, GetTrainingMetricValuesError,
            GetTrainingMetricValuesRequest, GetTrainingMetricsOrderingError, GetTrainingNoteError,
            ITrainingService, SetTrainingMetricsOrderingError, TrainingRepository,
            UpdateTrainingMetricNameError, UpdateTrainingMetricNameRequest,
            UpdateTrainingNoteError, UpdateTrainingPeriodDatesError,
            UpdateTrainingPeriodDatesRequest, UpdateTrainingPeriodNameError,
            UpdateTrainingPeriodNameRequest, UpdateTrainingPeriodNoteError,
            UpdateTrainingPeriodNoteRequest,
        },
    },
};

///////////////////////////////////////////////////////////////////
/// TRAINING SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Constructor)]
pub struct TrainingService<TR, AS>
where
    TR: TrainingRepository,
    AS: IActivityService,
{
    training_repository: TR,
    activity_service: AS,
}

impl<TR, AS> TrainingService<TR, AS>
where
    TR: TrainingRepository,
    AS: IActivityService,
{
    /// Compute training metric values from a [TrainingMetricDefinition] and a [DateRange] using
    /// activities within the [DateRange].
    async fn compute_training_metric_values(
        &self,
        definition: &TrainingMetricDefinition,
        date_range: &DateRange,
    ) -> Result<TrainingMetricValues, ComputeTrainingMetricValuesError> {
        let activities = self
            .activity_service
            .list_activities_with_metrics(
                definition.user(),
                &ListActivitiesFilters::empty().set_date_range(Some(date_range.clone())),
                &[*definition.metric()],
            )
            .await
            .map_err(|err| anyhow!(err))?
            .into_iter()
            .filter_map(|(activity, values)| {
                if let Some(Some(value)) = values.get(definition.metric()).cloned() {
                    Some((activity, value))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(definition.compute_values(&activities))
    }
}

impl<TMR, AS> ITrainingService for TrainingService<TMR, AS>
where
    TMR: TrainingRepository,
    AS: IActivityService,
{
    async fn create_metric(
        &self,
        req: CreateTrainingMetricRequest,
    ) -> Result<TrainingMetricId, CreateTrainingMetricError> {
        let extra_sports = match req.scope() {
            TrainingMetricScope::Global => None,
            TrainingMetricScope::TrainingPeriod(period) => self
                .training_repository
                .get_training_period(req.user(), period)
                .await
                .ok_or(CreateTrainingMetricError::TrainingPeriodDoesNotExist(
                    period.clone(),
                ))?
                .sports()
                .items()
                .cloned(),
        };

        let id = TrainingMetricId::new();
        let definition = TrainingMetricDefinition::new(
            req.user().clone(),
            *req.metric(),
            req.granularity().clone(),
            req.aggregate().clone(),
            req.filters().clone().merge_sports(&extra_sports),
            req.group_by().clone(),
        );
        let training_metric = TrainingMetric::new(
            id.clone(),
            Some(req.name().clone()),
            req.scope().clone(),
            definition,
        );
        self.training_repository
            .save_training_metric_definition(training_metric.clone())
            .await?;

        Ok(id)
    }

    async fn copy_training_metric(
        &self,
        req: CopyTrainingMetricRequest,
    ) -> Result<(), CopyTrainingMetricError> {
        let source_definition = self
            .training_repository
            .get_definition(req.user(), req.source_metric())
            .await
            .map_err(|err| CopyTrainingMetricError::Unknown(anyhow!(err)))?
            .ok_or_else(|| {
                CopyTrainingMetricError::MetricDoesNotExist(req.source_metric().clone())
            })?;

        let _target_period = self
            .training_repository
            .get_training_period(req.user(), req.target_period())
            .await
            .ok_or_else(|| {
                CopyTrainingMetricError::PeriodDoesNotExist(req.source_metric().clone())
            })?;

        let new_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            None,
            TrainingMetricScope::TrainingPeriod(req.target_period().clone()),
            source_definition,
        );

        self.training_repository
            .save_training_metric_definition(new_metric)
            .await?;

        Ok(())
    }

    async fn get_training_metrics_values(
        &self,
        user: &UserId,
        date_range: &DateRange,
        scope: &TrainingMetricScope,
    ) -> Result<Vec<(TrainingMetric, TrainingMetricValues)>, GetTrainingMetricValuesError> {
        let extra_sports = match scope {
            TrainingMetricScope::Global => None,
            TrainingMetricScope::TrainingPeriod(period) => self
                .training_repository
                .get_training_period(user, period)
                .await
                .ok_or(GetTrainingMetricValuesError::TrainingPeriodDoesNotExist(
                    period.clone(),
                ))?
                .sports()
                .items()
                .cloned(),
        };

        let metrics = match scope {
            TrainingMetricScope::Global => self.training_repository.get_global_metrics(user).await,
            TrainingMetricScope::TrainingPeriod(period) => {
                self.training_repository
                    .get_period_metrics(user, period)
                    .await
            }
        }
        .map_err(|err| GetTrainingMetricValuesError::Unknown(anyhow!(err)))?;

        let ordering = self
            .training_repository
            .get_training_metrics_ordering(user, scope)
            .await
            .unwrap_or_default();

        let metrics = ordering.sort(metrics);

        let mut res = vec![];
        for metric in metrics {
            let definition = metric.definition().clone().merge_sports(&extra_sports);
            let aligned_date_range = date_range.align_to(metric.definition().granularity());

            let values = self
                .compute_training_metric_values(&definition, &aligned_date_range)
                .await
                .unwrap_or_default();

            res.push((metric.clone(), values))
        }

        Ok(res)
    }

    async fn get_training_metric_values(
        &self,
        req: GetTrainingMetricValuesRequest,
        date_range: &DateRange,
    ) -> Result<TrainingMetricValues, GetTrainingMetricValuesError> {
        let definition = match req {
            GetTrainingMetricValuesRequest::ByDefinition {
                user,
                metric,
                granularity,
                aggregate,
                filters,
                group_by,
            } => TrainingMetricDefinition::new(
                user,
                metric,
                granularity,
                aggregate,
                filters,
                group_by,
            ),
            GetTrainingMetricValuesRequest::ByTrainingMetricId(user, id) => self
                .training_repository
                .get_definition(&user, &id)
                .await
                .map_err(|err| GetTrainingMetricValuesError::Unknown(err.into()))?
                .ok_or(GetTrainingMetricValuesError::TrainingMetricDoesNotExist(id))?,
        };

        self.compute_training_metric_values(&definition, date_range)
            .await
            .map_err(GetTrainingMetricValuesError::from)
    }

    async fn delete_metric(
        &self,
        req: DeleteTrainingMetricRequest,
    ) -> Result<(), DeleteTrainingMetricError> {
        self.training_repository
            .delete_definition(req.user(), req.metric())
            .await
    }

    async fn update_training_metric_name(
        &self,
        req: UpdateTrainingMetricNameRequest,
    ) -> Result<(), UpdateTrainingMetricNameError> {
        self.training_repository
            .update_training_metric_name(req.user(), req.metric_id(), req.name().clone())
            .await
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

    async fn get_active_training_periods(
        &self,
        user: &UserId,
        ref_date: &NaiveDate,
    ) -> Vec<crate::domain::models::training::TrainingPeriod> {
        self.training_repository
            .get_active_training_periods(user, ref_date)
            .await
    }

    async fn get_training_period_with_activities_with_metrics(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
        metrics: &[ActivityMetricV2],
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
            .activity_service
            .list_activities_with_metrics(user, &filters, metrics)
            .await
        else {
            return None;
        };

        // Filter activities by the period's sport filters
        let matching_activities: Vec<_> = activities
            .into_iter()
            .filter(|(activity, _)| period.matches(activity))
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
        self.training_repository
            .delete_training_period(req.user(), req.period_id())
            .await
    }

    async fn update_training_period_name(
        &self,
        req: UpdateTrainingPeriodNameRequest,
    ) -> Result<(), UpdateTrainingPeriodNameError> {
        self.training_repository
            .update_training_period_name(req.user(), req.period_id(), req.name().to_string())
            .await
    }

    async fn update_training_period_note(
        &self,
        req: UpdateTrainingPeriodNoteRequest,
    ) -> Result<(), UpdateTrainingPeriodNoteError> {
        self.training_repository
            .update_training_period_note(req.user(), req.period_id(), req.note().clone())
            .await
    }

    async fn update_training_period_dates(
        &self,
        req: UpdateTrainingPeriodDatesRequest,
    ) -> Result<(), UpdateTrainingPeriodDatesError> {
        // Validate dates
        if let Some(end_date) = req.end()
            && req.start() > end_date
        {
            return Err(UpdateTrainingPeriodDatesError::EndDateBeforeStartDate);
        }

        // Update the period dates
        self.training_repository
            .update_training_period_dates(req.user(), req.period_id(), *req.start(), *req.end())
            .await
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
        let note = self
            .training_repository
            .get_training_note(user, note_id)
            .await?;

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
        date_range: &Option<DateRange>,
    ) -> Result<Vec<TrainingNote>, GetTrainingNoteError> {
        self.training_repository
            .get_training_notes(user, date_range)
            .await
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
            .get_training_note(user, note_id)
            .await
            .map_err(|err| UpdateTrainingNoteError::Unknown(err.into()))?;

        match note {
            Some(n) if n.user() == user => {
                self.training_repository
                    .update_training_note(user, note_id, title, content, date)
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
            .get_training_note(user, note_id)
            .await
            .map_err(|err| DeleteTrainingNoteError::Unknown(err.into()))?;

        match note {
            Some(n) if n.user() == user => {
                self.training_repository
                    .delete_training_note(user, note_id)
                    .await?;
                Ok(())
            }
            _ => Err(DeleteTrainingNoteError::Unknown(anyhow::anyhow!(
                "Training note not found or unauthorized"
            ))),
        }
    }

    async fn get_training_period_notes(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
    ) -> Result<Vec<TrainingNote>, GetTrainingNoteError> {
        // Get the training period to verify it exists and belongs to the user
        let Some(period) = self
            .training_repository
            .get_training_period(user, period_id)
            .await
        else {
            return Err(GetTrainingNoteError::Unknown(anyhow::anyhow!(
                "Training period not found"
            )));
        };

        // Get the date range from the period (use range_default_tomorrow to include today)
        let date_range = period.range_default_tomorrow();

        // Fetch notes using the existing method
        self.get_training_notes(user, &Some(date_range)).await
    }

    async fn get_training_period_metrics_values(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
    ) -> Result<Vec<(TrainingMetric, TrainingMetricValues)>, GetTrainingMetricValuesError> {
        // Get the training period to verify it exists and belongs to the user
        let period = self
            .training_repository
            .get_training_period(user, period_id)
            .await
            .ok_or(GetTrainingMetricValuesError::TrainingPeriodDoesNotExist(
                period_id.clone(),
            ))?;

        let date_range = period.range_default_tomorrow();

        // Get metrics with TrainingMetricScope::TrainingPeriod
        let scope = TrainingMetricScope::TrainingPeriod(period_id.clone());
        self.get_training_metrics_values(user, &date_range, &scope)
            .await
    }

    async fn get_training_metrics_ordering(
        &self,
        user: &UserId,
        scope: &TrainingMetricScope,
    ) -> Result<TrainingMetricsOrdering, GetTrainingMetricsOrderingError> {
        // If scope is TrainingPeriod, verify the period exists and belongs to user
        if let Some(period_id) = scope.period() {
            let _period = self
                .training_repository
                .get_training_period(user, &period_id)
                .await
                .ok_or(GetTrainingMetricsOrderingError::TrainingPeriodDoesNotExist(
                    period_id,
                ))?;
        }

        self.training_repository
            .get_training_metrics_ordering(user, scope)
            .await
    }

    async fn set_training_metrics_ordering(
        &self,
        user: &UserId,
        scope: &TrainingMetricScope,
        ordering: TrainingMetricsOrdering,
    ) -> Result<(), SetTrainingMetricsOrderingError> {
        // If scope is TrainingPeriod, verify the period exists and belongs to user
        if let Some(period_id) = scope.period() {
            let _period = self
                .training_repository
                .get_training_period(user, &period_id)
                .await
                .ok_or(SetTrainingMetricsOrderingError::TrainingPeriodDoesNotExist(
                    period_id,
                ))?;
        }

        self.training_repository
            .set_training_metrics_ordering(user, scope, ordering)
            .await
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
            TrainingMetricName, TrainingNote, TrainingNoteContent, TrainingPeriod,
            TrainingPeriodWithActivities,
        },
        ports::training::{
            CopyTrainingMetricError, CopyTrainingMetricRequest, CreateTrainingPeriodError,
            CreateTrainingPeriodRequest, DeleteTrainingNoteError, DeleteTrainingPeriodError,
            DeleteTrainingPeriodRequest, GetDefinitionError, GetTrainingMetricValuesRequest,
            GetTrainingMetricsDefinitionsError, GetTrainingNoteError, SaveTrainingMetricError,
            SaveTrainingNoteError, SaveTrainingPeriodError, UpdateTrainingNoteError,
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
                date_range: &DateRange,
                scope: &TrainingMetricScope,
            ) -> Result<Vec<(TrainingMetric, TrainingMetricValues)>, GetTrainingMetricValuesError>;

            async fn copy_training_metric(
                &self,
                req: CopyTrainingMetricRequest,
            ) -> Result<(), CopyTrainingMetricError>;

            async fn delete_metric(
                &self,
                req: DeleteTrainingMetricRequest,
            ) -> Result<(), DeleteTrainingMetricError>;

            async fn update_training_metric_name(
                &self,
                req: UpdateTrainingMetricNameRequest,
            ) -> Result<(), UpdateTrainingMetricNameError>;

            async fn create_training_period(
                &self,
                req: CreateTrainingPeriodRequest,
            ) -> Result<TrainingPeriodId, CreateTrainingPeriodError>;

            async fn get_training_periods(
                &self,
                user: &UserId,
            ) -> Vec<TrainingPeriod>;

            async fn get_active_training_periods(
                &self,
                user: &UserId,
                ref_date: &NaiveDate,
            ) -> Vec<TrainingPeriod>;

            async fn get_training_period(
                &self,
                user: &UserId,
                period: &TrainingPeriodId,
            ) -> Option<TrainingPeriod>;

            async fn get_training_period_with_activities_with_metrics(
                &self,
                user: &UserId,
                period: &TrainingPeriodId,
                metrics: &[ActivityMetricV2]
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
                date_range: &Option<DateRange>,
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

            async fn get_training_period_notes(
                &self,
                user: &UserId,
                period_id: &TrainingPeriodId,
            ) -> Result<Vec<TrainingNote>, GetTrainingNoteError>;

            async fn get_training_period_metrics_values(
                &self,
                user: &UserId,
                period_id: &TrainingPeriodId,
            ) -> Result<Vec<(TrainingMetric, TrainingMetricValues)>, GetTrainingMetricValuesError>;

            async fn get_training_metrics_ordering(
                &self,
                user: &UserId,
                scope: &TrainingMetricScope,
            ) -> Result<TrainingMetricsOrdering, GetTrainingMetricsOrderingError>;

            async fn set_training_metrics_ordering(
                &self,
                user: &UserId,
                scope: &TrainingMetricScope,
                ordering: TrainingMetricsOrdering,
            ) -> Result<(), SetTrainingMetricsOrderingError>;

            async fn get_training_metric_values(
                &self,
                req: GetTrainingMetricValuesRequest,
                date_range: &DateRange,
            ) -> Result<TrainingMetricValues, GetTrainingMetricValuesError>;
        }
    }

    impl MockTrainingService {
        pub fn test_default() -> Self {
            let mut mock = Self::new();

            mock.expect_create_metric()
                .returning(|_| Ok(TrainingMetricId::default()));
            mock.expect_get_training_metrics_values()
                .returning(|_, _, _| Ok(vec![]));
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

            async fn get_global_metrics(
                &self,
                user: &UserId,
            ) -> Result<Vec<TrainingMetric>, GetTrainingMetricsDefinitionsError>;

            async fn get_period_metrics(
                &self,
                user: &UserId,
                period: &TrainingPeriodId,
            ) -> Result<Vec<TrainingMetric>, GetTrainingMetricsDefinitionsError>;

            async fn get_definition(
                &self,
                user: &UserId,
                metric: &TrainingMetricId,
            ) -> Result<Option<TrainingMetricDefinition>, GetDefinitionError>;

            async fn delete_definition(
                &self,
                user: &UserId,
                metric: &TrainingMetricId,
            ) -> Result<(), DeleteTrainingMetricError>;

            async fn update_training_metric_name(
                &self,
                user: &UserId,
                metric_id: &TrainingMetricId,
                name: TrainingMetricName,
            ) -> Result<(), UpdateTrainingMetricNameError>;

            async fn save_training_period(
                &self,
                period: TrainingPeriod,
            ) -> Result<(), SaveTrainingPeriodError>;

            async fn get_training_periods(
                &self,
                user: &UserId,
            ) -> Vec<TrainingPeriod>;

            async fn get_active_training_periods(
                &self,
                user: &UserId,
                ref_date: &NaiveDate,
            ) -> Vec<TrainingPeriod>;

            async fn get_training_period(
                &self,
                user: &UserId,
                period: &TrainingPeriodId,
            ) -> Option<TrainingPeriod>;

            async fn delete_training_period(
                &self,
                user: &UserId,
                period_id: &TrainingPeriodId,
            ) -> Result<(), DeleteTrainingPeriodError>;

            async fn update_training_period_name(
                &self,
                user: &UserId,
                period_id: &TrainingPeriodId,
                name: String,
            ) -> Result<(), UpdateTrainingPeriodNameError>;

            async fn update_training_period_note(
                &self,
                user: &UserId,
                period_id: &TrainingPeriodId,
                note: Option<String>,
            ) -> Result<(), UpdateTrainingPeriodNoteError>;

            async fn update_training_period_dates(
                &self,
                user: &UserId,
                period_id: &TrainingPeriodId,
                start: NaiveDate,
                end: Option<NaiveDate>,
            ) -> Result<(), UpdateTrainingPeriodDatesError>;

            async fn save_training_note(
                &self,
                note: TrainingNote,
            ) -> Result<(), SaveTrainingNoteError>;

            async fn get_training_note(
                &self,
                user: &UserId,
                note_id: &TrainingNoteId,
            ) -> Result<Option<TrainingNote>, GetTrainingNoteError>;

            async fn get_training_notes(
                &self,
                user: &UserId,
                date_range: &Option<DateRange>,
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

            async fn get_training_metrics_ordering(
                &self,
                user: &UserId,
                scope: &TrainingMetricScope,
            ) -> Result<TrainingMetricsOrdering, GetTrainingMetricsOrderingError>;

            async fn set_training_metrics_ordering(
                &self,
                user: &UserId,
                scope: &TrainingMetricScope,
                ordering: TrainingMetricsOrdering,
            ) -> Result<(), SetTrainingMetricsOrderingError>;
        }
    }
}

#[cfg(test)]
mod tests_training_metrics_service {

    use std::collections::HashMap;

    use anyhow::anyhow;

    use super::*;
    use crate::domain::models::activity::{
        Activity, ActivityDuration, ActivityId, ActivityMetricV2, ActivityMetricsV2,
        ActivityStartTime, Sport,
    };

    use crate::domain::models::training::{
        SportFilter, TrainingMetricBin, TrainingPeriod, TrainingPeriodSports,
    };
    use crate::domain::ports::DateRange;
    use crate::domain::services::activity::test_utils::MockActivityService;
    use crate::domain::{
        models::training::{
            TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricFilters,
            TrainingMetricGranularity, TrainingMetricGroupBy, TrainingMetricId, TrainingMetricName,
        },
        ports::training::{GetTrainingMetricsDefinitionsError, SaveTrainingMetricError},
        services::training::test_utils::MockTrainingRepository,
    };
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    #[tokio::test]
    async fn test_create_metric_ok() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_save_training_metric_definition()
            .returning(|_| Ok(()));
        let activities = MockActivityService::new();

        let service = TrainingService::new(repository, activities);

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            TrainingMetricName::from("Test Metric"),
            ActivityMetricV2::Calories,
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty(),
            TrainingMetricGroupBy::none(),
            TrainingMetricScope::Global,
        );

        let _ = service
            .create_metric(req)
            .await
            .expect("Should have return ok");
    }

    #[tokio::test]
    async fn test_create_metric_add_period_sports_to_metric_filters() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_training_period().returning(|_, _| {
            Some(
                TrainingPeriod::new(
                    TrainingPeriodId::new(),
                    UserId::test_default(),
                    "2025-10-17".parse::<NaiveDate>().unwrap(),
                    Some("2025-10-21".parse::<NaiveDate>().unwrap()),
                    "Test Period".to_string(),
                    TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::Running)])),
                    None,
                )
                .unwrap(),
            )
        });
        repository
            .expect_save_training_metric_definition()
            .withf(|metric| {
                metric.definition().filters().sports()
                    == &Some(vec![
                        SportFilter::Sport(Sport::AlpineSki),
                        SportFilter::Sport(Sport::Running),
                    ])
            })
            .returning(|_| Ok(()));
        let activities = MockActivityService::new();

        let service = TrainingService::new(repository, activities);

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            TrainingMetricName::from("Test Metric"),
            ActivityMetricV2::Calories,
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty()
                .merge_sports(&Some(vec![SportFilter::Sport(Sport::AlpineSki)])),
            TrainingMetricGroupBy::none(),
            TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from("period-id")),
        );

        let _ = service
            .create_metric(req)
            .await
            .expect("Should have return ok");
    }

    #[tokio::test]
    async fn test_create_metric_fails_if_training_period_does_not_exist() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_training_period()
            .returning(|_, _| None);
        repository.expect_save_training_metric_definition().times(0);
        let activities = MockActivityService::new();

        let service = TrainingService::new(repository, activities);

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            TrainingMetricName::from("Test Metric"),
            ActivityMetricV2::Calories,
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty()
                .merge_sports(&Some(vec![SportFilter::Sport(Sport::AlpineSki)])),
            TrainingMetricGroupBy::none(),
            TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from("period-id")),
        );

        let Err(CreateTrainingMetricError::TrainingPeriodDoesNotExist(period)) =
            service.create_metric(req).await
        else {
            unreachable!("Should have err")
        };
        assert_eq!(period, TrainingPeriodId::from("period-id"))
    }

    #[tokio::test]
    async fn test_create_metric_fails_to_save_definition() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_save_training_metric_definition()
            .returning(|_| Err(SaveTrainingMetricError::Unknown(anyhow!("error"))));
        let activities = MockActivityService::new();
        let service = TrainingService::new(repository, activities);

        let req = CreateTrainingMetricRequest::new(
            UserId::test_default(),
            TrainingMetricName::from("Test Metric"),
            ActivityMetricV2::Calories,
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricFilters::empty(),
            TrainingMetricGroupBy::none(),
            TrainingMetricScope::Global,
        );

        let _ = service
            .create_metric(req)
            .await
            .expect_err("Should have return an err");
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_when_get_definitions_err() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_global_metrics().returning(|_| {
            Err(GetTrainingMetricsDefinitionsError::Unknown(anyhow!(
                "an error"
            )))
        });

        let activity_service = MockActivityService::new();
        let service = TrainingService::new(repository, activity_service);

        let err = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &DateRange::new(
                    NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
                ),
                &TrainingMetricScope::Global,
            )
            .await
            .unwrap_err();

        let GetTrainingMetricValuesError::Unknown(_err) = err else {
            unreachable!("Should have err")
        };
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_def_without_values() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_global_metrics().returning(|_| {
            Ok(vec![TrainingMetric::new(
                TrainingMetricId::from("test"),
                None,
                TrainingMetricScope::Global,
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricV2::Calories,
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Average,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )])
        });
        repository
            .expect_get_training_metrics_ordering()
            .returning(|_, _| Ok(TrainingMetricsOrdering::default()));

        let mut activity_service = MockActivityService::default();
        // When no date range is specified, it should query the user's history
        activity_service
            .expect_list_activities_with_metrics()
            .returning(|_, _, _| Ok(vec![]));

        let service = TrainingService::new(repository, activity_service);

        let res = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &DateRange::new(
                    NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
                ),
                &TrainingMetricScope::Global,
            )
            .await
            .unwrap();

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetric::new(
                TrainingMetricId::from("test"),
                None,
                TrainingMetricScope::Global,
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricV2::Calories,
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Average,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )
        );
        assert!(value.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_map_def_with_its_values() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_global_metrics().returning(|_| {
            Ok(vec![TrainingMetric::new(
                TrainingMetricId::from("test"),
                None,
                TrainingMetricScope::Global,
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricV2::Calories,
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )])
        });
        repository
            .expect_get_training_metrics_ordering()
            .returning(|_, _| Ok(TrainingMetricsOrdering::default()));

        let mut activity_service = MockActivityService::default();

        activity_service
            .expect_list_activities_with_metrics()
            .returning(|_, _, _| {
                // Empty as no activity has this metric
                Ok(vec![])
            });

        let service = TrainingService::new(repository, activity_service);

        let res = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &DateRange::new(
                    NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
                ),
                &TrainingMetricScope::Global,
            )
            .await
            .unwrap();

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetric::new(
                TrainingMetricId::from("test"),
                None,
                TrainingMetricScope::Global,
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricV2::Calories,
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )
        );
        // With default stats (no calories), should have empty values
        assert!(value.is_empty());
    }

    #[tokio::test]
    async fn test_training_service_get_metrics_with_date_range() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_global_metrics().returning(|_| {
            Ok(vec![TrainingMetric::new(
                TrainingMetricId::from("test"),
                None,
                TrainingMetricScope::Global,
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricV2::Calories,
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Average,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )])
        });
        repository
            .expect_get_training_metrics_ordering()
            .returning(|_, _| Ok(TrainingMetricsOrdering::default()));

        let mut activity_service = MockActivityService::default();
        // Should list activities in the date range
        activity_service
            .expect_list_activities_with_metrics()
            .withf(|_, filters, _| {
                // The date range should be aligned to full days (Sep 24-26)
                filters.date_range().is_some()
                    && filters.date_range().as_ref().unwrap().start()
                        == &NaiveDate::from_ymd_opt(2025, 9, 24).unwrap()
                    && filters.date_range().as_ref().unwrap().end()
                        == &NaiveDate::from_ymd_opt(2025, 9, 26).unwrap()
            })
            .returning(|_, _, _| Ok(vec![]));

        let activity_service = activity_service;
        let service = TrainingService::new(repository, activity_service);

        let date_range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
            NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
        );

        let res = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &date_range,
                &TrainingMetricScope::Global,
            )
            .await
            .unwrap();

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_training_service_get_metrics_aligns_date_range_to_granularity() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_global_metrics().returning(|_| {
            Ok(vec![TrainingMetric::new(
                TrainingMetricId::from("test"),
                None,
                TrainingMetricScope::Global,
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricV2::Distance,
                    TrainingMetricGranularity::Weekly, // Weekly granularity
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )])
        });
        repository
            .expect_get_training_metrics_ordering()
            .returning(|_, _| Ok(TrainingMetricsOrdering::default()));

        let mut activity_service = MockActivityService::default();
        // The date range will be aligned to week boundaries (Monday)
        // Input: Wed Sep 24 to Thu Sep 25, 2025
        // Should be aligned to: Mon Sep 22 to Mon Sep 29, 2025
        activity_service
            .expect_list_activities_with_metrics()
            .withf(|_, filters, _| {
                let Some(range) = filters.date_range() else {
                    return false;
                };
                // Verify that the range starts on Monday (Sep 22) and ends on following Monday (Sep 29)
                *range.start() == NaiveDate::from_ymd_opt(2025, 9, 22).unwrap()
                    && *range.end() == NaiveDate::from_ymd_opt(2025, 9, 29).unwrap()
            })
            .returning(|_, _, _| {
                Ok(vec![(
                    Activity::new_empty(
                        ActivityId::from("test"),
                        UserId::test_default(),
                        ActivityStartTime::new(
                            NaiveDateTime::new(
                                NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                                NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                            )
                            .and_utc()
                            .fixed_offset(),
                        ),
                        ActivityDuration::default(),
                        Sport::Running,
                    ),
                    ActivityMetricsV2::new(HashMap::from([(ActivityMetricV2::Distance, Some(0.))])),
                )])
            });

        let activity_service = activity_service;
        let service = TrainingService::new(repository, activity_service);

        // Input date range: Wednesday to Thursday (mid-week)
        let date_range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(), // Wednesday
            NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(), // Thursday
        );

        let res = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &date_range,
                &TrainingMetricScope::Global,
            )
            .await
            .unwrap();

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_get_training_metrics_values_with_global_scope() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_global_metrics().returning(|_| {
            Ok(vec![TrainingMetric::new(
                TrainingMetricId::from("global-metric"),
                Some(TrainingMetricName::from("Global Metric")),
                TrainingMetricScope::Global,
                TrainingMetricDefinition::new(
                    UserId::test_default(),
                    ActivityMetricV2::Distance,
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Sum,
                    TrainingMetricFilters::empty(),
                    TrainingMetricGroupBy::none(),
                ),
            )])
        });
        repository
            .expect_get_training_metrics_ordering()
            .returning(|_, _| Ok(TrainingMetricsOrdering::default()));

        let mut activity_service = MockActivityService::default();
        activity_service
            .expect_list_activities_with_metrics()
            .returning(|_, _, _| Ok(vec![]));
        let activity_service = activity_service;
        let service = TrainingService::new(repository, activity_service);

        let res = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &DateRange::new(
                    NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
                ),
                &TrainingMetricScope::Global,
            )
            .await
            .unwrap();

        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0.id(), &TrainingMetricId::from("global-metric"));
        assert_eq!(
            res[0].0.name(),
            &Some(TrainingMetricName::from("Global Metric"))
        );
    }

    #[tokio::test]
    async fn test_get_training_metrics_values_with_training_period_scope() {
        let mut repository = MockTrainingRepository::new();

        repository.expect_get_training_period().returning(|_, _| {
            Some(
                TrainingPeriod::new(
                    TrainingPeriodId::from("test-period"),
                    UserId::test_default(),
                    "2025-10-17".parse::<NaiveDate>().unwrap(),
                    Some("2025-10-21".parse::<NaiveDate>().unwrap()),
                    "Test Period".to_string(),
                    TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::Running)])),
                    None,
                )
                .unwrap(),
            )
        });
        repository
            .expect_get_training_metrics_ordering()
            .returning(|_, _| Ok(TrainingMetricsOrdering::default()));

        repository
            .expect_get_period_metrics()
            .withf(move |_, period| period == &TrainingPeriodId::from("test-period"))
            .returning(|_, _| {
                Ok(vec![TrainingMetric::new(
                    TrainingMetricId::from("period-metric"),
                    Some(TrainingMetricName::from("Period Metric")),
                    TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from("test-period")),
                    TrainingMetricDefinition::new(
                        UserId::test_default(),
                        ActivityMetricV2::Duration,
                        TrainingMetricGranularity::Weekly,
                        TrainingMetricAggregate::Average,
                        TrainingMetricFilters::empty(),
                        TrainingMetricGroupBy::none(),
                    ),
                )])
            });

        let mut activity_service = MockActivityService::default();
        activity_service
            .expect_list_activities_with_metrics()
            .returning(|_, _, _| Ok(vec![]));
        let activity_service = activity_service;
        let service = TrainingService::new(repository, activity_service);

        let res = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &DateRange::new(
                    NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
                ),
                &TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from("test-period")),
            )
            .await
            .unwrap();

        // Should have both global and period metrics merged
        assert_eq!(res.len(), 1);

        let period_metric = res
            .iter()
            .find(|(m, _)| m.id() == &TrainingMetricId::from("period-metric"));
        assert!(period_metric.is_some());
    }

    #[tokio::test]
    async fn test_get_training_metrics_values_with_training_period_handles_period_error() {
        let period_id = TrainingPeriodId::new();
        let mut repository = MockTrainingRepository::new();

        repository.expect_get_training_period().returning(|_, _| {
            Some(
                TrainingPeriod::new(
                    TrainingPeriodId::from("metric-1"),
                    UserId::test_default(),
                    "2025-10-17".parse::<NaiveDate>().unwrap(),
                    Some("2025-10-21".parse::<NaiveDate>().unwrap()),
                    "Test Period".to_string(),
                    TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::AlpineSki)])),
                    None,
                )
                .unwrap(),
            )
        });

        // Period metrics returns error
        let test_period_id = period_id.clone();
        repository
            .expect_get_period_metrics()
            .withf(move |_, period| period == &test_period_id)
            .returning(|_, _| {
                Err(GetTrainingMetricsDefinitionsError::Unknown(anyhow!(
                    "error"
                )))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let err = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &DateRange::new(
                    NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
                ),
                &TrainingMetricScope::TrainingPeriod(period_id.clone()),
            )
            .await
            .unwrap_err();

        let GetTrainingMetricValuesError::Unknown(_err) = err else {
            unreachable!("Should have err")
        };
    }

    #[tokio::test]
    async fn test_get_training_metrics_values_returns_metrics_when_ordering_fails() {
        let mut repository = MockTrainingRepository::new();

        // Create two metrics
        let metric_id_1 = TrainingMetricId::from("metric-1");
        let metric_id_2 = TrainingMetricId::from("metric-2");

        repository.expect_get_global_metrics().returning(move |_| {
            Ok(vec![
                TrainingMetric::new(
                    metric_id_1.clone(),
                    Some(TrainingMetricName::from("Metric 1")),
                    TrainingMetricScope::Global,
                    TrainingMetricDefinition::new(
                        UserId::test_default(),
                        ActivityMetricV2::Distance,
                        TrainingMetricGranularity::Daily,
                        TrainingMetricAggregate::Sum,
                        TrainingMetricFilters::empty(),
                        TrainingMetricGroupBy::none(),
                    ),
                ),
                TrainingMetric::new(
                    metric_id_2.clone(),
                    Some(TrainingMetricName::from("Metric 2")),
                    TrainingMetricScope::Global,
                    TrainingMetricDefinition::new(
                        UserId::test_default(),
                        ActivityMetricV2::Distance,
                        TrainingMetricGranularity::Daily,
                        TrainingMetricAggregate::Sum,
                        TrainingMetricFilters::empty(),
                        TrainingMetricGroupBy::none(),
                    ),
                ),
            ])
        });

        // Getting ordering returns an error
        repository
            .expect_get_training_metrics_ordering()
            .returning(|_, _| {
                Err(GetTrainingMetricsOrderingError::Unknown(anyhow!(
                    "ordering error"
                )))
            });

        let mut activity_service = MockActivityService::default();
        activity_service
            .expect_list_activities_with_metrics()
            .returning(|_, _, _| Ok(vec![]));

        let activity_service = activity_service;
        let service = TrainingService::new(repository, activity_service);

        let res = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &DateRange::new(
                    NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
                ),
                &TrainingMetricScope::Global,
            )
            .await
            .unwrap();

        // Metrics should still be returned even when ordering fails
        assert_eq!(res.len(), 2);
        // Verify both metrics are present (order doesn't matter since ordering failed)
        let ids: Vec<_> = res.iter().map(|(m, _)| m.id()).collect();
        assert!(ids.contains(&&TrainingMetricId::from("metric-1")));
        assert!(ids.contains(&&TrainingMetricId::from("metric-2")));
    }

    #[tokio::test]
    async fn test_get_training_metrics_values_applies_ordering() {
        let mut repository = MockTrainingRepository::new();

        // Create three metrics
        let metric_id_1 = TrainingMetricId::from("metric-1");
        let metric_id_2 = TrainingMetricId::from("metric-2");
        let metric_id_3 = TrainingMetricId::from("metric-3");

        // Clone for use in ordering closure
        let ordering_id_1 = metric_id_1.clone();
        let ordering_id_2 = metric_id_2.clone();
        let ordering_id_3 = metric_id_3.clone();

        // Return metrics in one order (1, 2, 3)
        repository.expect_get_global_metrics().returning(move |_| {
            Ok(vec![
                TrainingMetric::new(
                    metric_id_1.clone(),
                    Some(TrainingMetricName::from("Metric 1")),
                    TrainingMetricScope::Global,
                    TrainingMetricDefinition::new(
                        UserId::test_default(),
                        ActivityMetricV2::Distance,
                        TrainingMetricGranularity::Daily,
                        TrainingMetricAggregate::Sum,
                        TrainingMetricFilters::empty(),
                        TrainingMetricGroupBy::none(),
                    ),
                ),
                TrainingMetric::new(
                    metric_id_2.clone(),
                    Some(TrainingMetricName::from("Metric 2")),
                    TrainingMetricScope::Global,
                    TrainingMetricDefinition::new(
                        UserId::test_default(),
                        ActivityMetricV2::Distance,
                        TrainingMetricGranularity::Daily,
                        TrainingMetricAggregate::Sum,
                        TrainingMetricFilters::empty(),
                        TrainingMetricGroupBy::none(),
                    ),
                ),
                TrainingMetric::new(
                    metric_id_3.clone(),
                    Some(TrainingMetricName::from("Metric 3")),
                    TrainingMetricScope::Global,
                    TrainingMetricDefinition::new(
                        UserId::test_default(),
                        ActivityMetricV2::Distance,
                        TrainingMetricGranularity::Daily,
                        TrainingMetricAggregate::Sum,
                        TrainingMetricFilters::empty(),
                        TrainingMetricGroupBy::none(),
                    ),
                ),
            ])
        });

        // Return ordering in different order: 3, 1, 2
        repository
            .expect_get_training_metrics_ordering()
            .returning(move |_, _| {
                Ok(TrainingMetricsOrdering::try_from(vec![
                    ordering_id_3.clone(),
                    ordering_id_1.clone(),
                    ordering_id_2.clone(),
                ])
                .unwrap())
            });

        let mut activity_service = MockActivityService::default();
        activity_service
            .expect_list_activities_with_metrics()
            .returning(|_, _, _| Ok(vec![]));

        let activity_service = activity_service;
        let service = TrainingService::new(repository, activity_service);

        let res = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &DateRange::new(
                    NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
                ),
                &TrainingMetricScope::Global,
            )
            .await
            .unwrap();

        // Metrics should be returned in the order specified by ordering: 3, 1, 2
        assert_eq!(res.len(), 3);
        assert_eq!(res[0].0.id(), &TrainingMetricId::from("metric-3"));
        assert_eq!(res[1].0.id(), &TrainingMetricId::from("metric-1"));
        assert_eq!(res[2].0.id(), &TrainingMetricId::from("metric-2"));
    }

    #[tokio::test]
    async fn test_get_training_metrics_values_applies_training_period_sports_to_metrics_filters() {
        let mut repository = MockTrainingRepository::new();

        repository.expect_get_training_period().returning(|_, _| {
            Some(
                TrainingPeriod::new(
                    TrainingPeriodId::from("metric-1"),
                    UserId::test_default(),
                    "2025-10-17".parse::<NaiveDate>().unwrap(),
                    Some("2025-10-21".parse::<NaiveDate>().unwrap()),
                    "Test Period".to_string(),
                    TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::AlpineSki)])),
                    None,
                )
                .unwrap(),
            )
        });

        repository
            .expect_get_period_metrics()
            .returning(move |_, _| {
                Ok(vec![
                    TrainingMetric::new(
                        TrainingMetricId::from("metric-1"),
                        Some(TrainingMetricName::from("Metric 1")),
                        TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from("period-id")),
                        TrainingMetricDefinition::new(
                            UserId::test_default(),
                            ActivityMetricV2::Distance,
                            TrainingMetricGranularity::Daily,
                            TrainingMetricAggregate::Sum,
                            TrainingMetricFilters::empty()
                                .merge_sports(&Some(vec![SportFilter::Sport(Sport::AlpineSki)])),
                            TrainingMetricGroupBy::none(),
                        ),
                    ),
                    TrainingMetric::new(
                        TrainingMetricId::from("metric-2"),
                        Some(TrainingMetricName::from("Metric 2")),
                        TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from("period-id")),
                        TrainingMetricDefinition::new(
                            UserId::test_default(),
                            ActivityMetricV2::Distance,
                            TrainingMetricGranularity::Daily,
                            TrainingMetricAggregate::Sum,
                            TrainingMetricFilters::empty(),
                            TrainingMetricGroupBy::none(),
                        ),
                    ),
                ])
            });

        repository
            .expect_get_training_metrics_ordering()
            .returning(move |_, _| {
                Ok(TrainingMetricsOrdering::try_from(vec![
                    TrainingMetricId::from("metric-1"),
                    TrainingMetricId::from("metric-2"),
                ])
                .unwrap())
            });

        let mut activity_service = MockActivityService::default();
        activity_service
            .expect_list_activities_with_metrics()
            .returning(|_, _, _| {
                Ok(vec![(
                    Activity::new_empty(
                        ActivityId::from("test"),
                        UserId::test_default(),
                        ActivityStartTime::new(
                            NaiveDateTime::new(
                                NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                                NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                            )
                            .and_utc()
                            .fixed_offset(),
                        ),
                        ActivityDuration::default(),
                        Sport::Running,
                    ),
                    ActivityMetricsV2::new(HashMap::from([(
                        ActivityMetricV2::Distance,
                        Some(10.),
                    )])),
                )])
            });

        let activity_service = activity_service;
        let service = TrainingService::new(repository, activity_service);

        let res = service
            .get_training_metrics_values(
                &UserId::test_default(),
                &DateRange::new(
                    NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
                ),
                &TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from("period-id")),
            )
            .await
            .unwrap();

        assert_eq!(
            res.get(0)
                .unwrap()
                .1
                .get(&TrainingMetricBin::from_granule("2025-09-24")),
            None // None as activity should be excluded from the metric computation
        );
        assert_eq!(
            res.get(1)
                .unwrap()
                .1
                .get(&TrainingMetricBin::from_granule("2025-09-24")),
            None // None as activity should be excluded from the metric computation
        )
    }

    #[tokio::test]
    async fn test_training_service_delete_metric_does_not_exist() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_delete_definition().returning(|_, _| {
            Err(DeleteTrainingMetricError::MetricDoesNotExist(
                TrainingMetricId::from("test"),
            ))
        });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

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
    async fn test_training_service_delete_metric() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definition().returning(|_, _| {
            Ok(Some(TrainingMetricDefinition::new(
                "user".to_string().into(),
                ActivityMetricV2::Calories,
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
            )))
        });
        repository
            .expect_delete_definition()
            .times(1)
            .withf(|user, id| {
                user == &UserId::from("user") && id == &TrainingMetricId::from("test")
            })
            .returning(|_, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let req = DeleteTrainingMetricRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
        );

        let res = service.delete_metric(req).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_training_service_update_metric_name_does_not_exist() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_update_training_metric_name()
            .returning(|_, _, _| {
                Err(UpdateTrainingMetricNameError::MetricDoesNotExist(
                    TrainingMetricId::from("test"),
                ))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let req = UpdateTrainingMetricNameRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
            TrainingMetricName::from("Updated Name"),
        );

        let res = service.update_training_metric_name(req).await;

        let Err(UpdateTrainingMetricNameError::MetricDoesNotExist(metric)) = res else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(metric, TrainingMetricId::from("test"));
    }

    #[tokio::test]
    async fn test_training_service_update_metric_name() {
        let mut repository = MockTrainingRepository::new();
        repository.expect_get_definition().returning(|_, _| {
            Ok(Some(TrainingMetricDefinition::new(
                "user".to_string().into(),
                ActivityMetricV2::Calories,
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
            )))
        });
        repository
            .expect_update_training_metric_name()
            .times(1)
            .withf(|user, id, name| {
                user == &UserId::from("user")
                    && id == &TrainingMetricId::from("test")
                    && name == &TrainingMetricName::from("Updated Name")
            })
            .returning(|_, _, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let req = UpdateTrainingMetricNameRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
            TrainingMetricName::from("Updated Name"),
        );

        let res = service.update_training_metric_name(req).await;

        assert!(res.is_ok());
    }
}

#[cfg(test)]
mod test_training_service_period {
    use anyhow::anyhow;
    use chrono::NaiveDate;

    use crate::domain::{
        models::{
            activity::{ActivityDuration, ActivityMetricsV2},
            training::{TrainingPeriod, TrainingPeriodSports},
        },
        ports::{
            activity::ListActivitiesError,
            training::{CreateTrainingPeriodRequest, SaveTrainingPeriodError},
        },
        services::{
            activity::test_utils::MockActivityService, training::test_utils::MockTrainingRepository,
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
        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

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
        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

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
    async fn test_get_training_period_with_activities_period_not_found() {
        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| None);
        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let result = service
            .get_training_period_with_activities_with_metrics(
                &UserId::test_default(),
                &TrainingPeriodId::new(),
                &[],
            )
            .await;

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_no_sport_filter() {
        use crate::domain::models::activity::{Activity, ActivityId, ActivityStartTime, Sport};

        // Create test activities with different sports and dates
        let activities = vec![
            (
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-18T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Running,
                ),
                ActivityMetricsV2::default(),
            ),
            (
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-19T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Cycling,
                ),
                ActivityMetricsV2::default(),
            ),
            (
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-20T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Swimming,
                ),
                ActivityMetricsV2::default(),
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

        let mut activity_service = MockActivityService::new();
        let activities_clone = activities.clone();
        activity_service
            .expect_list_activities_with_metrics()
            .times(1)
            .returning(move |_, _, _| Ok(activities_clone.clone()));

        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_period_with_activities_with_metrics(
                &UserId::test_default(),
                &TrainingPeriodId::new(),
                &[],
            )
            .await;

        assert!(result.is_some());
        let period_with_activities = result.unwrap();
        assert_eq!(period_with_activities.activities().len(), 3);
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_with_sport_filter() {
        use crate::domain::models::activity::{Activity, ActivityId, ActivityStartTime, Sport};
        use crate::domain::models::training::SportFilter;

        // Create test activities with different sports
        let activities = vec![
            (
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-18T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Running,
                ),
                ActivityMetricsV2::default(),
            ),
            (
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-19T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Cycling,
                ),
                ActivityMetricsV2::default(),
            ),
            (
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-20T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Swimming,
                ),
                ActivityMetricsV2::default(),
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

        let mut activity_service = MockActivityService::new();
        let activities_clone = activities.clone();
        activity_service
            .expect_list_activities_with_metrics()
            .times(1)
            .returning(move |_, _, _| Ok(activities_clone.clone()));

        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_period_with_activities_with_metrics(
                &UserId::test_default(),
                &TrainingPeriodId::new(),
                &[],
            )
            .await;

        assert!(result.is_some());
        let period_with_activities = result.unwrap();
        // Should only include Running activity
        assert_eq!(period_with_activities.activities().len(), 1);
        assert_eq!(
            period_with_activities.activities()[0].0.sport(),
            &Sport::Running
        );
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_with_category_filter() {
        use crate::domain::models::activity::{
            Activity, ActivityId, ActivityStartTime, Sport, SportCategory,
        };
        use crate::domain::models::training::SportFilter;

        // Create test activities with different sports in the Running category
        let activities = vec![
            (
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-18T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Running,
                ),
                ActivityMetricsV2::default(),
            ),
            (
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-19T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::TrailRunning,
                ),
                ActivityMetricsV2::default(),
            ),
            (
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-20T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Cycling,
                ),
                ActivityMetricsV2::default(),
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

        let mut activity_service = MockActivityService::new();
        let activities_clone = activities.clone();
        activity_service
            .expect_list_activities_with_metrics()
            .times(1)
            .returning(move |_, _, _| Ok(activities_clone.clone()));

        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_period_with_activities_with_metrics(
                &UserId::test_default(),
                &TrainingPeriodId::new(),
                &[],
            )
            .await;

        assert!(result.is_some());
        let period_with_activities = result.unwrap();
        // Should include Running and TrailRunning, but not Cycling
        assert_eq!(period_with_activities.activities().len(), 2);
        assert!(
            period_with_activities
                .activities()
                .iter()
                .any(|a| a.0.sport() == &Sport::Running)
        );
        assert!(
            period_with_activities
                .activities()
                .iter()
                .any(|a| a.0.sport() == &Sport::TrailRunning)
        );
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_date_filtering() {
        use crate::domain::models::activity::{Activity, ActivityId, ActivityStartTime, Sport};

        // Create activities with dates both inside and outside the period
        let activities = vec![
            (
                // Before period
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-16T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Running,
                ),
                ActivityMetricsV2::default(),
            ),
            (
                // Inside period
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-18T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Running,
                ),
                ActivityMetricsV2::default(),
            ),
            (
                // After period
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(
                        "2025-10-22T10:00:00Z"
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .unwrap()
                            .timestamp()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ActivityDuration::default(),
                    Sport::Running,
                ),
                ActivityMetricsV2::default(),
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

        let mut activity_service = MockActivityService::new();
        let activities_clone = activities.clone();
        activity_service
            .expect_list_activities_with_metrics()
            .times(1)
            .returning(move |_, _, _| Ok(activities_clone.clone()));

        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_period_with_activities_with_metrics(
                &UserId::test_default(),
                &TrainingPeriodId::new(),
                &[],
            )
            .await;

        assert!(result.is_some());
        let period_with_activities = result.unwrap();
        // Should only include the activity inside the period (2025-10-18)
        assert_eq!(period_with_activities.activities().len(), 1);
    }

    #[tokio::test]
    async fn test_get_training_period_with_activities_open_ended_includes_today() {
        use crate::domain::models::activity::{Activity, ActivityId, ActivityStartTime, Sport};
        use chrono::{Days, Utc};

        let today = Utc::now().date_naive();
        let yesterday = today - Days::new(1);

        // Create activities: yesterday and today
        // Note: Tomorrow's activity wouldn't be returned by the repository
        // because the date range filter is exclusive of the end date
        let activities = vec![
            (
                // Yesterday - should be included
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
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
                    ActivityDuration::default(),
                    Sport::Running,
                ),
                ActivityMetricsV2::default(),
            ),
            (
                // Today - should be included (this is the bug we're fixing)
                Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
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
                    ActivityDuration::default(),
                    Sport::Cycling,
                ),
                ActivityMetricsV2::default(),
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

        let mut activity_service = MockActivityService::new();
        let activities_clone = activities.clone();
        activity_service
            .expect_list_activities_with_metrics()
            .times(1)
            .returning(move |_, _, _| Ok(activities_clone.clone()));

        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_period_with_activities_with_metrics(
                &UserId::test_default(),
                &TrainingPeriodId::new(),
                &[],
            )
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
            .map(|a| a.0.sport())
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

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_list_activities_with_metrics()
            .times(1)
            .returning(|_, _, _| Err(ListActivitiesError::Unknown(anyhow!("database error"))));

        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_period_with_activities_with_metrics(
                &UserId::test_default(),
                &TrainingPeriodId::new(),
                &[],
            )
            .await;

        // Should return None when repository fails
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_training_period_ok() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_delete_training_period()
            .times(1)
            .returning(|_, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let req = DeleteTrainingPeriodRequest::new(user_id, period_id);
        let result = service.delete_training_period(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_training_period_not_found() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();

        let mut training_repository = MockTrainingRepository::new();
        let period = period_id.clone();
        training_repository
            .expect_delete_training_period()
            .times(1)
            .returning(move |_, _| {
                Err(DeleteTrainingPeriodError::PeriodDoesNotExist(
                    period.clone(),
                ))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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
    async fn test_delete_training_period_repository_error() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_delete_training_period()
            .times(1)
            .returning(|_, _| {
                Err(DeleteTrainingPeriodError::Unknown(anyhow!(
                    "database error"
                )))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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

        let mut training_repository = MockTrainingRepository::new();
        let period_id_clone = period_id.clone();
        training_repository
            .expect_update_training_period_name()
            .times(1)
            .withf(move |user, id, name| {
                user == &UserId::test_default()
                    && id == &period_id_clone
                    && name == "Updated Period Name"
            })
            .returning(|_, _, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let req = UpdateTrainingPeriodNameRequest::new(user_id, period_id, new_name);
        let result = service.update_training_period_name(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_name_period_not_found() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_name = "Updated Period Name".to_string();

        let mut training_repository = MockTrainingRepository::new();
        let period = period_id.clone();
        training_repository
            .expect_update_training_period_name()
            .times(1)
            .return_once(move |_, _, _| {
                Err(UpdateTrainingPeriodNameError::PeriodDoesNotExist(
                    period.clone(),
                ))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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
    async fn test_update_training_period_name_repository_error() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_name = "Updated Period Name".to_string();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_update_training_period_name()
            .times(1)
            .returning(|_, _, _| {
                Err(UpdateTrainingPeriodNameError::Unknown(anyhow!(
                    "database error"
                )))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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

        let mut training_repository = MockTrainingRepository::new();

        let period_id_clone = period_id.clone();
        let new_note_clone = new_note.clone();
        training_repository
            .expect_update_training_period_note()
            .times(1)
            .withf(move |user, id, note| {
                user == &UserId::test_default() && id == &period_id_clone && note == &new_note_clone
            })
            .returning(|_, _, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let req = UpdateTrainingPeriodNoteRequest::new(user_id, period_id, new_note);
        let result = service.update_training_period_note(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_note_clear_note() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_note = None;

        let mut training_repository = MockTrainingRepository::new();
        let period_id_clone = period_id.clone();
        training_repository
            .expect_update_training_period_note()
            .times(1)
            .withf(move |user, id, note| {
                user == &UserId::test_default() && id == &period_id_clone && note.is_none()
            })
            .returning(|_, _, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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
        let period = period_id.clone();
        training_repository
            .expect_update_training_period_note()
            .times(1)
            .return_once(move |_, _, _| {
                Err(UpdateTrainingPeriodNoteError::PeriodDoesNotExist(
                    period.clone(),
                ))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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
    async fn test_update_training_period_note_repository_error() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_note = Some("Updated note".to_string());

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_update_training_period_note()
            .times(1)
            .returning(|_, _, _| {
                Err(UpdateTrainingPeriodNoteError::Unknown(anyhow!(
                    "database error"
                )))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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

        let mut training_repository = MockTrainingRepository::new();

        let period_id_clone = period_id.clone();
        let new_start_clone = new_start;
        let new_end_clone = new_end;
        training_repository
            .expect_update_training_period_dates()
            .times(1)
            .withf(move |user, id, start, end| {
                user == &UserId::test_default()
                    && id == &period_id_clone
                    && start == &new_start_clone
                    && end == &new_end_clone
            })
            .returning(|_, _, _, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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

        let mut training_repository = MockTrainingRepository::new();

        let period_id_clone = period_id.clone();
        let new_start_clone = new_start;
        training_repository
            .expect_update_training_period_dates()
            .times(1)
            .withf(move |user, id, start, end| {
                user == &UserId::test_default()
                    && id == &period_id_clone
                    && start == &new_start_clone
                    && end.is_none()
            })
            .returning(|_, _, _, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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

        let training_repository = MockTrainingRepository::new();

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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
        let period = period_id.clone();
        training_repository
            .expect_update_training_period_dates()
            .times(1)
            .return_once(move |_, _, _, _| {
                Err(UpdateTrainingPeriodDatesError::PeriodDoesNotExist(
                    period.clone(),
                ))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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
    async fn test_update_training_period_dates_repository_error() {
        let period_id = TrainingPeriodId::new();
        let user_id = UserId::test_default();
        let new_start = "2025-11-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-11-30".parse::<NaiveDate>().unwrap());

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_update_training_period_dates()
            .times(1)
            .returning(|_, _, _, _| {
                Err(UpdateTrainingPeriodDatesError::Unknown(anyhow!(
                    "database error"
                )))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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
    use crate::domain::ports::training::{
        CreateTrainingNoteError, CreateTrainingNoteRequest, SaveTrainingNoteError,
    };
    use crate::domain::services::activity::test_utils::MockActivityService;
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

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

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

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let req = CreateTrainingNoteRequest::new(user_id, title, content, date);
        let result = service.create_training_note(req).await;

        assert!(result.is_err());
        match result {
            Err(CreateTrainingNoteError::Unknown(_)) => {}
            _ => panic!("Expected Unknown error"),
        }
    }

    #[tokio::test]
    async fn test_get_training_period_notes_ok() {
        let user_id = UserId::from("user1");
        let period_id = TrainingPeriodId::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = Some(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap());

        let mut training_repository = MockTrainingRepository::new();

        // Mock get_training_period
        let period_id_clone = period_id.clone();
        training_repository
            .expect_get_training_period()
            .times(1)
            .returning(move |_, _| {
                use crate::domain::models::training::{TrainingPeriod, TrainingPeriodSports};
                Some(
                    TrainingPeriod::new(
                        period_id_clone.clone(),
                        UserId::from("user1"),
                        start,
                        end,
                        "Test Period".to_string(),
                        TrainingPeriodSports::new(None),
                        None,
                    )
                    .unwrap(),
                )
            });

        // Mock get_training_notes - will be called with a date range
        training_repository
            .expect_get_training_notes()
            .times(1)
            .returning(|_, _| {
                use chrono::Utc;
                Ok(vec![TrainingNote::new(
                    TrainingNoteId::new(),
                    UserId::from("user1"),
                    None,
                    TrainingNoteContent::from("Test note"),
                    TrainingNoteDate::today(),
                    Utc::now().into(),
                )])
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_period_notes(&user_id, &period_id)
            .await;

        assert!(result.is_ok());
        let notes = result.unwrap();
        assert_eq!(notes.len(), 1);
    }

    #[tokio::test]
    async fn test_get_training_period_notes_period_not_found() {
        let user_id = UserId::from("user1");
        let period_id = TrainingPeriodId::new();

        let mut training_repository = MockTrainingRepository::new();

        // Mock get_training_period to return None
        training_repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| None);

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_period_notes(&user_id, &period_id)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_training_period_metrics_values_ok() {
        let user_id = UserId::from("user1");
        let period_id = TrainingPeriodId::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = Some(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap());

        let mut training_repository = MockTrainingRepository::new();

        // Mock get_training_period
        let period_id_clone = period_id.clone();
        training_repository
            .expect_get_training_period()
            .times(2)
            .returning(move |_, _| {
                use crate::domain::models::training::{TrainingPeriod, TrainingPeriodSports};
                Some(
                    TrainingPeriod::new(
                        period_id_clone.clone(),
                        UserId::from("user1"),
                        start,
                        end,
                        "Test Period".to_string(),
                        TrainingPeriodSports::new(None),
                        None,
                    )
                    .unwrap(),
                )
            });

        // Mock get_global_metrics and get_period_metrics for metrics values computation
        training_repository
            .expect_get_global_metrics()
            .returning(|_| Ok(vec![]));
        training_repository
            .expect_get_period_metrics()
            .returning(|_, _| Ok(vec![]));
        training_repository
            .expect_get_training_metrics_ordering()
            .returning(|_, _| Ok(TrainingMetricsOrdering::default()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_period_metrics_values(&user_id, &period_id)
            .await
            .unwrap();

        assert!(result.is_empty()); // Empty because we mocked no metrics
    }

    #[tokio::test]
    async fn test_get_training_period_metrics_values_period_not_found() {
        let user_id = UserId::from("user1");
        let period_id = TrainingPeriodId::new();

        let mut training_repository = MockTrainingRepository::new();

        // Mock get_training_period to return None
        training_repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| None);

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let err = service
            .get_training_period_metrics_values(&user_id, &period_id)
            .await
            .unwrap_err();

        let GetTrainingMetricValuesError::TrainingPeriodDoesNotExist(id) = err else {
            unreachable!("Should have err")
        };
        assert_eq!(id, period_id);
    }
}

#[cfg(test)]
mod test_training_service_metric_values {
    use chrono::NaiveDate;

    use super::*;
    use crate::domain::models::activity::{
        Activity, ActivityDuration, ActivityId, ActivityMetricV2, ActivityMetricsV2,
        ActivityStartTime, Sport,
    };
    use crate::domain::models::training::{
        TrainingMetricAggregate, TrainingMetricFilters, TrainingMetricGranularity,
    };
    use crate::domain::ports::training::GetTrainingMetricValuesError;
    use crate::domain::services::activity::test_utils::MockActivityService;
    use crate::domain::services::training::test_utils::MockTrainingRepository;
    use std::collections::HashMap;

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
            .return_once(move |_, _| Ok(None));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let req =
            GetTrainingMetricValuesRequest::ByTrainingMetricId(user_id.clone(), metric_id.clone());

        let result = service.get_training_metric_values(req, &date_range).await;

        assert!(result.is_err());
        match result {
            Err(GetTrainingMetricValuesError::TrainingMetricDoesNotExist(_)) => {}
            _ => panic!("Expected TrainingMetricDoesNotExists error"),
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
            ActivityMetricV2::Distance,
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        let mut activity_service = MockActivityService::default();
        activity_service
            .expect_list_activities_with_metrics()
            .times(1)
            .returning(|_, _, _| Ok(vec![]));

        let training_repository = MockTrainingRepository::new();
        let activity_service = activity_service;
        let service = TrainingService::new(training_repository, activity_service);

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
            ActivityMetricV2::Distance,
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            None,
        );

        // Create some test activities
        let activity = Activity::new_empty(
            ActivityId::new(),
            user_id.clone(),
            ActivityStartTime::from_timestamp(1705315200).unwrap(), // 2024-01-15T10:00:00Z
            ActivityDuration::default(),
            Sport::Running,
        );

        let mut activity_service = MockActivityService::default();
        activity_service
            .expect_list_activities_with_metrics()
            .times(1)
            .returning(move |_, _, _| {
                Ok(vec![(
                    activity.clone(),
                    ActivityMetricsV2::new(HashMap::from([(
                        ActivityMetricV2::Distance,
                        Some(10000.0),
                    )])),
                )])
            });

        let training_repository = MockTrainingRepository::new();
        let activity_service = activity_service;
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .compute_training_metric_values(&definition, &date_range)
            .await;

        assert!(result.is_ok());
        let values = result.unwrap();
        // With weekly granularity and sum aggregate, we should have one bin with the activity distance
        assert!(!values.is_empty());
    }
}

#[cfg(test)]
mod test_training_service_metrics_ordering {
    use super::*;
    use crate::domain::models::training::{
        TrainingMetricId, TrainingMetricsOrdering, TrainingPeriod, TrainingPeriodSports,
    };
    use crate::domain::ports::training::{
        GetTrainingMetricsOrderingError, SetTrainingMetricsOrderingError,
    };
    use crate::domain::services::activity::test_utils::MockActivityService;
    use crate::domain::services::training::test_utils::MockTrainingRepository;
    use anyhow::anyhow;
    use chrono::NaiveDate;

    #[tokio::test]
    async fn test_get_training_metrics_ordering_global_scope_ok() {
        let user_id = UserId::from("user1");
        let scope = TrainingMetricScope::Global;
        let ordering = TrainingMetricsOrdering::try_from(vec![
            TrainingMetricId::from("metric1"),
            TrainingMetricId::from("metric2"),
        ])
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_metrics_ordering()
            .times(1)
            .returning(move |_, _| Ok(ordering.clone()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_metrics_ordering(&user_id, &scope)
            .await;

        assert!(result.is_ok());
        let returned_ordering = result.unwrap();
        assert_eq!(
            returned_ordering,
            TrainingMetricsOrdering::try_from(vec![
                TrainingMetricId::from("metric1"),
                TrainingMetricId::from("metric2"),
            ])
            .unwrap()
        );
    }

    #[tokio::test]
    async fn test_get_training_metrics_ordering_period_scope_ok() {
        let user_id = UserId::from("user1");
        let period_id = TrainingPeriodId::from("period1");
        let scope = TrainingMetricScope::TrainingPeriod(period_id.clone());
        let ordering =
            TrainingMetricsOrdering::try_from(vec![TrainingMetricId::from("metric1")]).unwrap();

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            None,
            "Test Period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .returning(move |_, _| Some(period.clone()));
        training_repository
            .expect_get_training_metrics_ordering()
            .times(1)
            .returning(move |_, _| Ok(ordering.clone()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_metrics_ordering(&user_id, &scope)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_training_metrics_ordering_period_does_not_exist() {
        let user_id = UserId::from("user1");
        let period_id = TrainingPeriodId::from("period1");
        let scope = TrainingMetricScope::TrainingPeriod(period_id.clone());

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| None);

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_metrics_ordering(&user_id, &scope)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            GetTrainingMetricsOrderingError::TrainingPeriodDoesNotExist(id) => {
                assert_eq!(id, period_id);
            }
            _ => panic!("Expected TrainingPeriodDoesNotExist error"),
        }
    }

    #[tokio::test]
    async fn test_get_training_metrics_ordering_repository_error() {
        let user_id = UserId::from("user1");
        let scope = TrainingMetricScope::Global;

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_metrics_ordering()
            .times(1)
            .returning(|_, _| {
                Err(GetTrainingMetricsOrderingError::Unknown(anyhow!(
                    "database error"
                )))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .get_training_metrics_ordering(&user_id, &scope)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            GetTrainingMetricsOrderingError::Unknown(_) => {}
            _ => panic!("Expected Unknown error"),
        }
    }

    #[tokio::test]
    async fn test_set_training_metrics_ordering_global_scope_ok() {
        let user_id = UserId::from("user1");
        let scope = TrainingMetricScope::Global;
        let ordering = TrainingMetricsOrdering::try_from(vec![
            TrainingMetricId::from("metric1"),
            TrainingMetricId::from("metric2"),
        ])
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_set_training_metrics_ordering()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .set_training_metrics_ordering(&user_id, &scope, ordering)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_training_metrics_ordering_period_scope_ok() {
        let user_id = UserId::from("user1");
        let period_id = TrainingPeriodId::from("period1");
        let scope = TrainingMetricScope::TrainingPeriod(period_id.clone());
        let ordering =
            TrainingMetricsOrdering::try_from(vec![TrainingMetricId::from("metric1")]).unwrap();

        let period = TrainingPeriod::new(
            period_id.clone(),
            user_id.clone(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            None,
            "Test Period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .returning(move |_, _| Some(period.clone()));
        training_repository
            .expect_set_training_metrics_ordering()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .set_training_metrics_ordering(&user_id, &scope, ordering)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_training_metrics_ordering_period_does_not_exist() {
        let user_id = UserId::from("user1");
        let period_id = TrainingPeriodId::from("period1");
        let scope = TrainingMetricScope::TrainingPeriod(period_id.clone());
        let ordering =
            TrainingMetricsOrdering::try_from(vec![TrainingMetricId::from("metric1")]).unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| None);

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .set_training_metrics_ordering(&user_id, &scope, ordering)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SetTrainingMetricsOrderingError::TrainingPeriodDoesNotExist(id) => {
                assert_eq!(id, period_id);
            }
            _ => panic!("Expected TrainingPeriodDoesNotExist error"),
        }
    }

    #[tokio::test]
    async fn test_set_training_metrics_ordering_repository_error() {
        let user_id = UserId::from("user1");
        let scope = TrainingMetricScope::Global;
        let ordering =
            TrainingMetricsOrdering::try_from(vec![TrainingMetricId::from("metric1")]).unwrap();

        let mut training_repository = MockTrainingRepository::new();
        training_repository
            .expect_set_training_metrics_ordering()
            .times(1)
            .returning(|_, _, _| {
                Err(SetTrainingMetricsOrderingError::Unknown(anyhow!(
                    "database error"
                )))
            });

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(training_repository, activity_service);

        let result = service
            .set_training_metrics_ordering(&user_id, &scope, ordering)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SetTrainingMetricsOrderingError::Unknown(_) => {}
            _ => panic!("Expected Unknown error"),
        }
    }
}

#[cfg(test)]
mod test_training_service_copy_metric {
    use super::*;
    use crate::domain::models::activity::ActivityMetricV2;
    use crate::domain::models::training::{
        TrainingMetricAggregate, TrainingMetricFilters, TrainingMetricGranularity,
        TrainingMetricGroupBy,
    };
    use crate::domain::ports::training::{GetDefinitionError, SaveTrainingMetricError};
    use crate::domain::services::activity::test_utils::MockActivityService;
    use crate::domain::services::training::test_utils::MockTrainingRepository;
    use anyhow::anyhow;

    fn make_definition() -> TrainingMetricDefinition {
        TrainingMetricDefinition::new(
            UserId::test_default(),
            ActivityMetricV2::Distance,
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
            TrainingMetricFilters::empty(),
            TrainingMetricGroupBy::none(),
        )
    }

    #[tokio::test]
    async fn test_copy_training_metric_ok() {
        let source_id = TrainingMetricId::from("source");
        let period_id = TrainingPeriodId::new();
        let definition = make_definition();

        let mut repository = MockTrainingRepository::new();

        let def_clone = definition.clone();
        repository
            .expect_get_definition()
            .times(1)
            .returning(move |_, _| Ok(Some(def_clone.clone())));

        repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| {
                Some(
                    crate::domain::models::training::TrainingPeriod::new(
                        TrainingPeriodId::new(),
                        UserId::test_default(),
                        "2025-01-01".parse().unwrap(),
                        None,
                        "Test".to_string(),
                        crate::domain::models::training::TrainingPeriodSports::new(None),
                        None,
                    )
                    .unwrap(),
                )
            });

        let period_id_clone = period_id.clone();
        let def_for_assert = definition.clone();
        repository
            .expect_save_training_metric_definition()
            .times(1)
            .withf(move |metric| {
                metric.scope() == &TrainingMetricScope::TrainingPeriod(period_id_clone.clone())
                    && metric.definition() == &def_for_assert
                    && metric.name().is_none()
            })
            .returning(|_| Ok(()));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let req = CopyTrainingMetricRequest::new(UserId::test_default(), source_id, period_id);

        let result = service.copy_training_metric(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_copy_training_metric_source_metric_not_found() {
        let source_id = TrainingMetricId::from("source");
        let period_id = TrainingPeriodId::new();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definition()
            .times(1)
            .returning(|_, _| Ok(None));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let req =
            CopyTrainingMetricRequest::new(UserId::test_default(), source_id.clone(), period_id);

        let result = service.copy_training_metric(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CopyTrainingMetricError::MetricDoesNotExist(id) => assert_eq!(id, source_id),
            _ => panic!("Expected MetricDoesNotExist"),
        }
    }

    #[tokio::test]
    async fn test_copy_training_metric_get_definition_error() {
        let source_id = TrainingMetricId::from("source");
        let period_id = TrainingPeriodId::new();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definition()
            .times(1)
            .returning(|_, _| Err(GetDefinitionError::Unknown(anyhow!("db error"))));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let req = CopyTrainingMetricRequest::new(UserId::test_default(), source_id, period_id);

        let result = service.copy_training_metric(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CopyTrainingMetricError::Unknown(_) => {}
            _ => panic!("Expected Unknown error"),
        }
    }

    #[tokio::test]
    async fn test_copy_training_metric_target_period_not_found() {
        let source_id = TrainingMetricId::from("source");
        let period_id = TrainingPeriodId::new();
        let definition = make_definition();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definition()
            .times(1)
            .returning(move |_, _| Ok(Some(definition.clone())));

        repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| None);

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let req = CopyTrainingMetricRequest::new(UserId::test_default(), source_id, period_id);

        let result = service.copy_training_metric(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CopyTrainingMetricError::PeriodDoesNotExist(_) => {}
            _ => panic!("Expected PeriodDoesNotExist"),
        }
    }

    #[tokio::test]
    async fn test_copy_training_metric_save_error() {
        let source_id = TrainingMetricId::from("source");
        let period_id = TrainingPeriodId::new();
        let definition = make_definition();

        let mut repository = MockTrainingRepository::new();
        repository
            .expect_get_definition()
            .times(1)
            .returning(move |_, _| Ok(Some(definition.clone())));

        repository
            .expect_get_training_period()
            .times(1)
            .returning(|_, _| {
                Some(
                    crate::domain::models::training::TrainingPeriod::new(
                        TrainingPeriodId::new(),
                        UserId::test_default(),
                        "2025-01-01".parse().unwrap(),
                        None,
                        "Test".to_string(),
                        crate::domain::models::training::TrainingPeriodSports::new(None),
                        None,
                    )
                    .unwrap(),
                )
            });

        repository
            .expect_save_training_metric_definition()
            .times(1)
            .returning(|_| Err(SaveTrainingMetricError::Unknown(anyhow!("save error"))));

        let activity_service = MockActivityService::default();
        let service = TrainingService::new(repository, activity_service);

        let req = CopyTrainingMetricRequest::new(UserId::test_default(), source_id, period_id);

        let result = service.copy_training_metric(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CopyTrainingMetricError::SaveMetricError(_) => {}
            _ => panic!("Expected SaveMetricError"),
        }
    }
}
