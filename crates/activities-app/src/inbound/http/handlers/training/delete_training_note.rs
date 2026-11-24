use crate::inbound::http::error_status::ApiErrorStatus;
use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::domain::activity::ports::IActivityService;
use crate::domain::preferences::ports::IPreferencesService;
use crate::domain::training::models::TrainingNoteId;
use crate::domain::training::ports::ITrainingService;
use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::http::AppState;
use crate::{domain::training::ports::DeleteTrainingNoteError, inbound::parser::ParseFile};

impl ApiErrorStatus for DeleteTrainingNoteError {
    fn status_code(self) -> StatusCode {
        StatusCode::UNPROCESSABLE_ENTITY
    }
}

#[tracing::instrument(skip_all, err)]
pub async fn delete_training_note<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(note_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let note_id = TrainingNoteId::from(note_id.as_str());

    match state
        .training_metrics_service
        .delete_training_note(user.user(), &note_id)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            if matches!(&err, DeleteTrainingNoteError::Unknown(_)) {
                tracing::error!("Error deleting training note: {}", err.to_string());
            }
            Err(err.status_code())
        }
    }
}
