use crate::http::error_status::ApiErrorStatus;
use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::domain::preferences::models::Preference;
use crate::http::AppState;
use activities_auth::infra::http::AuthenticatedUser;
use activities_domain::activity::ports::IActivityService;
use activities_domain::preferences::ports::IPreferencesService;
use activities_domain::preferences::ports::SetPreferenceError;
use activities_domain::training::ports::ITrainingService;
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
