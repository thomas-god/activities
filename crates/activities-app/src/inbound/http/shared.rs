use serde::{
    Deserialize, Deserializer,
    de::{self, Visitor},
};
use std::{fmt, marker::PhantomData};

/// A patchable field to distinguishes a field that is absent from one that is explicitly cleared.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PatchField<T> {
    /// Field not provided in the request: leave the current value untouched.
    #[default]
    Absent,
    /// Field provided as `null`: clear/remove the current value.
    Clear,
    /// Field provided with a value: set it.
    Set(T),
}

impl<'de, T> Deserialize<'de> for PatchField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PatchFieldVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for PatchFieldVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = PatchField<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a field that can be absent, null or a value")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(PatchField::Clear)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(PatchField::Clear)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                T::deserialize(deserializer).map(PatchField::Set)
            }
        }

        deserializer.deserialize_option(PatchFieldVisitor(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[derive(Debug, PartialEq, Deserialize)]
    struct PatchBody {
        #[serde(default)]
        name: PatchField<String>,
        #[serde(default)]
        rpe: PatchField<u8>,
        #[serde(default)]
        tags: PatchField<Vec<String>>,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct Details {
        bonk_status: String,
        details: Option<String>,
    }

    // =========================================================================
    // Top-level deserialization
    // =========================================================================

    #[test]
    fn null_maps_to_clear_for_any_inner_type() {
        let field: PatchField<String> = serde_json::from_str("null").unwrap();
        assert_eq!(field, PatchField::Clear);

        let field: PatchField<u8> = serde_json::from_str("null").unwrap();
        assert_eq!(field, PatchField::Clear);

        let field: PatchField<Vec<String>> = serde_json::from_str("null").unwrap();
        assert_eq!(field, PatchField::Clear);

        let field: PatchField<Details> = serde_json::from_str("null").unwrap();
        assert_eq!(field, PatchField::Clear);
    }

    #[test]
    fn value_maps_to_set_for_scalar_types() {
        let field: PatchField<String> = serde_json::from_str(r#""hello""#).unwrap();
        assert_eq!(field, PatchField::Set("hello".to_string()));

        let field: PatchField<u8> = serde_json::from_str("7").unwrap();
        assert_eq!(field, PatchField::Set(7));
    }

    #[test]
    fn value_maps_to_set_for_containers() {
        let field: PatchField<Vec<String>> = serde_json::from_str(r#"["a", "b"]"#).unwrap();
        assert_eq!(
            field,
            PatchField::Set(vec!["a".to_string(), "b".to_string()])
        );

        let field: PatchField<Details> =
            serde_json::from_str(r#"{"bonk_status": "bonked", "details": "forgot to eat"}"#)
                .unwrap();
        assert_eq!(
            field,
            PatchField::Set(Details {
                bonk_status: "bonked".to_string(),
                details: Some("forgot to eat".to_string()),
            })
        );
    }

    #[test]
    fn inner_null_in_struct_value_is_deserialized_by_inner_type() {
        // Only the outer `null` means `Clear`; a `null` nested inside the value
        // is delegated to `T` and follows its own semantics (`Option<String>` -> None).
        let field: PatchField<Details> =
            serde_json::from_str(r#"{"bonk_status": "none", "details": null}"#).unwrap();
        assert_eq!(
            field,
            PatchField::Set(Details {
                bonk_status: "none".to_string(),
                details: None,
            })
        );
    }

    #[test]
    fn wrong_type_is_a_deserialization_error() {
        let result: Result<PatchField<String>, _> = serde_json::from_str("7");
        assert!(result.is_err());

        let result: Result<PatchField<u8>, _> = serde_json::from_str(r#""seven""#);
        assert!(result.is_err());

        let result: Result<PatchField<u8>, _> = serde_json::from_str("300");
        assert!(result.is_err(), "u8 out of range must fail");

        let result: Result<PatchField<Vec<String>>, _> =
            serde_json::from_str(r#"{"not": "a list"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_json_is_a_deserialization_error() {
        let result: Result<PatchField<String>, _> = serde_json::from_str("nul");
        assert!(result.is_err());

        let result: Result<PatchField<String>, _> = serde_json::from_str("");
        assert!(result.is_err());
    }

    // =========================================================================
    // Deserialization inside a struct with #[serde(default)]
    // =========================================================================

    #[test]
    fn absent_field_stays_absent() {
        let body: PatchBody = serde_json::from_value(json!({})).unwrap();
        assert_eq!(body.name, PatchField::Absent);
        assert_eq!(body.rpe, PatchField::Absent);
        assert_eq!(body.tags, PatchField::Absent);
    }

    #[test]
    fn mixed_absent_null_value_states_in_one_body() {
        let body: PatchBody = serde_json::from_value(json!({
            "name": null,          // null  => clear
            "rpe": 5,              // value => set
                                   // tags omitted => absent
        }))
        .unwrap();

        assert_eq!(body.name, PatchField::Clear);
        assert_eq!(body.rpe, PatchField::Set(5));
        assert_eq!(body.tags, PatchField::Absent);
    }
}
