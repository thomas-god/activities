use crate::{DataMessage, DataValue, FitField};

pub fn find_field_value_by_kind<'a>(
    messages: &'a [DataMessage],
    target_kind: &FitField,
) -> Option<&'a [DataValue]> {
    messages.iter().find_map(|msg| {
        msg.fields.iter().find_map(|field| {
            if field.kind == *target_kind {
                Some(field.values.as_ref())
            } else {
                None
            }
        })
    })
}

pub fn find_fied_value_as_string(
    messages: &[DataMessage],
    target_field: &FitField,
) -> Option<String> {
    find_field_value_by_kind(messages, target_field).and_then(|values| {
        values.iter().find_map(|val| match val {
            DataValue::String(val) => Some(val.to_string()),
            _ => None,
        })
    })
}

#[cfg(test)]
mod tests {

    use crate::DataMessageField;

    use super::*;

    #[test]
    fn test_find_value_by_field_kind_not_found() {
        let messages = vec![DataMessage {
            local_message_type: 0,
            fields: vec![DataMessageField {
                kind: FitField::Record(crate::RecordField::Speed),
                values: vec![DataValue::Float32(1.3)],
            }],
        }];

        assert!(
            find_field_value_by_kind(
                &messages,
                &FitField::DeviceInfo(crate::DeviceInfoField::Product)
            )
            .is_none()
        );
    }

    #[test]
    fn test_find_value_by_field_kind_found() {
        let messages = vec![
            DataMessage {
                local_message_type: 0,
                fields: vec![DataMessageField {
                    kind: FitField::Record(crate::RecordField::Speed),
                    values: vec![DataValue::Float32(1.3)],
                }],
            },
            DataMessage {
                local_message_type: 0,
                fields: vec![DataMessageField {
                    kind: FitField::DeviceInfo(crate::DeviceInfoField::Product),
                    values: vec![DataValue::String("device".to_string())],
                }],
            },
        ];

        assert_eq!(
            find_field_value_by_kind(
                &messages,
                &FitField::DeviceInfo(crate::DeviceInfoField::Product)
            ),
            Some(vec![DataValue::String("device".to_string())].as_slice())
        );
    }

    #[test]
    fn test_find_value_by_field_kind_found_first_occurrence() {
        let messages = vec![
            DataMessage {
                local_message_type: 0,
                fields: vec![DataMessageField {
                    kind: FitField::Record(crate::RecordField::Speed),
                    values: vec![DataValue::Float32(1.3)],
                }],
            },
            DataMessage {
                local_message_type: 0,
                fields: vec![DataMessageField {
                    kind: FitField::DeviceInfo(crate::DeviceInfoField::Product),
                    values: vec![DataValue::String("device".to_string())],
                }],
            },
            DataMessage {
                local_message_type: 0,
                fields: vec![DataMessageField {
                    kind: FitField::DeviceInfo(crate::DeviceInfoField::Product),
                    values: vec![DataValue::String("another_device".to_string())],
                }],
            },
        ];

        assert_eq!(
            find_field_value_by_kind(
                &messages,
                &FitField::DeviceInfo(crate::DeviceInfoField::Product)
            ),
            Some(vec![DataValue::String("device".to_string())].as_slice())
        );
    }

    #[test]
    fn test_find_value_by_field_kind_as_string() {
        let messages = vec![DataMessage {
            local_message_type: 0,
            fields: vec![DataMessageField {
                kind: FitField::DeviceInfo(crate::DeviceInfoField::Product),
                values: vec![DataValue::String("device".to_string())],
            }],
        }];

        assert_eq!(
            find_fied_value_as_string(
                &messages,
                &FitField::DeviceInfo(crate::DeviceInfoField::Product)
            ),
            Some("device".to_string())
        );
    }

    #[test]
    fn test_find_value_by_field_kind_as_string_not_a_string() {
        let messages = vec![DataMessage {
            local_message_type: 0,
            fields: vec![DataMessageField {
                kind: FitField::DeviceInfo(crate::DeviceInfoField::Product),
                values: vec![DataValue::Uint32(12)],
            }],
        }];

        assert!(
            find_fied_value_as_string(
                &messages,
                &FitField::DeviceInfo(crate::DeviceInfoField::Product)
            )
            .is_none(),
        );
    }
}
