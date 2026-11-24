use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{domain::ports::training::DeleteTrainingNoteError, inbound::parser::ParseFile};
use crate::{
    domain::{
        models::training::TrainingNoteId,
        ports::{
            activity::IActivityService, preferences::IPreferencesService,
            training::ITrainingService,
        },
    },
    inbound::{auth::AuthenticatedUser, http::AppState},
};

impl From<DeleteTrainingNoteError> for StatusCode {
    fn from(_value: DeleteTrainingNoteError) -> Self {
        Self::UNPROCESSABLE_ENTITY
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
            Err(StatusCode::from(err))
        }
    }
}
