use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::domain::ports::{IActivityService, IPreferencesService};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::{models::preferences::Preference, ports::ITrainingService},
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

use super::types::SetPreferenceRequest;

pub async fn set_preference<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Json(request): Json<SetPreferenceRequest>,
) -> Result<StatusCode, StatusCode> {
    let preference = Preference::from(request);

    state
        .preferences_service
        .set_preference(user.user(), preference)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(StatusCode::from)
}
