use std::str::FromStr;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::{
    domain::{
        models::training::HooperIndexPatch,
        ports::{
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{HooperIndexError, ITrainingService, SaveHooperIndexRequest},
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{AppState, handlers::training::types::APIHooperIndexPatch},
        parser::ParseFile,
    },
};

/// Body of the `PATCH /api/training/hooper-index/{date}` request.
///
/// Fields are optional and follow the patch semantics described on [`APIHooperIndexPatch`]:
/// omitted fields are left untouched, `null` clears them and a value in `1..=10` sets them.
#[derive(Debug, Deserialize)]
pub struct UpdateHooperIndexBody {
    #[serde(flatten)]
    patch: APIHooperIndexPatch,
}

#[tracing::instrument(skip_all, err)]
pub async fn save_hooper_index<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(date): Path<String>,
    Json(payload): Json<UpdateHooperIndexBody>,
) -> Result<StatusCode, StatusCode> {
    let date = NaiveDate::from_str(&date).map_err(|_| StatusCode::BAD_REQUEST)?;
    let patch = HooperIndexPatch::try_from(payload.patch).map_err(|_| StatusCode::BAD_REQUEST)?;
    let req = SaveHooperIndexRequest::new(user.user().clone(), date, patch);

    match state.training_metrics_service.save_hooper_index(req).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            if matches!(&err, HooperIndexError::Unknown(_)) {
                tracing::error!("Error updating hooper index: {}", err);
            }
            Err(StatusCode::from(err))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::inbound::http::shared::PatchField;

    use super::*;

    #[test]
    fn test_payload_distinguishes_absent_null_and_value() {
        let body: UpdateHooperIndexBody =
            serde_json::from_str(r#"{ "fatigue": 5, "sleep": null }"#).unwrap();

        assert_eq!(body.patch.fatigue, PatchField::Set(5));
        assert_eq!(body.patch.sleep, PatchField::Clear);
        assert_eq!(body.patch.pain, PatchField::Absent);
        assert_eq!(body.patch.stress, PatchField::Absent);
        assert_eq!(body.patch.mood, PatchField::Absent);
    }

    #[test]
    fn test_payload_accepts_empty_object() {
        let body: UpdateHooperIndexBody = serde_json::from_str(r#"{}"#).unwrap();

        assert_eq!(body.patch, APIHooperIndexPatch::default());
    }
}
