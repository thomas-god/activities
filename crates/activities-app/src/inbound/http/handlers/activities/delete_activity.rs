use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    domain::{
        models::activity::ActivityId,
        ports::activity::{DeleteActivityError, DeleteActivityRequest, IActivityService},
        ports::preferences::IPreferencesService,
        ports::training::ITrainingService,
    },
    inbound::{auth::AuthenticatedUser, http::AppState, parser::ParseFile},
};

impl From<DeleteActivityError> for StatusCode {
    fn from(value: DeleteActivityError) -> Self {
        match value {
            DeleteActivityError::ActivityDoesNotExist(_) => Self::NOT_FOUND,
            DeleteActivityError::UserDoesNotOwnActivity(_, _) => Self::FORBIDDEN,
            _ => Self::UNPROCESSABLE_ENTITY,
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
            StatusCode::from(err)
        }
    }
}
