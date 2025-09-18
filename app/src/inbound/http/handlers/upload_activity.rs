use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;

use crate::{
    domain::ports::{CreateActivityError, IActivityService, ITrainingMetricService},
    inbound::{http::AppState, parser::ParseFile},
};

impl From<CreateActivityError> for StatusCode {
    fn from(_value: CreateActivityError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

#[derive(Serialize)]
struct UnprocessableFilesResponse {
    unprocessable_files: Vec<String>,
}

pub async fn upload_activities<AS: IActivityService, PF: ParseFile, TMS: ITrainingMetricService>(
    State(state): State<AppState<AS, PF, TMS>>,
    mut multipart: Multipart,
) -> Result<impl axum::response::IntoResponse, StatusCode> {
    let mut unprocessable_files = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let Some(name) = field.name().map(|n| n.to_string()) else {
            continue;
        };
        let Ok(file_content) = field.bytes().await else {
            unprocessable_files.push(name.to_string());
            continue;
        };
        let Ok(create_activity_request) = state
            .file_parser
            .try_bytes_into_domain(file_content.to_vec())
        else {
            unprocessable_files.push(name.to_string());
            continue;
        };

        if let Err(_err) = state
            .activity_service
            .create_activity(create_activity_request)
            .await
        {
            unprocessable_files.push(name.to_string());
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

// [TODO]: Need to refactor how services are mocked to extensively test this handler
// #[cfg(test)]
// mod tests {
//     use std::sync::{Arc, Mutex};

//     use axum::{Router, routing::post};
//     use axum_test::TestServer;

//     use crate::{
//         domain::services::test_utils::{MockActivityService, MockTrainingMetricsService},
//         inbound::parser::{ParseCreateActivityHttpRequestBodyError, test_utils::MockFileParser},
//     };

//     use super::*;

//     #[tokio::test]
//     async fn test_upload_single_activity() {
//         let service = MockActivityService::default();
//         let metrics = MockTrainingMetricsService::default();
//         let file_parser = MockFileParser::default();
//         let state = AppState {
//             activity_service: Arc::new(service),
//             training_metrics_service: Arc::new(metrics),
//             file_parser: Arc::new(file_parser),
//         };

//         let app = Router::new()
//             .route("/test_upload", post(upload_activities))
//             .with_state(state);
//         let server = TestServer::new(app).expect("unable to create test server");

//         let file1_data = b"test fit file content 1".to_vec();
//         // let file2_data = b"test fit file content 2".to_vec();

//         let response = server
//             .post("/test_upload")
//             .multipart(
//                 axum_test::multipart::MultipartForm::new().add_part(
//                     "test1.fit".to_string(),
//                     axum_test::multipart::Part::bytes(file1_data),
//                 ), // .add_part(
//                    //     "test2.fit".to_string(),
//                    //     axum_test::multipart::Part::bytes(file2_data),
//                    // ),
//             )
//             .await;

//         response.assert_status(StatusCode::CREATED);
//         assert!(response.as_bytes().is_empty());
//     }

//     #[tokio::test]
//     async fn test_upload_multiple_activities() {
//         let service = MockActivityService::default();
//         let metrics = MockTrainingMetricsService::default();
//         let file_parser = MockFileParser::default();
//         let state = AppState {
//             activity_service: Arc::new(service),
//             training_metrics_service: Arc::new(metrics),
//             file_parser: Arc::new(file_parser),
//         };

//         let app = Router::new()
//             .route("/test_upload", post(upload_activities))
//             .with_state(state);
//         let server = TestServer::new(app).expect("unable to create test server");

//         let file1_data = b"test fit file content 1".to_vec();
//         let file2_data = b"test fit file content 2".to_vec();

//         let response = server
//             .post("/test_upload")
//             .multipart(
//                 axum_test::multipart::MultipartForm::new()
//                     .add_part(
//                         "test1.fit".to_string(),
//                         axum_test::multipart::Part::bytes(file1_data),
//                     )
//                     .add_part(
//                         "test2.fit".to_string(),
//                         axum_test::multipart::Part::bytes(file2_data),
//                     ),
//             )
//             .await;

//         response.assert_status(StatusCode::CREATED);
//         dbg!(response.text());
//         assert!(response.as_bytes().is_empty());
//     }

//     // #[tokio::test]
//     // async fn test_upload_activities_contains_non_fit_files() {
//     //     let service = MockActivityService::default();
//     //     let metrics = MockTrainingMetricsService::default();
//     //     let file_parser = MockFileParser {
//     //         try_into_domain_result: Arc::new(Mutex::new(Err(
//     //             ParseCreateActivityHttpRequestBodyError::InvalidFitContent,
//     //         ))),
//     //     };
//     //     let state = AppState {
//     //         activity_service: Arc::new(service),
//     //         training_metrics_service: Arc::new(metrics),
//     //         file_parser: Arc::new(file_parser),
//     //     };

//     //     let app = Router::new()
//     //         .route("/test_upload", post(upload_activities))
//     //         .with_state(state);
//     //     let server = TestServer::new(app).expect("unable to create test server");

//     //     let file1_data = b"test fit file content 1".to_vec();
//     //     let file2_data = b"test fit file content 2".to_vec();

//     //     let response = server
//     //         .post("/test_upload")
//     //         .multipart(
//     //             axum_test::multipart::MultipartForm::new()
//     //                 .add_part(
//     //                     "test1.fit".to_string(),
//     //                     axum_test::multipart::Part::bytes(file1_data),
//     //                 )
//     //                 .add_part(
//     //                     "test2.fit".to_string(),
//     //                     axum_test::multipart::Part::bytes(file2_data),
//     //                 ),
//     //         )
//     //         .await;

//     //     response.assert_status(StatusCode::CREATED);
//     //     let json: serde_json::Value = response.json();
//     //     assert!(json.is_array())
//     // }

//     // #[tokio::test]
//     // async fn test_create_activity_with_similar_already_exists() {
//     //     let content = vec![1, 2, 3];
//     //     let service = MockActivityService {
//     //         create_activity_result: Arc::new(Mutex::new(Err(
//     //             CreateActivityError::SimilarActivityExistsError,
//     //         ))),
//     //         ..Default::default()
//     //     };
//     //     let metrics = MockTrainingMetricsService::default();

//     //     let file_parser = MockFileParser::default();

//     //     let state = axum::extract::State(AppState {
//     //         activity_service: Arc::new(service),
//     //         training_metrics_service: Arc::new(metrics),
//     //         file_parser: Arc::new(file_parser),
//     //     });
//     //     let bytes = axum::body::Bytes::from(content);

//     //     let response = upload_activities(state, bytes).await;
//     //     assert!(response.is_err());
//     //     assert_eq!(response.unwrap_err(), StatusCode::CONFLICT)
//     // }
// }
