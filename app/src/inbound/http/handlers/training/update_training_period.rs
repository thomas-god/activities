use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::training::TrainingPeriodId;
use crate::domain::ports::{
    IActivityService, IPreferencesService, ITrainingService, UpdateTrainingPeriodDatesError,
    UpdateTrainingPeriodDatesRequest, UpdateTrainingPeriodNameError,
    UpdateTrainingPeriodNameRequest, UpdateTrainingPeriodNoteError,
    UpdateTrainingPeriodNoteRequest,
};
use crate::inbound::http::AppState;
use crate::inbound::http::auth::{AuthenticatedUser, IUserService};
use crate::inbound::parser::ParseFile;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
pub struct UpdateTrainingPeriodBody {
    name: Option<String>,
    note: Option<String>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
}

pub async fn update_training_period<
    AS: IActivityService,
    PF: ParseFile,
    TS: ITrainingService,
    US: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TS, US, PS>>,
    Path(period_id): Path<Uuid>,
    axum::Json(body): axum::Json<UpdateTrainingPeriodBody>,
) -> Response {
    let period_id = TrainingPeriodId::from(&period_id.to_string());

    // Update name if provided
    if let Some(name) = body.name {
        let request =
            UpdateTrainingPeriodNameRequest::new(user.user().clone(), period_id.clone(), name);

        match state
            .training_metrics_service
            .update_training_period_name(request)
            .await
        {
            Ok(_) => {}
            Err(UpdateTrainingPeriodNameError::PeriodDoesNotExist(_)) => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(ErrorResponse {
                        error: "Training period does not exist".to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodNameError::UserDoesNotOwnPeriod(_, _)) => {
                return (
                    StatusCode::FORBIDDEN,
                    axum::Json(ErrorResponse {
                        error: "You do not have permission to update this training period"
                            .to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodNameError::Unknown(e)) => {
                eprintln!("Error updating training period name: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(ErrorResponse {
                        error: "Internal server error".to_string(),
                    }),
                )
                    .into_response();
            }
        }
    }

    // Update note if provided
    if body.note.is_some() {
        let request =
            UpdateTrainingPeriodNoteRequest::new(user.user().clone(), period_id.clone(), body.note);

        match state
            .training_metrics_service
            .update_training_period_note(request)
            .await
        {
            Ok(_) => {}
            Err(UpdateTrainingPeriodNoteError::PeriodDoesNotExist(_)) => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(ErrorResponse {
                        error: "Training period does not exist".to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodNoteError::UserDoesNotOwnPeriod(_, _)) => {
                return (
                    StatusCode::FORBIDDEN,
                    axum::Json(ErrorResponse {
                        error: "You do not have permission to update this training period"
                            .to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodNoteError::Unknown(e)) => {
                eprintln!("Error updating training period note: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(ErrorResponse {
                        error: "Internal server error".to_string(),
                    }),
                )
                    .into_response();
            }
        }
    }

    // Update dates if provided
    if let Some(start) = body.start {
        // When start is provided, use the end value from the request (or None if not provided)
        let end = body.end;

        let request = UpdateTrainingPeriodDatesRequest::new(
            user.user().clone(),
            period_id.clone(),
            start,
            end,
        );

        match state
            .training_metrics_service
            .update_training_period_dates(request)
            .await
        {
            Ok(_) => {}
            Err(UpdateTrainingPeriodDatesError::PeriodDoesNotExist(_)) => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(ErrorResponse {
                        error: "Training period does not exist".to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodDatesError::UserDoesNotOwnPeriod(_, _)) => {
                return (
                    StatusCode::FORBIDDEN,
                    axum::Json(ErrorResponse {
                        error: "You do not have permission to update this training period"
                            .to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodDatesError::EndDateBeforeStartDate) => {
                return (
                    StatusCode::BAD_REQUEST,
                    axum::Json(ErrorResponse {
                        error: "End date must be after start date".to_string(),
                    }),
                )
                    .into_response();
            }
            Err(UpdateTrainingPeriodDatesError::Unknown(e)) => {
                eprintln!("Error updating training period dates: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(ErrorResponse {
                        error: "Internal server error".to_string(),
                    }),
                )
                    .into_response();
            }
        }
    }

    StatusCode::OK.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_update_name_only() {
        let json = r#"{"name": "New Period Name"}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.name, Some("New Period Name".to_string()));
        assert_eq!(body.note, None);
        assert_eq!(body.start, None);
        assert_eq!(body.end, None);
    }

    #[test]
    fn test_deserialize_update_note_only() {
        let json = r#"{"note": "Updated note content"}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.name, None);
        assert_eq!(body.note, Some("Updated note content".to_string()));
        assert_eq!(body.start, None);
        assert_eq!(body.end, None);
    }

    #[test]
    fn test_deserialize_clear_note() {
        let json = r#"{"note": null}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.name, None);
        assert_eq!(body.note, None);
        assert_eq!(body.start, None);
        assert_eq!(body.end, None);
    }

    #[test]
    fn test_deserialize_update_dates_with_end() {
        let json = r#"{"start": "2025-12-01", "end": "2025-12-31"}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.name, None);
        assert_eq!(body.note, None);
        assert_eq!(body.start, Some("2025-12-01".parse::<NaiveDate>().unwrap()));
        assert_eq!(body.end, Some("2025-12-31".parse::<NaiveDate>().unwrap()));
    }

    #[test]
    fn test_deserialize_update_dates_without_end() {
        let json = r#"{"start": "2025-12-01"}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.name, None);
        assert_eq!(body.note, None);
        assert_eq!(body.start, Some("2025-12-01".parse::<NaiveDate>().unwrap()));
        assert_eq!(body.end, None);
    }

    #[test]
    fn test_deserialize_update_dates_clear_end() {
        // When end is explicitly null in JSON, serde deserializes it as None
        let json = r#"{"start": "2025-12-01", "end": null}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.name, None);
        assert_eq!(body.note, None);
        assert_eq!(body.start, Some("2025-12-01".parse::<NaiveDate>().unwrap()));
        assert_eq!(body.end, None);
    }

    #[test]
    fn test_deserialize_update_all_fields() {
        let json = r#"{
            "name": "Updated Period",
            "note": "Updated note",
            "start": "2025-12-01",
            "end": "2025-12-31"
        }"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.name, Some("Updated Period".to_string()));
        assert_eq!(body.note, Some("Updated note".to_string()));
        assert_eq!(body.start, Some("2025-12-01".parse::<NaiveDate>().unwrap()));
        assert_eq!(body.end, Some("2025-12-31".parse::<NaiveDate>().unwrap()));
    }

    #[test]
    fn test_deserialize_empty_body() {
        let json = r#"{}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();

        assert_eq!(body.name, None);
        assert_eq!(body.note, None);
        assert_eq!(body.start, None);
        assert_eq!(body.end, None);
    }

    #[test]
    fn test_end_deserialization_behavior() {
        // When end is not in request: body.end = None
        let json = r#"{"start": "2025-12-01"}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.end, None);

        // When end is explicitly null: body.end = None
        let json = r#"{"start": "2025-12-01", "end": null}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.end, None);

        // When end has a value: body.end = Some(date)
        let json = r#"{"start": "2025-12-01", "end": "2025-12-31"}"#;
        let body: UpdateTrainingPeriodBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.end, Some("2025-12-31".parse::<NaiveDate>().unwrap()));
    }
}
