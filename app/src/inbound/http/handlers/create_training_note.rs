use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::domain::models::training::{TrainingNoteDate, TrainingNoteTitle};
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
    title: Option<String>,
    content: String,
    date: String,
}

#[derive(Debug, Serialize)]
pub struct CreateTrainingNoteResponse {
    id: String,
}

fn build_request(
    body: CreateTrainingNoteBody,
    user: &UserId,
) -> Result<CreateTrainingNoteRequest, StatusCode> {
    let date = TrainingNoteDate::try_from(body.date).map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(CreateTrainingNoteRequest::new(
        user.clone(),
        body.title.map(TrainingNoteTitle::from),
        TrainingNoteContent::from(body.content),
        date,
    ))
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
    let req = build_request(payload, user.user())?;

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
            "content": "This is a test training note",
            "date": "2025-10-27"
        }"#,
            )
            .is_ok()
        );

        assert!(
            serde_json::from_str::<CreateTrainingNoteBody>(
                r#"{
            "content": "",
            "date": "2025-10-27"
        }"#,
            )
            .is_ok()
        );
    }

    #[test]
    fn test_payload_format_missing_content() {
        assert!(serde_json::from_str::<CreateTrainingNoteBody>(r#"{}"#,).is_err());
    }

    #[test]
    fn test_payload_format_missing_date() {
        assert!(
            serde_json::from_str::<CreateTrainingNoteBody>(
                r#"{
            "content": "This is a test training note"
        }"#,
            )
            .is_err()
        );
    }

    #[test]
    fn test_build_request_with_invalid_date() {
        let body = CreateTrainingNoteBody {
            title: None,
            content: "Test content".to_string(),
            date: "invalid-date".to_string(),
        };
        let user = UserId::new();
        let result = build_request(body, &user);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }
}
