use derive_more::Constructor;
use unicode_categories::UnicodeCategories;
use unicode_normalization::UnicodeNormalization;

use crate::domain::models::UserId;

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
    user: UserId,
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
    pub fn user(&self) -> &UserId {
        &self.user
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

pub fn normalize_for_search(input: &str) -> String {
    input
        .trim()
        .nfd()
        .filter(|c| !c.is_mark_nonspacing())
        .collect()
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

    #[test]
    fn empty_and_whitespace_only_input_normalize_to_empty() {
        assert_eq!(normalize_for_search(""), "");
        assert_eq!(normalize_for_search("   "), "");
        assert_eq!(normalize_for_search("\t\r\n "), "");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize_for_search("  Hello  "), "Hello");
        assert_eq!(normalize_for_search("\n\t World \r\n"), "World");
    }

    #[test]
    fn preserves_inner_whitespace() {
        assert_eq!(normalize_for_search("  a   b  "), "a   b");
    }

    #[test]
    fn strips_diacritics_from_precomposed_input() {
        assert_eq!(normalize_for_search("Café"), "Cafe");
        assert_eq!(normalize_for_search("Crème brûlée"), "Creme brulee");
        assert_eq!(normalize_for_search("Déjà vu"), "Deja vu");
    }

    #[test]
    fn strips_diacritics_from_already_decomposed_input() {
        // "e\u{301}" (e + combining acute) is the NFD form of "é"; both must normalize the same.
        assert_eq!(normalize_for_search("e\u{0301}"), "e");
        assert_eq!(normalize_for_search("e\u{0301}"), normalize_for_search("é"));
        // "ế" decomposes as e + combining circumflex + combining acute.
        assert_eq!(normalize_for_search("ế"), "e");
        // "Å" decomposes as A + combining ring above.
        assert_eq!(normalize_for_search("A\u{030A}"), "A");
        assert_eq!(normalize_for_search("Ångström"), "Angstrom");
    }

    #[test]
    fn preserves_letter_case() {
        assert_eq!(normalize_for_search("Éléphant"), "Elephant");
        assert_eq!(normalize_for_search("Über"), "Uber");
    }

    #[test]
    fn keeps_characters_without_decomposable_diacritics() {
        // ß and ø have no canonical decomposition, so they survive NFD untouched.
        assert_eq!(normalize_for_search("Straße"), "Straße");
        assert_eq!(normalize_for_search("øre"), "øre");
        // Scripts without marks are passed through unchanged.
        assert_eq!(normalize_for_search("中文"), "中文");
        assert_eq!(normalize_for_search("Привет"), "Привет");
    }

    #[test]
    fn preserves_digits_and_punctuation() {
        assert_eq!(
            normalize_for_search("  Hello, world! #123.  "),
            "Hello, world! #123."
        );
    }

    #[test]
    fn handles_mark_only_and_leading_marks() {
        assert_eq!(normalize_for_search("\u{0301}"), "");
        assert_eq!(normalize_for_search("\u{0301}a\u{0301}"), "a");
    }

    #[test]
    fn normalization_is_idempotent() {
        let samples = ["Café déjà vu", "  Ångström  ", "Straße", "中文"];
        for sample in samples {
            let once = normalize_for_search(sample);
            assert_eq!(normalize_for_search(&once), once, "{sample:?}");
        }
    }
}
