use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};

use crate::domain::ports::{IActivityService, IPreferencesService};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::{
        models::training::TrainingNoteId,
        ports::{DeleteTrainingNoteError, ITrainingService},
    },
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

impl From<DeleteTrainingNoteError> for StatusCode {
    fn from(_value: DeleteTrainingNoteError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

pub async fn delete_training_note<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Path(note_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let note_id = TrainingNoteId::from(note_id.as_str());

    state
        .training_metrics_service
        .delete_training_note(user.user(), &note_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(StatusCode::from)
}
