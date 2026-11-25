use crate::http::error_status::ApiErrorStatus;
use std::str::FromStr;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDate;

use crate::{
    domain::{
        activity::ports::IActivityService,
        preferences::ports::IPreferencesService,
        training::ports::{HooperIndexError, ITrainingService},
    },
    http::{AppState, handlers::training::types::APIHooperIndex},
    parser::ParseFile,
};
use activities_auth::infra::http::AuthenticatedUser;

/// Returns the Hooper index for a given date.
///
/// A date without a stored index is reported as an index with every measure unset rather than a
/// `404`, so callers can always treat the response shape the same way.
#[tracing::instrument(skip_all, err)]
pub async fn get_hooper_index<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(date): Path<String>,
) -> Result<Json<APIHooperIndex>, StatusCode> {
    let date = NaiveDate::from_str(&date).map_err(|_| StatusCode::BAD_REQUEST)?;

    match state
        .training_metrics_service
        .get_hooper_index(user.user(), &date)
        .await
    {
        Ok(index) => Ok(Json(APIHooperIndex::from(&index.unwrap_or_default()))),
        Err(err) => {
            if matches!(&err, HooperIndexError::Unknown(_)) {
                tracing::error!("Error getting hooper index: {}", err);
            }
            Err(err.status_code())
        }
    }
}
