use crate::http::error_status::ApiErrorStatus;
use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::domain::activity::models::ActivityId;
use crate::domain::activity::ports::DeleteActivityError;
use crate::domain::activity::ports::DeleteActivityRequest;
use crate::domain::activity::ports::IActivityService;
use crate::domain::preferences::ports::IPreferencesService;
use crate::domain::training::ports::ITrainingService;
use crate::http::AppState;
use crate::parser::ParseFile;
use activities_auth::infra::http::AuthenticatedUser;

impl ApiErrorStatus for DeleteActivityError {
    fn status_code(self) -> StatusCode {
        match self {
            DeleteActivityError::ActivityDoesNotExist(_) => StatusCode::NOT_FOUND,
            DeleteActivityError::UserDoesNotOwnActivity(_, _) => StatusCode::FORBIDDEN,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn delete_activity<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(activity_id): Path<String>,
) -> StatusCode {
    let req = DeleteActivityRequest::new(user.user().clone(), ActivityId::from(&activity_id));
    match state.activity_service.delete_activity(req).await {
        Ok(()) => StatusCode::OK,
        Err(err) => {
            if matches!(err, DeleteActivityError::Unknown(_)) {
                tracing::error!(
                    "Error while deleting activity {}: {}",
                    activity_id,
                    err.to_string()
                )
            }
            err.status_code()
        }
    }
}
