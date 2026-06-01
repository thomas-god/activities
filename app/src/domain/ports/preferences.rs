use thiserror::Error;

///////////////////////////////////////////////////////////////////
/// PREFERENCES SERVICE
///////////////////////////////////////////////////////////////////
use crate::domain::models::{
    UserId,
    preferences::{Preference, PreferenceKey},
};

#[derive(Debug, Error)]
pub enum GetPreferenceError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum SetPreferenceError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum DeletePreferenceError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait IPreferencesService: Clone + Send + Sync + 'static {
    /// Get a specific preference for a user
    fn get_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> impl Future<Output = Result<Option<Preference>, GetPreferenceError>> + Send;

    /// Get all preferences for a user
    fn get_all_preferences(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Preference>, GetPreferenceError>> + Send;

    /// Set (add or modify) a preference for a user
    fn set_preference(
        &self,
        user: &UserId,
        preference: Preference,
    ) -> impl Future<Output = Result<(), SetPreferenceError>> + Send;

    /// Delete a specific preference for a user
    fn delete_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> impl Future<Output = Result<(), DeletePreferenceError>> + Send;
}

///////////////////////////////////////////////////////////////////
/// PREFERENCES REPOSITORY
///////////////////////////////////////////////////////////////////

#[derive(Debug, Error)]
pub enum SavePreferenceError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait PreferencesRepository: Clone + Send + Sync + 'static {
    fn get_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> impl Future<Output = Result<Option<Preference>, anyhow::Error>> + Send;

    fn get_all_preferences(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Preference>, anyhow::Error>> + Send;

    fn save_preference(
        &self,
        user: &UserId,
        preference: &Preference,
    ) -> impl Future<Output = Result<(), SavePreferenceError>> + Send;

    fn delete_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;
}
