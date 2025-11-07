use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::Serialize;

use crate::domain::ports::{IActivityService, IPreferencesService};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::{
        models::training::TrainingNote,
        ports::{GetTrainingNoteError, ITrainingService},
    },
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

#[derive(Debug, Serialize)]
pub struct TrainingNoteResponse {
    id: String,
    title: Option<String>,
    content: String,
    date: String,
    created_at: String,
}

impl From<TrainingNote> for TrainingNoteResponse {
    fn from(note: TrainingNote) -> Self {
        Self {
            id: note.id().to_string(),
            title: note.title().as_ref().map(|t| t.to_string()),
            content: note.content().to_string(),
            date: note.date().to_string(),
            created_at: note.created_at().to_rfc3339(),
        }
    }
}

impl From<GetTrainingNoteError> for StatusCode {
    fn from(_value: GetTrainingNoteError) -> Self {
        Self::INTERNAL_SERVER_ERROR
    }
}

pub async fn get_training_notes<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
) -> Result<Json<Vec<TrainingNoteResponse>>, StatusCode> {
    state
        .training_metrics_service
        .get_training_notes(user.user())
        .await
        .map(|notes| Json(notes.into_iter().map(TrainingNoteResponse::from).collect()))
        .map_err(StatusCode::from)
}
