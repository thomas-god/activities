use std::str::FromStr;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDate;

use crate::{
    domain::ports::{
        activity::IActivityService,
        preferences::IPreferencesService,
        training::{ITrainingService, WeightAndNutritionError},
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{AppState, handlers::training::types::APIWeightAndNutrition},
        parser::ParseFile,
    },
};

/// Returns the weight and nutrition values for a given date.
///
/// A date without stored values is reported as an entry with every measure unset rather than a
/// `404`, so callers can always treat the response shape the same way.
#[tracing::instrument(skip_all, err)]
pub async fn get_weight_and_nutrition<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(date): Path<String>,
) -> Result<Json<APIWeightAndNutrition>, StatusCode> {
    let date = NaiveDate::from_str(&date).map_err(|_| StatusCode::BAD_REQUEST)?;

    match state
        .training_metrics_service
        .get_weight_and_nutrition(user.user(), &date)
        .await
    {
        Ok(value) => Ok(Json(APIWeightAndNutrition::from(
            &value.unwrap_or_default(),
        ))),
        Err(err) => {
            if matches!(&err, WeightAndNutritionError::Unknown(_)) {
                tracing::error!("Error getting weight and nutrition: {}", err);
            }
            Err(StatusCode::from(err))
        }
    }
}
