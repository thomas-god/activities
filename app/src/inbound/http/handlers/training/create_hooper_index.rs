use axum::{Extension, Json, extract::State, http::StatusCode};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::{
    domain::{
        models::{
            UserId,
            training::{HooperIndex, SubjectiveScale},
        },
        ports::{
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{AddHooperIndexRequest, HooperIndexError, ITrainingService},
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{AppState, handlers::training::types::APIHooperIndex},
        parser::ParseFile,
    },
};

/// Body of the `POST /api/training/hooper-index` request.
///
/// Every subjective measure is optional and must be in the `1..=10` range. Saving an index for an
/// existing `(user, date)` pair replaces the stored values.
#[derive(Debug, Deserialize)]
pub struct CreateHooperIndexBody {
    date: NaiveDate,
    #[serde(flatten)]
    values: APIHooperIndex,
}

fn build_request(
    body: CreateHooperIndexBody,
    user: &UserId,
) -> Result<AddHooperIndexRequest, StatusCode> {
    let value = HooperIndex::try_from(body.values).map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(AddHooperIndexRequest::new(user.clone(), body.date, value))
}

#[tracing::instrument(skip_all, err)]
pub async fn create_hooper_index<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Json(payload): Json<CreateHooperIndexBody>,
) -> Result<StatusCode, StatusCode> {
    let req = build_request(payload, user.user())?;

    match state
        .training_metrics_service
        .create_hooper_index(req)
        .await
    {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(err) => {
            if matches!(&err, HooperIndexError::Unknown(_)) {
                tracing::error!("Error creating hooper index: {}", err);
            }
            Err(StatusCode::from(err))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_deserializes_with_all_values() {
        let body: CreateHooperIndexBody = serde_json::from_str(
            r#"{
                "date": "2026-01-15",
                "fatigue": 1,
                "sleep": 2,
                "pain": 3,
                "stress": 4,
                "mood": 5
            }"#,
        )
        .unwrap();

        assert_eq!(body.date, NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
        assert_eq!(
            body.values,
            APIHooperIndex {
                fatigue: Some(1),
                sleep: Some(2),
                pain: Some(3),
                stress: Some(4),
                mood: Some(5),
            }
        );
    }

    #[test]
    fn test_payload_deserializes_with_only_date() {
        let body: CreateHooperIndexBody =
            serde_json::from_str(r#"{ "date": "2026-01-15" }"#).unwrap();

        assert_eq!(body.values, APIHooperIndex::default());
    }

    #[test]
    fn test_payload_requires_date() {
        let result: Result<CreateHooperIndexBody, _> = serde_json::from_str(r#"{ "fatigue": 3 }"#);

        assert!(result.is_err());
    }

    #[test]
    fn test_payload_rejects_invalid_date() {
        let result: Result<CreateHooperIndexBody, _> =
            serde_json::from_str(r#"{ "date": "not-a-date" }"#);

        assert!(result.is_err());
    }

    #[test]
    fn test_build_request_accepts_valid_values() {
        let body: CreateHooperIndexBody =
            serde_json::from_str(r#"{ "date": "2026-01-15", "fatigue": 10, "mood": 1 }"#).unwrap();
        let user = UserId::from("user1");

        let req = build_request(body, &user).expect("Should build request");

        assert_eq!(req.user(), &user);
        assert_eq!(req.date(), &NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
        assert_eq!(
            req.value().fatigue(),
            &Some(SubjectiveScale::try_from(10).unwrap())
        );
        assert_eq!(
            req.value().mood(),
            &Some(SubjectiveScale::try_from(1).unwrap())
        );
        assert_eq!(req.value().sleep(), &None);
    }

    #[test]
    fn test_build_request_rejects_out_of_range_values() {
        for invalid in ["0", "11", "255"] {
            let body: CreateHooperIndexBody = serde_json::from_str(&format!(
                r#"{{ "date": "2026-01-15", "fatigue": {invalid} }}"#
            ))
            .unwrap();

            let result = build_request(body, &UserId::from("user1"));

            assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
        }
    }
}
