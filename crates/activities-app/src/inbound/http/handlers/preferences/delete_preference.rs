use crate::inbound::http::error_status::ApiErrorStatus;
use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::domain::activity::ports::IActivityService;
use crate::domain::preferences::ports::DeletePreferenceError;
use crate::domain::preferences::ports::IPreferencesService;
use crate::domain::training::ports::ITrainingService;
use crate::{domain::preferences::models::PreferenceKey, inbound::http::AppState};
use activities_auth::infra::http::AuthenticatedUser;
use activities_infra::parser::ParseFile;

#[tracing::instrument(skip_all, err)]
pub async fn delete_preference<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(key): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let preference_key = key
        .parse::<PreferenceKey>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match state
        .preferences_service
        .delete_preference(user.user(), &preference_key)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            if matches!(&err, DeletePreferenceError::Unknown(_)) {
                tracing::error!("Error deleting preference: {}", err.to_string());
            }
            Err(err.status_code())
        }
    }
}
