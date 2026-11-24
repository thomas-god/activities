use crate::inbound::http::error_status::ApiErrorStatus;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

use crate::domain::activity::ports::IActivityService;
use crate::domain::preferences::ports::IPreferencesService;
use crate::domain::training::models::TrainingNote;
use crate::domain::training::models::TrainingPeriodId;
use crate::domain::training::ports::ITrainingService;
use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::http::AppState;
use activities_domain::shared::DateRange;
use activities_domain::training::ports::GetTrainingNoteError;
use activities_infra::parser::ParseFile;

#[derive(Debug, Deserialize)]
pub struct TrainingNotesQuery {
    start: Option<DateTime<FixedOffset>>,
    end: Option<DateTime<FixedOffset>>,
}

impl From<&TrainingNotesQuery> for Option<DateRange> {
    fn from(value: &TrainingNotesQuery) -> Self {
        value.start.map(|start| {
            let start_date = start.date_naive();
            let end_date = value
                .end
                .map(|e| e.date_naive())
                .unwrap_or_else(|| Local::now().date_naive());
            DateRange::new(start_date, end_date)
        })
    }
}

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

impl ApiErrorStatus for GetTrainingNoteError {
    fn status_code(self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[tracing::instrument(skip_all, err)]
pub async fn get_training_notes<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Query(query): Query<TrainingNotesQuery>,
) -> Result<Json<Vec<TrainingNoteResponse>>, StatusCode> {
    let date_range = Option::<DateRange>::from(&query);

    match state
        .training_metrics_service
        .get_training_notes(user.user(), &date_range)
        .await
    {
        Ok(notes) => Ok(Json(
            notes.into_iter().map(TrainingNoteResponse::from).collect(),
        )),
        Err(err) => {
            if matches!(&err, GetTrainingNoteError::Unknown(_)) {
                tracing::error!("Error getting training notes: {}", err.to_string());
            }
            Err(err.status_code())
        }
    }
}

/// Get all training notes for a specific training period.
///
/// Returns all notes that fall within the date range of the specified training period.
/// For open-ended periods (no end date), includes today's activities.
#[tracing::instrument(skip_all, err)]
pub async fn get_training_period_notes<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(period_id): Path<String>,
) -> Result<Json<Vec<TrainingNoteResponse>>, StatusCode> {
    let period_id = TrainingPeriodId::from(&period_id);

    match state
        .training_metrics_service
        .get_training_period_notes(user.user(), &period_id)
        .await
    {
        Ok(notes) => Ok(Json(
            notes.into_iter().map(TrainingNoteResponse::from).collect(),
        )),
        Err(err) => {
            if matches!(&err, GetTrainingNoteError::Unknown(_)) {
                tracing::error!("Error getting training period notes: {}", err.to_string());
            }
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
