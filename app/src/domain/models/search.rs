use derive_more::Constructor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchDocumentEvent {
    Updated,
    Deleted,
}

impl TryFrom<&str> for SearchDocumentEvent {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "updated" => Ok(Self::Updated),
            "deleted" => Ok(Self::Deleted),
            _ => Err("Invalid enum variant".to_string()),
        }
    }
}

impl std::fmt::Display for SearchDocumentEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Updated => f.write_str("updated"),
            Self::Deleted => f.write_str("deleted"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchDocumentType {
    Activity,
    TrainingNote,
}
impl TryFrom<&str> for SearchDocumentType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "activity" => Ok(Self::Activity),
            "training_note" => Ok(Self::TrainingNote),
            _ => Err("Invalid enum variant".to_string()),
        }
    }
}

impl std::fmt::Display for SearchDocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Activity => f.write_str("activity"),
            Self::TrainingNote => f.write_str("training_note"),
        }
    }
}

/// Shared struct to a represent a domain-agnostic document to be indexed for search.
#[derive(Debug, Clone, Constructor)]
pub struct SearchDocument {
    document_type: SearchDocumentType, // Activity, Training note
    document_id: String,
    event: SearchDocumentEvent,
    content: String,
    occurred_at: chrono::DateTime<chrono::Utc>,
}

impl SearchDocument {
    pub fn document_type(&self) -> &SearchDocumentType {
        &self.document_type
    }
    pub fn document_id(&self) -> &str {
        &self.document_id
    }
    pub fn event(&self) -> &SearchDocumentEvent {
        &self.event
    }
    pub fn content(&self) -> &str {
        &self.content
    }
    pub fn occurred_at(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.occurred_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL_STRINGS: [&str; 2] = ["updated", "deleted"];

    fn variants() -> [SearchDocumentEvent; 2] {
        [SearchDocumentEvent::Updated, SearchDocumentEvent::Deleted]
    }

    #[test]
    fn event_to_string_parses_back_to_same_variant() {
        for variant in variants() {
            let serialized = variant.to_string();
            let parsed = SearchDocumentEvent::try_from(serialized.as_str())
                .expect("Display output must be a valid serialization");
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn canonical_strings_display_back_to_themselves() {
        for canonical in CANONICAL_STRINGS {
            let variant = SearchDocumentEvent::try_from(canonical)
                .expect("canonical string must parse to a variant");
            assert_eq!(variant.to_string(), canonical);
        }
    }

    #[test]
    fn rejects_non_canonical_strings() {
        for invalid in ["", "UPDATE", "udpated", "event:updated"] {
            assert!(
                SearchDocumentEvent::try_from(invalid).is_err(),
                "expected {invalid:?} to be rejected"
            );
        }
    }
}
