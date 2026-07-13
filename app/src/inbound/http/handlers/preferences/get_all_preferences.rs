use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::domain::ports::{
    activity::IActivityService,
    preferences::{GetPreferenceError, IPreferencesService},
    training::ITrainingService,
};
use crate::inbound::{auth::AuthenticatedUser, http::AppState, parser::ParseFile};

use super::types::PreferencePayload;

pub async fn get_all_preferences<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
) -> Result<Json<Vec<PreferencePayload>>, StatusCode> {
    let preferences = match state
        .preferences_service
        .get_all_preferences(user.user())
        .await
    {
        Ok(preferences) => preferences,
        Err(err) => {
            if matches!(&err, GetPreferenceError::Unknown(_)) {
                tracing::error!("Error getting all preferences: {}", err.to_string());
            }
            return Err(StatusCode::from(err));
        }
    };

    let mut items = Vec::new();
    for preference in preferences {
        match PreferencePayload::try_from(preference) {
            Ok(item) => items.push(item),
            Err(err) => tracing::error!("Error while serializing preference: {}", err),
        }
    }

    Ok(Json(items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::{
        domain::{
            models::{
                UserId,
                activity::ActivityMetricV2,
                preferences::{ActivityListSummary, ActivityListSummaryItem, Preference},
                training::{TrainingMetricId, TrainingMetricScope},
            },
            services::{
                activity::test_utils::MockActivityService,
                preferences::tests_utils::MockPreferencesService,
                training::test_utils::MockTrainingService,
            },
        },
        inbound::parser::test_utils::MockFileParser,
    };
    use mockall::predicate::*;

    fn create_test_state(
        preferences_service: MockPreferencesService,
    ) -> AppState<MockActivityService, MockFileParser, MockTrainingService, MockPreferencesService>
    {
        AppState {
            activity_service: Arc::new(MockActivityService::test_default()),
            file_parser: Arc::new(MockFileParser::test_default()),
            training_metrics_service: Arc::new(MockTrainingService::test_default()),
            preferences_service: Arc::new(preferences_service),
        }
    }

    #[tokio::test]
    async fn test_get_all_preferences_success() {
        let user_id = UserId::from("test_user");
        let user_id_clone = user_id.clone();

        let preferences = vec![
            Preference::FavoriteMetric(TrainingMetricId::from("test_metric_1")),
            Preference::ActivityListSummary(ActivityListSummary::new(
                TrainingMetricScope::Global,
                vec![
                    ActivityListSummaryItem::Metric(ActivityMetricV2::Distance),
                    ActivityListSummaryItem::RPE,
                ],
            )),
        ];
        let preferences_clone = preferences.clone();

        let mut preferences_service = MockPreferencesService::new();
        preferences_service
            .expect_get_all_preferences()
            .with(function(move |user: &UserId| user == &user_id_clone))
            .times(1)
            .returning(move |_| Ok(preferences_clone.clone()));

        let state = create_test_state(preferences_service);

        let user = AuthenticatedUser::new(user_id);

        let result = get_all_preferences(Extension(user), State(state)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.len(), 2);
    }

    #[tokio::test]
    async fn test_get_all_preferences_empty() {
        let user_id = UserId::from("test_user");
        let user_id_clone = user_id.clone();

        let mut preferences_service = MockPreferencesService::new();
        preferences_service
            .expect_get_all_preferences()
            .with(function(move |user: &UserId| user == &user_id_clone))
            .times(1)
            .returning(|_| Ok(vec![]));

        let state = create_test_state(preferences_service);

        let user = AuthenticatedUser::new(user_id);

        let result = get_all_preferences(Extension(user), State(state)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.len(), 0);
    }

    #[tokio::test]
    async fn test_get_all_preferences_service_error() {
        let user_id = UserId::from("test_user");
        let user_id_clone = user_id.clone();

        let mut preferences_service = MockPreferencesService::new();
        preferences_service
            .expect_get_all_preferences()
            .with(function(move |user: &UserId| user == &user_id_clone))
            .times(1)
            .returning(|_| {
                Err(GetPreferenceError::Unknown(anyhow::anyhow!(
                    "Database connection failed"
                )))
            });

        let state = create_test_state(preferences_service);

        let user = AuthenticatedUser::new(user_id);

        let result = get_all_preferences(Extension(user), State(state)).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_get_all_preferences_multiple_types() {
        let user_id = UserId::from("test_user");
        let user_id_clone = user_id.clone();

        let preferences = vec![
            Preference::FavoriteMetric(TrainingMetricId::from("metric_1")),
            Preference::FavoriteMetric(TrainingMetricId::from("metric_2")),
            Preference::ActivityListSummary(ActivityListSummary::new(
                TrainingMetricScope::Global,
                vec![ActivityListSummaryItem::WorkoutType],
            )),
        ];
        let preferences_clone = preferences.clone();

        let mut preferences_service = MockPreferencesService::new();
        preferences_service
            .expect_get_all_preferences()
            .with(function(move |user: &UserId| user == &user_id_clone))
            .times(1)
            .returning(move |_| Ok(preferences_clone.clone()));

        let state = create_test_state(preferences_service);

        let user = AuthenticatedUser::new(user_id);

        let result = get_all_preferences(Extension(user), State(state)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        // All 3 preferences should be serialized successfully
        assert_eq!(response.len(), 3);
    }
}
