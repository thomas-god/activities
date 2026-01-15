use axum::body::Body;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::{
    Extension,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::io::Write;

use crate::{
    domain::ports::{
        GetAllActivitiesRequest, IActivityService, IPreferencesService, ITrainingService,
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::ParseFile,
    },
};

pub async fn get_all_activities<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
) -> Result<Response, StatusCode> {
    let request = GetAllActivitiesRequest::new(user.user().clone());

    let activities = state
        .activity_service
        .get_all_activities(request)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create a ZIP file containing all activities
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for activity in activities {
        zip.start_file(activity.name(), options)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        zip.write_all(activity.content())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let zip_data = zip
        .finish()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_inner();

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/zip")
        .header(
            CONTENT_DISPOSITION,
            "attachment; filename=\"activities.zip\"",
        )
        .body(Body::from(zip_data))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        models::UserId, ports::RawActivity, services::activity::test_utils::MockActivityService,
        services::preferences::tests_utils::MockPreferencesService,
        services::training::test_utils::MockTrainingService,
    };
    use crate::inbound::http::auth::test_utils::MockUserService;
    use crate::inbound::parser::test_utils::MockFileParser;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_get_all_activities_returns_zip() {
        let user = UserId::test_default();
        let authenticated_user = AuthenticatedUser::new(user.clone());

        let mut activity_service = MockActivityService::new();
        activity_service.expect_get_all_activities().returning(|_| {
            Ok(vec![
                RawActivity::new("activity1.fit".to_string(), vec![1, 2, 3]),
                RawActivity::new("activity2.tcx".to_string(), vec![4, 5, 6]),
            ])
        });

        let state = AppState {
            activity_service: Arc::new(activity_service),
            file_parser: Arc::new(MockFileParser::new()),
            training_metrics_service: Arc::new(MockTrainingService::new()),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(crate::inbound::http::CookieConfig::default()),
        };

        let response = get_all_activities(Extension(authenticated_user), State(state))
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
        activity_service.expect_get_all_activities().returning(|_| {
            Err(crate::domain::ports::GetAllActivitiesError::Unknown(
                anyhow::anyhow!("error"),
            ))
        });

        let state = AppState {
            activity_service: Arc::new(activity_service),
            file_parser: Arc::new(MockFileParser::new()),
            training_metrics_service: Arc::new(MockTrainingService::new()),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(crate::inbound::http::CookieConfig::default()),
        };

        let result = get_all_activities(Extension(authenticated_user), State(state)).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_get_all_activities_empty_list() {
        let user = UserId::test_default();
        let authenticated_user = AuthenticatedUser::new(user.clone());

        let mut activity_service = MockActivityService::new();
        activity_service
            .expect_get_all_activities()
            .returning(|_| Ok(vec![]));

        let state = AppState {
            activity_service: Arc::new(activity_service),
            file_parser: Arc::new(MockFileParser::new()),
            training_metrics_service: Arc::new(MockTrainingService::new()),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(crate::inbound::http::CookieConfig::default()),
        };

        let response = get_all_activities(Extension(authenticated_user), State(state))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
