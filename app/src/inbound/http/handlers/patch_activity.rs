use axum::{
    Extension,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    domain::{
        models::activity::{ActivityId, ActivityName},
        ports::{
            IActivityService, ITrainingMetricService, ModifyActivityError, ModifyActivityRequest,
        },
    },
    inbound::{
        http::{AppState, auth::AuthenticatedUser},
        parser::ParseFile,
    },
};

impl From<ModifyActivityError> for StatusCode {
    fn from(value: ModifyActivityError) -> Self {
        match value {
            ModifyActivityError::ActivityDoesNotExist(_) => Self::NOT_FOUND,
            _ => Self::UNPROCESSABLE_ENTITY,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PatchActivityQuery {
    name: Option<String>,
}

pub async fn patch_activity<AS: IActivityService, PF: ParseFile, TMS: ITrainingMetricService>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS>>,
    Path(activity_id): Path<String>,
    Query(query): Query<PatchActivityQuery>,
) -> Result<StatusCode, StatusCode> {
    let req = ModifyActivityRequest::new(
        user.user().clone(),
        ActivityId::from(&activity_id),
        query.name.map(ActivityName::new),
    );

    state
        .activity_service
        .modify_activity(req)
        .await
        .map(|_| StatusCode::OK)
        .map_err(StatusCode::from)
}
