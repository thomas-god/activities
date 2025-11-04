use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::domain::ports::{IActivityService, IPreferencesService};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::{models::preferences::PreferenceKey, ports::ITrainingService},
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

pub async fn delete_preference<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
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
