use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::domain::ports::{
    activity::IActivityService, preferences::IPreferencesService, training::ITrainingService,
};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::models::preferences::Preference,
    inbound::http::{AppState, auth::AuthenticatedUser},
};

use super::types::SetPreferenceRequest;

pub async fn set_preference<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
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
