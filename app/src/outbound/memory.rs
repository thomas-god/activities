// In memory implementations of repository ports

use std::{collections::HashMap, ops::DerefMut, sync::Arc};

use tokio::sync::Mutex;

use crate::domain::{
    models::{
        UserId,
        activity::{Activity, ActivityId, ActivityNaturalKey},
        training_metrics::{TrainingMetricDefinition, TrainingMetricId, TrainingMetricValues},
    },
    ports::{
        ActivityRepository, GetActivityError, GetTrainingMetricValueError,
        GetTrainingMetricsDefinitionsError, ListActivitiesError, RawDataRepository,
        SaveActivityError, SaveRawDataError, SaveTrainingMetricError, SimilarActivityError,
        TrainingMetricsRepository, UpdateMetricError,
    },
};

#[derive(Clone)]
pub struct InMemoryActivityRepository {
    activities: Arc<Mutex<Vec<Activity>>>,
}

impl InMemoryActivityRepository {
    pub fn new(activities: Vec<Activity>) -> Self {
        Self {
            activities: Arc::new(Mutex::new(activities)),
        }
    }
}

impl ActivityRepository for InMemoryActivityRepository {
    async fn similar_activity_exists(
        &self,
        natural_key: &ActivityNaturalKey,
    ) -> Result<bool, SimilarActivityError> {
        let guard = self.activities.lock().await;
        Ok(guard
            .iter()
            .any(|activity| activity.natural_key() == *natural_key))
    }

    async fn save_activity(&self, activity: &Activity) -> Result<(), SaveActivityError> {
        let mut guard = self.activities.lock().await;
        guard.deref_mut().push(activity.clone());
        Ok(())
    }

    async fn list_activities(&self, user: &UserId) -> Result<Vec<Activity>, ListActivitiesError> {
        let activities = self.activities.lock().await;
        Ok(activities
            .iter()
            .filter(|activity| activity.user() == user)
            .cloned()
            .collect())
    }

    async fn get_activity(&self, id: &ActivityId) -> Result<Option<Activity>, GetActivityError> {
        let activities = self.activities.lock().await;
        Ok(activities
            .iter()
            .find(|activity| activity.id() == id)
            .cloned())
    }
}

#[derive(Clone)]
pub struct InMemoryRawDataRepository {
    raw_data: Arc<Mutex<HashMap<ActivityId, Vec<u8>>>>,
}

impl InMemoryRawDataRepository {
    pub fn new(raw_data: HashMap<ActivityId, Vec<u8>>) -> Self {
        Self {
            raw_data: Arc::new(Mutex::new(raw_data)),
        }
    }
}

impl RawDataRepository for InMemoryRawDataRepository {
    async fn save_raw_data(
        &self,
        activity_id: &ActivityId,
        content: &[u8],
    ) -> Result<(), SaveRawDataError> {
        let mut guard = self.raw_data.lock().await;
        guard
            .deref_mut()
            .insert(activity_id.clone(), content.into());
        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemoryTrainingMetricsRepository {
    definitions: Arc<Mutex<HashMap<TrainingMetricId, TrainingMetricDefinition>>>,
    values: Arc<Mutex<HashMap<TrainingMetricId, TrainingMetricValues>>>,
}

impl InMemoryTrainingMetricsRepository {
    pub fn new(definitions: HashMap<TrainingMetricId, TrainingMetricDefinition>) -> Self {
        Self {
            definitions: Arc::new(Mutex::new(definitions)),
            values: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl TrainingMetricsRepository for InMemoryTrainingMetricsRepository {
    async fn save_definitions(
        &self,
        definition: TrainingMetricDefinition,
    ) -> Result<(), SaveTrainingMetricError> {
        let mut definitons = self.definitions.lock().await;
        definitons.insert(definition.id().clone(), definition);
        Ok(())
    }

    async fn get_definitions(
        &self,
        user: &UserId,
    ) -> Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError> {
        let definitions = self.definitions.lock().await;
        Ok(definitions
            .values()
            .filter(|metric| metric.user() == user)
            .cloned()
            .collect())
    }

    async fn get_metric_values(
        &self,
        id: &TrainingMetricId,
    ) -> Result<TrainingMetricValues, GetTrainingMetricValueError> {
        let metrics = self.values.lock().await;
        metrics
            .get(id)
            .ok_or(GetTrainingMetricValueError::TrainingMetricDoesNotExists(
                id.clone(),
            ))
            .cloned()
    }

    async fn save_metric_values(
        &self,
        id: &TrainingMetricId,
        new_value: (&str, f64),
    ) -> Result<(), UpdateMetricError> {
        let mut metrics = self.values.lock().await;
        let (key, value) = new_value;
        metrics
            .entry(id.clone())
            .or_default()
            .insert(key.to_string(), value);
        Ok(())
    }
}

#[cfg(test)]
mod tests_activities_repository {

    use crate::domain::models::activity::{
        ActivityDuration, ActivityStartTime, ActivityStatistics, ActivityTimeseries, Sport,
        TimeseriesTime,
    };

    use super::*;

    fn default_activity(user: Option<String>) -> Activity {
        Activity::new(
            ActivityId::default(),
            user.unwrap_or_default().into(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration::new(10),
            Sport::Cycling,
            ActivityStatistics::new(HashMap::new()),
            ActivityTimeseries::new(TimeseriesTime::new(vec![]), vec![]),
        )
    }

    #[tokio::test]
    async fn test_list_only_activities_belonging_to_the_requested_user() {
        let activities = vec![
            default_activity(Some("user1".to_string())),
            default_activity(Some("user2".to_string())),
        ];

        let repository = InMemoryActivityRepository::new(activities);

        let res = repository
            .list_activities(&"user1".to_string().into())
            .await;

        let res = res.expect("Res should be Ok");

        assert_eq!(res.len(), 1);
        assert!(
            res.iter()
                .all(|activity| activity.user() == &"user1".to_string().into())
        )
    }
}

#[cfg(test)]
mod tests_training_metrics_repository {

    use crate::domain::models::{
        activity::ActivityStatistic,
        training_metrics::{
            TrainingMetricAggregate, TrainingMetricGranularity, TrainingMetricSource,
        },
    };

    use super::*;

    fn default_metric(user: Option<String>) -> (TrainingMetricId, TrainingMetricDefinition) {
        let id = TrainingMetricId::default();
        (
            id.clone(),
            TrainingMetricDefinition::new(
                id.clone(),
                user.unwrap_or_default().into(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Max,
            ),
        )
    }

    #[tokio::test]
    async fn test_list_only_metrics_belonging_to_the_requested_user() {
        let metrics = HashMap::from_iter(vec![
            default_metric(Some("user1".to_string())),
            default_metric(Some("user2".to_string())),
        ]);

        let repository = InMemoryTrainingMetricsRepository::new(metrics);

        let res = repository
            .get_definitions(&"user1".to_string().into())
            .await;

        let res = res.expect("Res should be Ok");

        assert_eq!(res.len(), 1);
        assert!(
            res.iter()
                .all(|metric| metric.user() == &"user1".to_string().into())
        );
    }
}
