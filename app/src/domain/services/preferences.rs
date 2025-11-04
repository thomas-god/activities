use crate::domain::{
    models::{
        UserId,
        preferences::{Preference, PreferenceKey},
    },
    ports::{
        DeletePreferenceError, GetPreferenceError, IPreferencesService, PreferencesRepository,
        SetPreferenceError,
    },
};

#[derive(Debug, Clone)]
pub struct PreferencesService<PR>
where
    PR: PreferencesRepository,
{
    preferences_repository: PR,
}

impl<PR> PreferencesService<PR>
where
    PR: PreferencesRepository,
{
    pub fn new(preferences_repository: PR) -> Self {
        Self {
            preferences_repository,
        }
    }
}

impl<PR> IPreferencesService for PreferencesService<PR>
where
    PR: PreferencesRepository,
{
    async fn get_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> Result<Option<Preference>, GetPreferenceError> {
        Ok(self
            .preferences_repository
            .get_preference(user, key)
            .await?)
    }

    async fn get_all_preferences(
        &self,
        user: &UserId,
    ) -> Result<Vec<Preference>, GetPreferenceError> {
        Ok(self
            .preferences_repository
            .get_all_preferences(user)
            .await?)
    }

    async fn set_preference(
        &self,
        user: &UserId,
        preference: Preference,
    ) -> Result<(), SetPreferenceError> {
        self.preferences_repository
            .save_preference(user, &preference)
            .await
            .map_err(|e| SetPreferenceError::Unknown(e.into()))?;
        Ok(())
    }

    async fn delete_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> Result<(), DeletePreferenceError> {
        self.preferences_repository
            .delete_preference(user, key)
            .await
            .map_err(|e| DeletePreferenceError::Unknown(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;

    use super::*;
    use crate::domain::{models::training::TrainingMetricId, ports::SavePreferenceError};

    mockall::mock! {
        PreferencesRepository {}

        impl Clone for PreferencesRepository {
            fn clone(&self) -> Self;
        }

        impl PreferencesRepository for PreferencesRepository {
            async fn get_preference(
                &self,
                user: &UserId,
                key: &PreferenceKey,
            ) -> Result<Option<Preference>, anyhow::Error>;

            async fn get_all_preferences(
                &self,
                user: &UserId,
            ) -> Result<Vec<Preference>, anyhow::Error>;

            async fn save_preference(
                &self,
                user: &UserId,
                preference: &Preference,
            ) -> Result<(), SavePreferenceError>;

            async fn delete_preference(
                &self,
                user: &UserId,
                key: &PreferenceKey,
            ) -> Result<(), anyhow::Error>;
        }
    }

    #[tokio::test]
    async fn test_get_preference_returns_none_when_not_exists() {
        let user = UserId::test_default();
        let mut mock_repo = MockPreferencesRepository::new();

        mock_repo
            .expect_get_preference()
            .with(eq(user.clone()), eq(PreferenceKey::FavoriteMetric))
            .times(1)
            .returning(|_, _| Ok(None));

        let service = PreferencesService::new(mock_repo);
        let result = service
            .get_preference(&user, &PreferenceKey::FavoriteMetric)
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_preference_returns_existing() {
        let user = UserId::test_default();
        let pref = Preference::FavoriteMetric(TrainingMetricId::from("test_metric_id"));
        let mut mock_repo = MockPreferencesRepository::new();

        mock_repo
            .expect_get_preference()
            .with(eq(user.clone()), eq(PreferenceKey::FavoriteMetric))
            .times(1)
            .returning(move |_, _| Ok(Some(pref.clone())));

        let service = PreferencesService::new(mock_repo);
        let result = service
            .get_preference(&user, &PreferenceKey::FavoriteMetric)
            .await;

        assert!(result.is_ok());
        let preference = result.unwrap().unwrap();
        match preference {
            Preference::FavoriteMetric(id) => {
                assert_eq!(id, TrainingMetricId::from("test_metric_id"))
            }
            #[allow(unreachable_patterns, reason = "Future proof for future preferences")]
            _ => panic!("Expected UnitSystem preference"),
        }
    }

    #[tokio::test]
    async fn test_get_all_preferences() {
        let user = UserId::test_default();
        let prefs = vec![Preference::FavoriteMetric(TrainingMetricId::from(
            "test_metric_id",
        ))];

        let mut mock_repo = MockPreferencesRepository::new();
        mock_repo
            .expect_get_all_preferences()
            .with(eq(user.clone()))
            .times(1)
            .returning(move |_| Ok(prefs.clone()));

        let service = PreferencesService::new(mock_repo);
        let result = service.get_all_preferences(&user).await;

        assert!(result.is_ok());
        let preferences = result.unwrap();
        assert_eq!(
            preferences,
            vec![Preference::FavoriteMetric(TrainingMetricId::from(
                "test_metric_id",
            ))]
        );
    }

    #[tokio::test]
    async fn test_set_preference() {
        let user = UserId::test_default();
        let pref = Preference::FavoriteMetric(TrainingMetricId::from("test_metric_id"));

        let mut mock_repo = MockPreferencesRepository::new();
        mock_repo
            .expect_save_preference()
            .times(1)
            .returning(|_, _| Ok(()));

        let service = PreferencesService::new(mock_repo);
        let result = service.set_preference(&user, pref).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_preference() {
        let user = UserId::test_default();

        let mut mock_repo = MockPreferencesRepository::new();
        mock_repo
            .expect_delete_preference()
            .with(eq(user.clone()), eq(PreferenceKey::FavoriteMetric))
            .times(1)
            .returning(|_, _| Ok(()));

        let service = PreferencesService::new(mock_repo);
        let result = service
            .delete_preference(&user, &PreferenceKey::FavoriteMetric)
            .await;

        assert!(result.is_ok());
    }
}
