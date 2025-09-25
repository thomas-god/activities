use std::io::Read;

use anyhow::anyhow;
use axum::{
    Extension, Json,
    extract::{Multipart, State, multipart::Field},
    http::StatusCode,
    response::IntoResponse,
};
use flate2::read::GzDecoder;
use serde::Serialize;

use crate::{
    domain::ports::{CreateActivityError, IActivityService, ITrainingMetricService},
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::ParseFile,
    },
};

impl From<CreateActivityError> for StatusCode {
    fn from(_value: CreateActivityError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

#[derive(Serialize)]
struct UnprocessableFilesResponse {
    unprocessable_files: Vec<(String, RejectionReason)>,
}

#[derive(Debug, Serialize)]
enum RejectionReason {
    CannotReadContent,
    CannotProcessFile,
    DuplicatedActivity,
    IncoherentTimeseries,
    Unknown,
}

impl From<CreateActivityError> for RejectionReason {
    fn from(value: CreateActivityError) -> Self {
        match value {
            CreateActivityError::SimilarActivityExistsError => Self::DuplicatedActivity,
            CreateActivityError::TimeseriesMetricsNotSameLength => Self::IncoherentTimeseries,
            _ => Self::Unknown,
        }
    }
}

pub async fn upload_activities<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    mut multipart: Multipart,
) -> Result<impl axum::response::IntoResponse, StatusCode> {
    let mut unprocessable_files = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let Some(name) = field.name().map(|n| n.to_string()) else {
            continue;
        };
        let Ok(file_content) = extract_content(&name, field).await else {
            unprocessable_files.push((name.to_string(), RejectionReason::CannotReadContent));
            continue;
        };
        let Ok(file_content) = state.file_parser.try_bytes_into_domain(file_content) else {
            unprocessable_files.push((name.to_string(), RejectionReason::CannotProcessFile));
            continue;
        };
        let create_activity_request = file_content.into_request(user.user());

        if let Err(err) = state
            .activity_service
            .create_activity(create_activity_request)
            .await
        {
            unprocessable_files.push((name.to_string(), err.into()));
        }
    }

    if unprocessable_files.is_empty() {
        Ok(StatusCode::CREATED.into_response())
    } else {
        // Return JSON with failed files
        Ok((
            StatusCode::CREATED,
            Json(UnprocessableFilesResponse {
                unprocessable_files,
            }),
        )
            .into_response())
    }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{Router, middleware::from_extractor, routing::post};
    use axum_test::TestServer;

    use crate::{
        domain::services::{
            activity::test_utils::MockActivityService,
            training_metrics::test_utils::MockTrainingMetricService,
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
        let service = MockActivityService::test_default();
        let metrics = MockTrainingMetricService::test_default();
        let file_parser = MockFileParser::test_default();
        let state = AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
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
        assert!(response.as_bytes().is_empty());
    }

    #[tokio::test]
    async fn test_upload_multiple_activities() {
        let service = MockActivityService::test_default();
        let metrics = MockTrainingMetricService::test_default();
        let file_parser = MockFileParser::test_default();
        let state = AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
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
        assert!(response.as_bytes().is_empty());
    }
}
