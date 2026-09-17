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
        models::training::WeightAndNutritionPatch,
        ports::{
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{ITrainingService, SaveWeightAndNutritionRequest, WeightAndNutritionError},
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{AppState, handlers::training::types::APIWeightAndNutritionPatch},
        parser::ParseFile,
    },
};

/// Body of the `PATCH /api/training/weight-and-nutrition/{date}` request.
///
/// Fields are optional and follow the patch semantics described on
/// [`APIWeightAndNutritionPatch`]: omitted fields are left untouched, `null` clears them and a
/// value sets them.
#[derive(Debug, Deserialize)]
pub struct UpdateWeightAndNutritionBody {
    #[serde(flatten)]
    patch: APIWeightAndNutritionPatch,
}

#[tracing::instrument(skip_all, err)]
pub async fn save_weight_and_nutrition<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(date): Path<String>,
    Json(payload): Json<UpdateWeightAndNutritionBody>,
) -> Result<StatusCode, StatusCode> {
    let date = NaiveDate::from_str(&date).map_err(|_| StatusCode::BAD_REQUEST)?;
    let patch = WeightAndNutritionPatch::from(payload.patch);
    let req = SaveWeightAndNutritionRequest::new(user.user().clone(), date, patch);

    match state
        .training_metrics_service
        .save_weight_and_nutrition(req)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            if matches!(&err, WeightAndNutritionError::Unknown(_)) {
                tracing::error!("Error updating weight and nutrition: {}", err);
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
        let body: UpdateWeightAndNutritionBody =
            serde_json::from_str(r#"{ "weight": 70.5, "fat": null }"#).unwrap();

        assert_eq!(body.patch.weight, PatchField::Set(70.5));
        assert_eq!(body.patch.fat, PatchField::Clear);
        assert_eq!(body.patch.muscle, PatchField::Absent);
        assert_eq!(body.patch.calories, PatchField::Absent);
        assert_eq!(body.patch.alcohol, PatchField::Absent);
    }

    #[test]
    fn test_payload_accepts_empty_object() {
        let body: UpdateWeightAndNutritionBody = serde_json::from_str(r#"{}"#).unwrap();

        assert_eq!(body.patch, APIWeightAndNutritionPatch::default());
    }
}
