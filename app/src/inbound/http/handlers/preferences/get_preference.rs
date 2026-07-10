use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::domain::ports::{
    activity::IActivityService,
    preferences::{GetPreferenceError, IPreferencesService},
    training::ITrainingService,
};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::models::preferences::PreferenceKey,
    inbound::{auth::AuthenticatedUser, http::AppState},
};

use super::types::PreferenceResponse;

pub async fn get_preference<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(key): Path<String>,
) -> Result<Json<Option<PreferenceResponse>>, StatusCode> {
    let preference_key = key
        .parse::<PreferenceKey>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match state
        .preferences_service
        .get_preference(user.user(), &preference_key)
        .await
    {
        Ok(pref) => Ok(Json(pref.map(PreferenceResponse::from))),
        Err(err) => {
            if matches!(&err, GetPreferenceError::Unknown(_)) {
                tracing::error!("Error getting preference: {}", err.to_string());
            }
            Err(StatusCode::from(err))
        }
    }
}
