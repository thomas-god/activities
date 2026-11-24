use crate::inbound::http::error_status::ApiErrorStatus;
use std::str::FromStr;

use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDate;

use crate::domain::activity::ports::IActivityService;
use crate::domain::preferences::ports::IPreferencesService;
use crate::domain::training::ports::DeleteWeightAndNutritionRequest;
use crate::domain::training::ports::ITrainingService;
use crate::domain::training::ports::WeightAndNutritionError;
use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::http::AppState;
use crate::inbound::parser::ParseFile;

#[tracing::instrument(skip_all, err)]
pub async fn delete_weight_and_nutrition<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(date): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let date = NaiveDate::from_str(&date).map_err(|_| StatusCode::BAD_REQUEST)?;
    let req = DeleteWeightAndNutritionRequest::new(user.user().clone(), date);

    match state
        .training_metrics_service
        .delete_weight_and_nutrition(req)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            if matches!(&err, WeightAndNutritionError::Unknown(_)) {
                tracing::error!("Error deleting weight and nutrition: {}", err);
            }
            Err(err.status_code())
        }
    }
}
