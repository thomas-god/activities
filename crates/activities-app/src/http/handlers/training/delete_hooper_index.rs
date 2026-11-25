use crate::http::error_status::ApiErrorStatus;
use std::str::FromStr;

use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDate;

use crate::{
    domain::{
        activity::ports::IActivityService,
        preferences::ports::IPreferencesService,
        training::ports::{DeleteHooperIndexRequest, HooperIndexError, ITrainingService},
    },
    http::AppState,
    parser::ParseFile,
};
use activities_auth::infra::http::AuthenticatedUser;

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
            Err(err.status_code())
        }
    }
}
