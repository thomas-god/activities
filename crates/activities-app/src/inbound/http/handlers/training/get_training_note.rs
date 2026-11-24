use crate::inbound::http::error_status::ApiErrorStatus;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    domain::training::models::TrainingNoteId,
    inbound::{auth::AuthenticatedUser, http::AppState},
};
use activities_domain::activity::ports::IActivityService;
use activities_domain::preferences::ports::IPreferencesService;
use activities_domain::training::ports::GetTrainingNoteError;
use activities_domain::training::ports::ITrainingService;
use activities_infra::parser::ParseFile;

use super::get_training_notes::TrainingNoteResponse;

#[tracing::instrument(skip_all, err)]
pub async fn get_training_note<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(note_id): Path<String>,
) -> Result<Json<TrainingNoteResponse>, StatusCode> {
    let note_id = TrainingNoteId::from(note_id.as_str());

    let note = match state
        .training_metrics_service
        .get_training_note(user.user(), &note_id)
        .await
    {
        Ok(note) => note,
        Err(err) => {
            if matches!(&err, GetTrainingNoteError::Unknown(_)) {
                tracing::error!("Error getting training note: {}", err.to_string());
            }
            return Err(err.status_code());
        }
    };

    note.map(|note| Json(TrainingNoteResponse::from(note)))
        .ok_or(StatusCode::NOT_FOUND)
}
