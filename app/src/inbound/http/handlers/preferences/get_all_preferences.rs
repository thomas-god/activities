use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::domain::ports::{
    activity::IActivityService, preferences::IPreferencesService, training::ITrainingService,
};
use crate::inbound::http::{AppState, auth::AuthenticatedUser};
use crate::inbound::parser::ParseFile;

use super::types::PreferenceResponse;

pub async fn get_all_preferences<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
) -> Result<Json<Vec<PreferenceResponse>>, StatusCode> {
    state
        .preferences_service
        .get_all_preferences(user.user())
        .await
        .map(|prefs| Json(prefs.into_iter().map(PreferenceResponse::from).collect()))
        .map_err(StatusCode::from)
}
