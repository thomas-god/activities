use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::domain::ports::{IActivityService, IPreferencesService};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::{models::preferences::PreferenceKey, ports::ITrainingService},
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

use super::types::PreferenceResponse;

pub async fn get_preference<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Path(key): Path<String>,
) -> Result<Json<Option<PreferenceResponse>>, StatusCode> {
    let preference_key = key
        .parse::<PreferenceKey>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    state
        .preferences_service
        .get_preference(user.user(), &preference_key)
        .await
        .map(|pref| Json(pref.map(PreferenceResponse::from)))
        .map_err(StatusCode::from)
}
