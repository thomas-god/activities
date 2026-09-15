use std::str::FromStr;

use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDate;

use crate::{
    domain::ports::{
        activity::IActivityService,
        preferences::IPreferencesService,
        training::{DeleteHooperIndexRequest, HooperIndexError, ITrainingService},
    },
    inbound::{auth::AuthenticatedUser, http::AppState, parser::ParseFile},
};

#[tracing::instrument(skip_all, err)]
pub async fn delete_hooper_index<
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
    let req = DeleteHooperIndexRequest::new(user.user().clone(), date);

    match state
        .training_metrics_service
        .delete_hooper_index(req)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            if matches!(&err, HooperIndexError::Unknown(_)) {
                tracing::error!("Error deleting hooper index: {}", err);
            }
            Err(StatusCode::from(err))
        }
    }
}
