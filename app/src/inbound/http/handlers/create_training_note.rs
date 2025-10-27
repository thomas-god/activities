use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::domain::ports::IActivityService;
use crate::inbound::parser::ParseFile;
use crate::{
    domain::{
        models::{UserId, training::TrainingNoteContent},
        ports::{CreateTrainingNoteError, CreateTrainingNoteRequest, ITrainingService},
    },
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

#[derive(Debug, Deserialize)]
pub struct CreateTrainingNoteBody {
    content: String,
}

#[derive(Debug, Serialize)]
pub struct CreateTrainingNoteResponse {
    id: String,
}

fn build_request(body: CreateTrainingNoteBody, user: &UserId) -> CreateTrainingNoteRequest {
    // TODO: replace None with title from request
    CreateTrainingNoteRequest::new(user.clone(), None, TrainingNoteContent::from(body.content))
}

impl From<CreateTrainingNoteError> for StatusCode {
    fn from(_value: CreateTrainingNoteError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

pub async fn create_training_note<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Json(payload): Json<CreateTrainingNoteBody>,
) -> Result<Json<CreateTrainingNoteResponse>, StatusCode> {
    let req = build_request(payload, user.user());

    state
        .training_metrics_service
        .create_training_note(req)
        .await
        .map(|note_id| {
            Json(CreateTrainingNoteResponse {
                id: note_id.to_string(),
            })
        })
        .map_err(StatusCode::from)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_payload_format() {
        assert!(
            serde_json::from_str::<CreateTrainingNoteBody>(
                r#"{
            "content": "This is a test training note"
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingNoteBody>(
                r#"{
            "content": ""
        }"#,
            )
            .is_ok()
        );
    }

    #[test]
    fn test_payload_format_missing_content() {
        assert!(serde_json::from_str::<CreateTrainingNoteBody>(r#"{}"#,).is_err());
    }
}
