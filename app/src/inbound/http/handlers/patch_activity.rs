use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    domain::{
        models::{
            UserId,
            activity::{ActivityId, ActivityName},
        },
        ports::{
            IActivityService, ITrainingMetricService, ModifyActivityError, ModifyActivityRequest,
        },
    },
    inbound::{http::AppState, parser::ParseFile},
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
    State(state): State<AppState<AS, PF, TMS>>,
    Path(activity_id): Path<String>,
    Query(query): Query<PatchActivityQuery>,
) -> Result<StatusCode, StatusCode> {
    let req = ModifyActivityRequest::new(
        UserId::default(),
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
