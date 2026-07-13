use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::domain::models::{
    activity::ActivityMetricV2,
    training::{TrainingMetricId, TrainingMetricScope},
};

///////////////////////////////////////////////////////////////////
/// PREFERENCE ENUM AND KEY
///////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreferenceKey {
    FavoriteMetric,
    ActivityListSummary,
}

impl std::fmt::Display for PreferenceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreferenceKey::FavoriteMetric => write!(f, "favorite_metric"),
            PreferenceKey::ActivityListSummary => write!(f, "activity-list-summary"),
        }
    }
}

impl std::str::FromStr for PreferenceKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "favorite_metric" => Ok(PreferenceKey::FavoriteMetric),
            "activity-list-summary" => Ok(PreferenceKey::ActivityListSummary),
            _ => Err(format!("Unknown preference key: {}", s)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Constructor)]
pub struct ActivityListSummary {
    scope: TrainingMetricScope,
    items: Vec<ActivityListSummaryItem>,
}

impl ActivityListSummary {
    pub fn scope(&self) -> &TrainingMetricScope {
        &self.scope
    }

    pub fn items(&self) -> &[ActivityListSummaryItem] {
        &self.items
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActivityListSummaryItem {
    Metric(ActivityMetricV2),
    RPE,
    WorkoutType,
}

/// Represents a single preference value with its associated data
#[derive(Clone, Debug, PartialEq)]
pub enum Preference {
    // TODO: remove as it's no longer used
    FavoriteMetric(TrainingMetricId),
    ActivityListSummary(ActivityListSummary),
}

impl Preference {
    /// Returns the preference key for this preference
    pub fn key(&self) -> PreferenceKey {
        match self {
            Preference::FavoriteMetric(_) => PreferenceKey::FavoriteMetric,
            Preference::ActivityListSummary(_) => PreferenceKey::ActivityListSummary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preference_key_display_and_parse() {
        assert_eq!(PreferenceKey::FavoriteMetric.to_string(), "favorite_metric");

        assert_eq!(
            "favorite_metric".parse::<PreferenceKey>().unwrap(),
            PreferenceKey::FavoriteMetric
        );
        assert!("unknown".parse::<PreferenceKey>().is_err());
    }
}
