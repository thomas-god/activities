use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::domain::ports::{
    activity::IActivityService,
    preferences::{GetPreferenceError, IPreferencesService},
    training::ITrainingService,
};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::models::preferences::PreferenceKey,
    inbound::{auth::AuthenticatedUser, http::AppState},
};

use super::types::PreferencePayload;

#[tracing::instrument(skip_all, err)]
pub async fn get_preference<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(key): Path<String>,
) -> Result<Json<Option<PreferencePayload>>, StatusCode> {
    let preference_key = key
        .parse::<PreferenceKey>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let pref = match state
        .preferences_service
        .get_preference(user.user(), &preference_key)
        .await
    {
        Ok(pref) => pref,
        Err(err) => {
            if matches!(&err, GetPreferenceError::Unknown(_)) {
                tracing::error!("Error getting preference: {}", err.to_string());
            }
            return Err(StatusCode::from(err));
        }
    };

    match pref.map(PreferencePayload::try_from).transpose() {
        Ok(res) => Ok(Json(res)),
        Err(err) => {
            tracing::error!("Error while serializing preference: {}", err.to_string());
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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
    async fn test_get_preference_success_activity_list_summary() {
        let user_id = UserId::from("test_user");
        let user_id_clone = user_id.clone();
        let summary = ActivityListSummary::new(
            TrainingMetricScope::Global,
            vec![
                ActivityListSummaryItem::Metric(ActivityMetricV2::Distance),
                ActivityListSummaryItem::RPE,
            ],
        );
        let preference = Preference::ActivityListSummary(summary);

        let mut preferences_service = MockPreferencesService::new();
        preferences_service
            .expect_get_preference()
            .withf(move |user: &UserId, key: &PreferenceKey| {
                user == &user_id_clone && key == &PreferenceKey::ActivityListSummary
            })
            .times(1)
            .returning(move |_, _| Ok(Some(preference.clone())));

        let state = create_test_state(preferences_service);
        let user = AuthenticatedUser::new(user_id);

        let result = get_preference(
            Extension(user),
            State(state),
            Path("activity-list-summary".to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.is_some());
    }

    #[tokio::test]
    async fn test_get_preference_not_found() {
        let user_id = UserId::from("test_user");
        let user_id_clone = user_id.clone();

        let mut preferences_service = MockPreferencesService::new();
        preferences_service
            .expect_get_preference()
            .withf(move |user: &UserId, key: &PreferenceKey| {
                user == &user_id_clone && key == &PreferenceKey::ActivityListSummary
            })
            .times(1)
            .returning(|_, _| Ok(None));

        let state = create_test_state(preferences_service);
        let user = AuthenticatedUser::new(user_id);

        let result = get_preference(
            Extension(user),
            State(state),
            Path("activity-list-summary".to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.is_none());
    }

    #[tokio::test]
    async fn test_get_preference_invalid_key() {
        let user_id = UserId::from("test_user");

        let preferences_service = MockPreferencesService::new();
        let state = create_test_state(preferences_service);
        let user = AuthenticatedUser::new(user_id);

        let result = get_preference(
            Extension(user),
            State(state),
            Path("invalid_key".to_string()),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_preference_service_error() {
        let user_id = UserId::from("test_user");
        let user_id_clone = user_id.clone();

        let mut preferences_service = MockPreferencesService::new();
        preferences_service
            .expect_get_preference()
            .withf(move |user: &UserId, key: &PreferenceKey| {
                user == &user_id_clone && key == &PreferenceKey::ActivityListSummary
            })
            .times(1)
            .returning(|_, _| {
                Err(GetPreferenceError::Unknown(anyhow::anyhow!(
                    "Database connection failed"
                )))
            });

        let state = create_test_state(preferences_service);
        let user = AuthenticatedUser::new(user_id);

        let result = get_preference(
            Extension(user),
            State(state),
            Path("activity-list-summary".to_string()),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
