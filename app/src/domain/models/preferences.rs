use crate::domain::models::training::TrainingMetricId;

///////////////////////////////////////////////////////////////////
/// PREFERENCE ENUM AND KEY
///////////////////////////////////////////////////////////////////

/// Identifies the type of a preference
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreferenceKey {
    FavoriteMetric,
}

impl std::fmt::Display for PreferenceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreferenceKey::FavoriteMetric => write!(f, "favorite_metric"),
        }
    }
}

impl std::str::FromStr for PreferenceKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "favorite_metric" => Ok(PreferenceKey::FavoriteMetric),
            _ => Err(format!("Unknown preference key: {}", s)),
        }
    }
}

/// Represents a single preference value with its associated data
#[derive(Clone, Debug, PartialEq)]
pub enum Preference {
    FavoriteMetric(TrainingMetricId),
}

impl Preference {
    /// Returns the preference key for this preference
    pub fn key(&self) -> PreferenceKey {
        match self {
            Preference::FavoriteMetric(_) => PreferenceKey::FavoriteMetric,
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
