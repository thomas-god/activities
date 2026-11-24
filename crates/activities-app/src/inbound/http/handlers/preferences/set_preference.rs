use crate::inbound::http::error_status::ApiErrorStatus;
use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::{
    domain::models::preferences::Preference,
    inbound::{auth::AuthenticatedUser, http::AppState},
};
use activities_domain::ports::{
    activity::IActivityService,
    preferences::{IPreferencesService, SetPreferenceError},
    training::ITrainingService,
};
use activities_infra::parser::ParseFile;

use super::types::PreferencePayload;

#[tracing::instrument(skip_all, err)]
pub async fn set_preference<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Json(request): Json<PreferencePayload>,
) -> Result<StatusCode, StatusCode> {
    let preference = Preference::from(request);

    match state
        .preferences_service
        .set_preference(user.user(), preference)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            if matches!(&err, SetPreferenceError::Unknown(_)) {
                tracing::error!("Error setting preference: {}", err.to_string());
            }
            Err(err.status_code())
        }
    }
}
