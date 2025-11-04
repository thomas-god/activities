use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::domain::ports::{IActivityService, IPreferencesService};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::ports::ITrainingService,
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

use super::types::PreferenceResponse;

pub async fn get_all_preferences<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
) -> Result<Json<Vec<PreferenceResponse>>, StatusCode> {
    state
        .preferences_service
        .get_all_preferences(user.user())
        .await
        .map(|prefs| Json(prefs.into_iter().map(PreferenceResponse::from).collect()))
        .map_err(StatusCode::from)
}
