use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::domain::ports::IActivityService;
use crate::inbound::parser::ParseFile;
use crate::{
    domain::{
        models::training::{TrainingNoteContent, TrainingNoteId, TrainingNoteTitle},
        ports::{ITrainingService, UpdateTrainingNoteError},
    },
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

#[derive(Debug, Deserialize)]
pub struct UpdateTrainingNoteBody {
    title: Option<String>,
    content: String,
}

impl From<UpdateTrainingNoteError> for StatusCode {
    fn from(_value: UpdateTrainingNoteError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

pub async fn update_training_note<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Path(note_id): Path<String>,
    Json(payload): Json<UpdateTrainingNoteBody>,
) -> Result<StatusCode, StatusCode> {
    let note_id = TrainingNoteId::from(note_id.as_str());
    let title = payload.title.map(TrainingNoteTitle::from);
    let content = TrainingNoteContent::from(payload.content);

    state
        .training_metrics_service
        .update_training_note(user.user(), &note_id, title, content)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(StatusCode::from)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_payload_format() {
        assert!(
            serde_json::from_str::<UpdateTrainingNoteBody>(
                r#"{
            "title": "Updated title",
            "content": "Updated training note content"
        }"#,
            )
            .is_ok()
        );
    }

    #[test]
    fn test_payload_format_without_title() {
        assert!(
            serde_json::from_str::<UpdateTrainingNoteBody>(
                r#"{
            "content": "Updated training note content"
        }"#,
            )
            .is_ok()
        );
    }

    #[test]
    fn test_payload_format_missing_content() {
        assert!(serde_json::from_str::<UpdateTrainingNoteBody>(r#"{}"#,).is_err());
    }
}
