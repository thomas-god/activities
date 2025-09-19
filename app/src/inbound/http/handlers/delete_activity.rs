use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    domain::{
        models::activity::ActivityId,
        ports::{
            DeleteActivityError, DeleteActivityRequest, IActivityService, ITrainingMetricService,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, ISessionRepository},
        },
        parser::ParseFile,
    },
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

pub async fn delete_activity<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    SR: ISessionRepository,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, SR>>,
    Path(activity_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let req = DeleteActivityRequest::new(user.user().clone(), ActivityId::from(&activity_id));
    state
        .activity_service
        .delete_activity(req)
        .await
        .map(|_| StatusCode::OK)
        .map_err(StatusCode::from)
}
