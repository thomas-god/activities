use crate::http::error_status::ApiErrorStatus;
use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    domain::{
        activity::ports::IActivityService,
        preferences::{
            models::PreferenceKey,
            ports::{DeletePreferenceError, IPreferencesService},
        },
        training::ports::ITrainingService,
    },
    http::AppState,
};
use activities_auth::infra::http::AuthenticatedUser;
use activities_core::infra::parser::ParseFile;

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
