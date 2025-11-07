use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::domain::ports::{IActivityService, IPreferencesService};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::{models::training::TrainingNoteId, ports::ITrainingService},
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

use super::get_training_notes::TrainingNoteResponse;

pub async fn get_training_note<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Path(note_id): Path<String>,
) -> Result<Json<TrainingNoteResponse>, StatusCode> {
    let note_id = TrainingNoteId::from(note_id.as_str());

    state
        .training_metrics_service
        .get_training_note(user.user(), &note_id)
        .await
        .map_err(StatusCode::from)?
        .map(|note| Json(TrainingNoteResponse::from(note)))
        .ok_or(StatusCode::NOT_FOUND)
}
