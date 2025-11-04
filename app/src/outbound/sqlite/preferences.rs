use std::str::FromStr;

use anyhow::anyhow;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::{
    domain::{
        models::{
            UserId,
            preferences::{Preference, PreferenceKey},
        },
        ports::{PreferencesRepository, SavePreferenceError},
    },
    outbound::sqlite::types::{deserialize_preference, serialize_preference_value},
};

type PreferenceRow = (UserId, PreferenceKey, String);
type RawPreferenceRow = (UserId, String, String);

#[derive(Debug, Clone)]
pub struct SqlitePreferencesRepository {
    pool: SqlitePool,
}

impl SqlitePreferencesRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migrations
        sqlx::migrate!("migrations/preferences").run(&pool).await?;

        Ok(Self { pool })
    }
}

impl PreferencesRepository for SqlitePreferencesRepository {
    async fn get_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> Result<Option<Preference>, anyhow::Error> {
        match sqlx::query_as::<_, PreferenceRow>(
            "SELECT user_id, preference_key, preference_value
             FROM t_user_preferences
             WHERE user_id = ?1 AND preference_key = ?2
             LIMIT 1;",
        )
        .bind(user)
        .bind(key)
        .fetch_one(&self.pool)
        .await
        {
            Ok((_, key, value)) => {
                let preference =
                    deserialize_preference(&key, &value).map_err(|e| anyhow!("{}", e))?;
                Ok(Some(preference))
            }
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(anyhow!(err)),
        }
    }

    async fn get_all_preferences(&self, user: &UserId) -> Result<Vec<Preference>, anyhow::Error> {
        // Query as raw strings to handle unknown keys gracefully
        let rows = sqlx::query_as::<_, RawPreferenceRow>(
            "SELECT user_id, preference_key, preference_value
             FROM t_user_preferences
             WHERE user_id = ?1;",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await?;

        let mut preferences = Vec::new();
        for (_, key_str, value) in rows {
            // Try to parse the key string as a known PreferenceKey
            if let Ok(key) = key_str.parse::<PreferenceKey>() {
                // If successful, deserialize the value
                if let Ok(preference) =
                    deserialize_preference(&key, &value).map_err(|e| anyhow!("{}", e))
                {
                    preferences.push(preference);
                }
            }
            // Silently ignore unknown keys and parse errors for forward compatibility
        }

        Ok(preferences)
    }

    async fn save_preference(
        &self,
        user: &UserId,
        preference: &Preference,
    ) -> Result<(), SavePreferenceError> {
        let key = preference.key();
        let value = serialize_preference_value(preference)
            .map_err(|e| SavePreferenceError::Unknown(anyhow!(e)))?;

        sqlx::query(
            "INSERT INTO t_user_preferences (user_id, preference_key, preference_value)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(user_id, preference_key) DO UPDATE SET
                preference_value = excluded.preference_value;",
        )
        .bind(user)
        .bind(&key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|err| SavePreferenceError::Unknown(anyhow!(err)))?;

        Ok(())
    }

    async fn delete_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            "DELETE FROM t_user_preferences
             WHERE user_id = ?1 AND preference_key = ?2;",
        )
        .bind(user)
        .bind(key)
        .execute(&self.pool)
        .await
        .map_err(|err| anyhow!(err))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::models::training::TrainingMetricId;

    use super::*;

    async fn create_test_repo() -> SqlitePreferencesRepository {
        SqlitePreferencesRepository::new("sqlite::memory:")
            .await
            .expect("Failed to create test repository")
    }

    #[tokio::test]
    async fn test_get_preference_returns_none_when_not_exists() {
        let repo = create_test_repo().await;
        let user = UserId::test_default();

        let result = repo
            .get_preference(&user, &PreferenceKey::FavoriteMetric)
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_save_and_get_preference() {
        let repo = create_test_repo().await;
        let user = UserId::test_default();
        let preference = Preference::FavoriteMetric(TrainingMetricId::from("test_metric_id"));

        // Save preference
        repo.save_preference(&user, &preference).await.unwrap();

        // Get preference
        let result = repo
            .get_preference(&user, &PreferenceKey::FavoriteMetric)
            .await
            .unwrap()
            .expect("Preference should exist");

        match result {
            Preference::FavoriteMetric(id) => {
                assert_eq!(id, TrainingMetricId::from("test_metric_id"))
            }
            #[allow(unreachable_patterns, reason = "Future proof for future preferences")]
            _ => panic!("Expected UnitSystem preference"),
        }
    }

    #[tokio::test]
    async fn test_save_preference_updates_existing() {
        let repo = create_test_repo().await;
        let user = UserId::test_default();

        // Save initial preference
        repo.save_preference(
            &user,
            &Preference::FavoriteMetric(TrainingMetricId::from("test_metric_id")),
        )
        .await
        .unwrap();

        // Update preference
        repo.save_preference(
            &user,
            &Preference::FavoriteMetric(TrainingMetricId::from("another_metric_id")),
        )
        .await
        .unwrap();

        // Verify updated preference
        let result = repo
            .get_preference(&user, &PreferenceKey::FavoriteMetric)
            .await
            .unwrap()
            .unwrap();

        match result {
            Preference::FavoriteMetric(id) => {
                assert_eq!(id, TrainingMetricId::from("another_metric_id"))
            }
            #[allow(unreachable_patterns, reason = "Future proof for future preferences")]
            _ => panic!("Expected UnitSystem preference"),
        }
    }

    #[tokio::test]
    async fn test_get_all_preferences() {
        let repo = create_test_repo().await;
        let user = UserId::test_default();

        // Save multiple preferences
        repo.save_preference(
            &user,
            &Preference::FavoriteMetric(TrainingMetricId::from("another_metric_id")),
        )
        .await
        .unwrap();

        // Get all preferences
        let result = repo.get_all_preferences(&user).await.unwrap();

        assert_eq!(
            result,
            vec![Preference::FavoriteMetric(TrainingMetricId::from(
                "another_metric_id"
            ))]
        );
    }

    #[tokio::test]
    async fn test_delete_preference() {
        let repo = create_test_repo().await;
        let user = UserId::test_default();

        // Save preference
        repo.save_preference(
            &user,
            &Preference::FavoriteMetric(TrainingMetricId::from("another_metric_id")),
        )
        .await
        .unwrap();

        // Verify it exists
        assert!(
            repo.get_preference(&user, &PreferenceKey::FavoriteMetric)
                .await
                .unwrap()
                .is_some()
        );

        // Delete preference
        repo.delete_preference(&user, &PreferenceKey::FavoriteMetric)
            .await
            .unwrap();

        // Verify it's gone
        assert!(
            repo.get_preference(&user, &PreferenceKey::FavoriteMetric)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_gracefully_ignores_unknown_preference_keys() {
        let repo = create_test_repo().await;
        let user = UserId::test_default();

        // Save a known preference through the API
        repo.save_preference(
            &user,
            &Preference::FavoriteMetric(TrainingMetricId::from("another_metric_id")),
        )
        .await
        .unwrap();

        // Simulate a future version adding a new preference key by inserting it as raw SQL
        sqlx::query(
            "INSERT INTO t_user_preferences (user_id, preference_key, preference_value) VALUES (?1, ?2, ?3);",
        )
        .bind(&user)
        .bind("unknown_future_preference")
        .bind("some_value")
        .execute(&repo.pool)
        .await
        .unwrap();

        // get_all_preferences should gracefully ignore the unknown key
        let result = repo.get_all_preferences(&user).await;
        assert!(result.is_ok());

        let prefs = result.unwrap();
        // Should only return the known preference
        assert_eq!(
            prefs,
            vec![Preference::FavoriteMetric(TrainingMetricId::from(
                "another_metric_id"
            ))]
        );
    }
}
