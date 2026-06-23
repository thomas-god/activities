use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::inbound::parser::ParseFile;
use crate::{domain::models::preferences::PreferenceKey, inbound::http::AppState};
use crate::{
    domain::ports::{
        activity::IActivityService, preferences::IPreferencesService, training::ITrainingService,
    },
    inbound::auth::AuthenticatedUser,
};

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

    state
        .preferences_service
        .delete_preference(user.user(), &preference_key)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(StatusCode::from)
}
