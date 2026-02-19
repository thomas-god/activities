use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

use crate::domain::ports::{DateRange, IActivityService, IPreferencesService};
use crate::inbound::parser::ParseFile;
use crate::{
    domain::{
        models::training::{TrainingNote, TrainingPeriodId},
        ports::{GetTrainingNoteError, ITrainingService},
    },
    inbound::http::{AppState, auth::AuthenticatedUser, auth::IUserService},
};

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
    Query(query): Query<TrainingNotesQuery>,
) -> Result<Json<Vec<TrainingNoteResponse>>, StatusCode> {
    let date_range = Option::<DateRange>::from(&query);

    state
        .training_metrics_service
        .get_training_notes(user.user(), &date_range)
        .await
        .map(|notes| Json(notes.into_iter().map(TrainingNoteResponse::from).collect()))
        .map_err(StatusCode::from)
}

/// Get all training notes for a specific training period.
///
/// Returns all notes that fall within the date range of the specified training period.
/// For open-ended periods (no end date), includes today's activities.
pub async fn get_training_period_notes<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Path(period_id): Path<String>,
) -> Result<Json<Vec<TrainingNoteResponse>>, StatusCode> {
    let period_id = TrainingPeriodId::from(&period_id);

    state
        .training_metrics_service
        .get_training_period_notes(user.user(), &period_id)
        .await
        .map(|notes| Json(notes.into_iter().map(TrainingNoteResponse::from).collect()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
