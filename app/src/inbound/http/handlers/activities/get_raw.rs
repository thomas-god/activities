use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::{
    Extension,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::io::Write;
use zip::result::ZipError;

use crate::domain::models::activity::ActivityId;
use crate::domain::ports::activity::{
    GetAllActivitiesError, GetRawActivityError, GetRawActivityRequest, RawActivity,
};
use crate::{
    domain::ports::{
        activity::{GetAllActivitiesRequest, IActivityService},
        preferences::IPreferencesService,
        training::ITrainingService,
    },
    inbound::{auth::AuthenticatedUser, http::AppState, parser::ParseFile},
};

pub async fn get_all_raw_activities<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
) -> Result<Response, StatusCode> {
    let request = GetAllActivitiesRequest::new(user.user().clone());

    let activities = match state.activity_service.get_all_raw_activities(request).await {
        Ok(activities) => activities,
        Err(err) => {
            if matches!(err, GetAllActivitiesError::Unknown(_)) {
                tracing::error!(
                    "Error while getting all raw activity files: {}",
                    err.to_string()
                );
            }
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let zip_data = match zip_activities(activities) {
        Ok(data) => data,
        Err(err) => {
            tracing::error!(
                "Error while trying to zip raw activity files: {}",
                err.to_string()
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let response = match Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/zip")
        .header(
            CONTENT_DISPOSITION,
            "attachment; filename=\"activities.zip\"",
        )
        .body(Body::from(zip_data))
    {
        Ok(body) => body,
        Err(err) => {
            tracing::error!(
                "Error while building raw activities body: {}",
                err.to_string()
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(response)
}

/// Create a ZIP file containing all activities raw files
fn zip_activities(activities: Vec<RawActivity>) -> Result<Vec<u8>, ZipError> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for activity in activities {
        zip.start_file(activity.name(), options)?;
        zip.write_all(activity.content())?;
    }

    Ok(zip.finish()?.into_inner())
}

pub async fn get_raw_activity<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(activity): Path<String>,
) -> Result<Response, StatusCode> {
    let request = GetRawActivityRequest::new(ActivityId::from(&activity), user.user().clone());

    let activity = match state.activity_service.get_raw_activity(request).await {
        Ok(activity) => activity,
        Err(GetRawActivityError::ActivityDoesNotExist(_)) => return Err(StatusCode::NOT_FOUND),
        Err(GetRawActivityError::Unknown(err)) => {
            tracing::error!(
                "Error while getting raw activity {}: {}",
                activity,
                err.to_string()
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let content_type = if activity.name().ends_with(".tcx") {
        "application/xml"
    } else {
        "application/octet-stream"
    };

    let response = match Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", activity.name()),
        )
        .body(Body::from(activity.as_vec()))
    {
        Ok(body) => body,
        Err(err) => {
            tracing::error!(
                "Error while building raw activity body: {}",
                err.to_string()
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        models::UserId, ports::activity::RawActivity,
        services::activity::test_utils::MockActivityService,
        services::preferences::tests_utils::MockPreferencesService,
        services::training::test_utils::MockTrainingService,
    };
    use crate::inbound::parser::test_utils::MockFileParser;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_get_all_activities_returns_zip() {
        let user = UserId::test_default();
        let authenticated_user = AuthenticatedUser::new(user.clone());

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_get_all_raw_activities()
            .returning(|_| {
                Ok(vec![
                    RawActivity::new("activity1.fit".to_string(), vec![1, 2, 3]),
                    RawActivity::new("activity2.tcx".to_string(), vec![4, 5, 6]),
                ])
            });

        let state = AppState {
            activity_service: Arc::new(activity_service),
            file_parser: Arc::new(MockFileParser::new()),
            training_metrics_service: Arc::new(MockTrainingService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        let response = get_all_raw_activities(Extension(authenticated_user), State(state))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();
        assert_eq!(headers.get(CONTENT_TYPE).unwrap(), "application/zip");
        assert_eq!(
            headers.get(CONTENT_DISPOSITION).unwrap(),
            "attachment; filename=\"activities.zip\""
        );
    }

    #[tokio::test]
    async fn test_get_all_activities_service_error() {
        let user = UserId::test_default();
        let authenticated_user = AuthenticatedUser::new(user.clone());

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_get_all_raw_activities()
            .returning(|_| {
                Err(
                    crate::domain::ports::activity::GetAllActivitiesError::Unknown(
                        anyhow::anyhow!("error"),
                    ),
                )
            });

        let state = AppState {
            activity_service: Arc::new(activity_service),
            file_parser: Arc::new(MockFileParser::new()),
            training_metrics_service: Arc::new(MockTrainingService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        let result = get_all_raw_activities(Extension(authenticated_user), State(state)).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_get_all_activities_empty_list() {
        let user = UserId::test_default();
        let authenticated_user = AuthenticatedUser::new(user.clone());

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_get_all_raw_activities()
            .returning(|_| Ok(vec![]));

        let state = AppState {
            activity_service: Arc::new(activity_service),
            file_parser: Arc::new(MockFileParser::new()),
            training_metrics_service: Arc::new(MockTrainingService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        let response = get_all_raw_activities(Extension(authenticated_user), State(state))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
