use std::io::Read;

use anyhow::anyhow;
use axum::{
    Extension, Json,
    extract::{Multipart, State, multipart::Field},
    http::StatusCode,
    response::IntoResponse,
};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use crate::{
    domain::ports::{CreateActivityError, IActivityService, IPreferencesService, ITrainingService},
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::{ParseBytesError, ParseFile, SupportedExtension},
    },
};

impl From<CreateActivityError> for StatusCode {
    fn from(_value: CreateActivityError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

#[derive(Serialize, Deserialize)]
struct UploadActivitiesResponse {
    created_ids: Vec<String>,
    unprocessable_files: Vec<(String, RejectionReason)>,
}

#[derive(Debug, Serialize, Deserialize)]
enum RejectionReason {
    CannotReadContent,
    CannotProcessFile,
    DuplicatedActivity,
    IncoherentTimeseries,
    UnsupportedFileExtension,
    Unknown,
}

impl From<CreateActivityError> for RejectionReason {
    fn from(value: CreateActivityError) -> Self {
        match value {
            CreateActivityError::SimilarActivityExistsError => Self::DuplicatedActivity,
            _ => Self::Unknown,
        }
    }
}

pub async fn upload_activities<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    mut multipart: Multipart,
) -> Result<impl axum::response::IntoResponse, StatusCode> {
    let mut created_ids = Vec::new();
    let mut unprocessable_files = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let Some(name) = field.name().map(|n| n.to_string()) else {
            continue;
        };
        let Some(extension) = extract_extension(&name) else {
            unprocessable_files.push((name.to_string(), RejectionReason::UnsupportedFileExtension));
            continue;
        };
        let Ok(file_content) = extract_content(&name, field).await else {
            unprocessable_files.push((name.to_string(), RejectionReason::CannotReadContent));
            continue;
        };

        let parsed_content = match state
            .file_parser
            .try_bytes_into_domain(&extension, file_content)
        {
            Ok(parsed_content) => parsed_content,
            Err(ParseBytesError::IncoherentTimeseriesLengths) => {
                unprocessable_files.push((name.to_string(), RejectionReason::IncoherentTimeseries));
                continue;
            }
            Err(_) => {
                unprocessable_files.push((name.to_string(), RejectionReason::CannotProcessFile));
                continue;
            }
        };

        let create_activity_request = parsed_content.into_request(user.user());

        match state
            .activity_service
            .create_activity(create_activity_request)
            .await
        {
            Ok(activity) => {
                created_ids.push(activity.id().to_string());
            }
            Err(err) => {
                unprocessable_files.push((name.to_string(), err.into()));
            }
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(UploadActivitiesResponse {
            created_ids,
            unprocessable_files,
        }),
    )
        .into_response())
}

async fn extract_content(filename: &str, field: Field<'_>) -> Result<Vec<u8>, anyhow::Error> {
    let content = match field.bytes().await {
        Ok(content) => content,
        Err(err) => return Err(anyhow!(err)),
    };

    if filename.ends_with(".gz") {
        let mut gz = GzDecoder::new(&content[..]);
        let mut content = Vec::new();
        if let Err(err) = gz.read_to_end(&mut content) {
            return Err(anyhow!(err));
        };

        return Ok(content);
    }

    Ok(content.to_vec())
}

fn extract_extension(filename: &str) -> Option<SupportedExtension> {
    let mut parts = filename.split('.').rev();
    let mut part = parts.next();

    if let Some("gz") = part {
        part = parts.next();
    }

    part.and_then(|p| SupportedExtension::try_from(p).ok())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{Router, middleware::from_extractor, routing::post};
    use axum_test::TestServer;
    use mockall::Sequence;

    use crate::{
        domain::{
            models::{
                UserId,
                activity::{Activity, ActivityId, ActivityStartTime, ActivityStatistics, Sport},
            },
            services::{
                activity::test_utils::MockActivityService,
                preferences::tests_utils::MockPreferencesService,
                training::test_utils::MockTrainingService,
            },
        },
        inbound::{
            http::{
                CookieConfig,
                auth::{DefaultUserExtractor, test_utils::MockUserService},
            },
            parser::test_utils::MockFileParser,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_upload_single_activity() {
        let mut service = MockActivityService::new();
        let expected_id = ActivityId::new();
        let expected_id_clone = expected_id.clone();
        service
            .expect_create_activity()
            .times(1)
            .returning(move |_| {
                Ok(Activity::new(
                    expected_id_clone.clone(),
                    UserId::test_default(),
                    None,
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    Sport::Running,
                    ActivityStatistics::default(),
                    None,
                    None,
                    None,
                    None,
                ))
            });

        let metrics = MockTrainingService::test_default();
        let file_parser = MockFileParser::test_default();
        let state = AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        };

        let app = Router::new()
            .route("/test_upload", post(upload_activities))
            .route_layer(from_extractor::<DefaultUserExtractor>())
            .with_state(state);
        let server = TestServer::new(app).expect("unable to create test server");

        let file1_data = b"test fit file content 1".to_vec();

        let response = server
            .post("/test_upload")
            .multipart(axum_test::multipart::MultipartForm::new().add_part(
                "test1.fit".to_string(),
                axum_test::multipart::Part::bytes(file1_data),
            ))
            .await;

        response.assert_status(StatusCode::CREATED);
        let json: UploadActivitiesResponse = response.json();
        assert_eq!(json.created_ids.len(), 1);
        assert_eq!(json.created_ids[0], expected_id.to_string());
        assert!(json.unprocessable_files.is_empty());
    }

    #[tokio::test]
    async fn test_upload_multiple_activities() {
        let mut seq = Sequence::new();
        let mut service = MockActivityService::new();
        let expected_id1 = ActivityId::new();
        let expected_id2 = ActivityId::new();
        let expected_id1_clone = expected_id1.clone();
        let expected_id2_clone = expected_id2.clone();

        service
            .expect_create_activity()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_| {
                Ok(Activity::new(
                    expected_id1_clone.clone(),
                    UserId::test_default(),
                    None,
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    Sport::Running,
                    ActivityStatistics::default(),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        service
            .expect_create_activity()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_| {
                Ok(Activity::new(
                    expected_id2_clone.clone(),
                    UserId::test_default(),
                    None,
                    ActivityStartTime::from_timestamp(2000).unwrap(),
                    Sport::Cycling,
                    ActivityStatistics::default(),
                    None,
                    None,
                    None,
                    None,
                ))
            });

        let metrics = MockTrainingService::test_default();
        let file_parser = MockFileParser::test_default();
        let state = AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        };

        let app = Router::new()
            .route("/test_upload", post(upload_activities))
            .route_layer(from_extractor::<DefaultUserExtractor>())
            .with_state(state);
        let server = TestServer::new(app).expect("unable to create test server");

        let file1_data = b"test fit file content 1".to_vec();
        let file2_data = b"test fit file content 2".to_vec();

        let response = server
            .post("/test_upload")
            .multipart(
                axum_test::multipart::MultipartForm::new()
                    .add_part(
                        "test1.fit".to_string(),
                        axum_test::multipart::Part::bytes(file1_data),
                    )
                    .add_part(
                        "test2.fit".to_string(),
                        axum_test::multipart::Part::bytes(file2_data),
                    ),
            )
            .await;

        response.assert_status(StatusCode::CREATED);
        let json: UploadActivitiesResponse = response.json();
        assert_eq!(json.created_ids.len(), 2);
        assert_eq!(json.created_ids[0], expected_id1.to_string());
        assert_eq!(json.created_ids[1], expected_id2.to_string());
        assert!(json.unprocessable_files.is_empty());
    }

    #[tokio::test]
    async fn test_upload_with_unprocessable_files() {
        use crate::domain::ports::CreateActivityError;

        let mut seq = Sequence::new();
        let mut service = MockActivityService::new();
        let expected_id = ActivityId::new();
        let expected_id_clone = expected_id.clone();

        service
            .expect_create_activity()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_| {
                Ok(Activity::new(
                    expected_id_clone.clone(),
                    UserId::test_default(),
                    None,
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    Sport::Running,
                    ActivityStatistics::default(),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        service
            .expect_create_activity()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Err(CreateActivityError::SimilarActivityExistsError));

        let metrics = MockTrainingService::test_default();
        let file_parser = MockFileParser::test_default();
        let state = AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        };

        let app = Router::new()
            .route("/test_upload", post(upload_activities))
            .route_layer(from_extractor::<DefaultUserExtractor>())
            .with_state(state);
        let server = TestServer::new(app).expect("unable to create test server");

        let file1_data = b"test fit file content 1".to_vec();
        let file2_data = b"test fit file content 2".to_vec();

        let response = server
            .post("/test_upload")
            .multipart(
                axum_test::multipart::MultipartForm::new()
                    .add_part(
                        "test1.fit".to_string(),
                        axum_test::multipart::Part::bytes(file1_data),
                    )
                    .add_part(
                        "test2.fit".to_string(),
                        axum_test::multipart::Part::bytes(file2_data),
                    ),
            )
            .await;

        response.assert_status(StatusCode::CREATED);
        let json: UploadActivitiesResponse = response.json();
        assert_eq!(json.created_ids.len(), 1);
        assert_eq!(json.created_ids[0], expected_id.to_string());
        assert_eq!(json.unprocessable_files.len(), 1);
        assert_eq!(json.unprocessable_files[0].0, "test2.fit");
        assert!(matches!(
            json.unprocessable_files[0].1,
            RejectionReason::DuplicatedActivity
        ));
    }

    #[tokio::test]
    async fn test_upload_unsupported_file_extension() {
        let service = MockActivityService::test_default();
        let metrics = MockTrainingService::test_default();
        let file_parser = MockFileParser::test_default();
        let state = AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        };

        let app = Router::new()
            .route("/test_upload", post(upload_activities))
            .route_layer(from_extractor::<DefaultUserExtractor>())
            .with_state(state);
        let server = TestServer::new(app).expect("unable to create test server");

        let file_data = b"test content".to_vec();

        let response = server
            .post("/test_upload")
            .multipart(axum_test::multipart::MultipartForm::new().add_part(
                "test.gpx".to_string(),
                axum_test::multipart::Part::bytes(file_data),
            ))
            .await;

        response.assert_status(StatusCode::CREATED);
        let json: UploadActivitiesResponse = response.json();
        assert!(json.created_ids.is_empty());
        assert_eq!(json.unprocessable_files.len(), 1);
        assert_eq!(json.unprocessable_files[0].0, "test.gpx");
        assert!(matches!(
            json.unprocessable_files[0].1,
            RejectionReason::UnsupportedFileExtension
        ));
    }

    #[test]
    fn test_extract_file_extension() {
        assert_eq!(extract_extension("toto.fit"), Some(SupportedExtension::FIT));
        assert_eq!(
            extract_extension("toto.fit.gz"),
            Some(SupportedExtension::FIT)
        );
        assert_eq!(extract_extension("toto.tcx"), Some(SupportedExtension::TCX));
        assert_eq!(
            extract_extension("toto.tcx.gz"),
            Some(SupportedExtension::TCX)
        );
        assert_eq!(extract_extension("toto"), None);
    }
}
