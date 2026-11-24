use std::{io::Read, str::FromStr};

use anyhow::anyhow;
use axum::{
    Extension, Json,
    extract::{Multipart, Path, State, multipart::Field},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::NaiveDate;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::training::WeightAndNutritionPatch,
        ports::{
            DateRange,
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{
                ITrainingService, SaveBulkWeightAndNutritionRequest, SaveWeightAndNutritionRequest,
                WeightAndNutritionError,
            },
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{AppState, handlers::training::types::APIWeightAndNutritionPatch},
        parser::ParseFile,
    },
};

/// Body of the `PATCH /api/training/weight-and-nutrition/{date}` request.
///
/// Fields are optional and follow the patch semantics described on
/// [`APIWeightAndNutritionPatch`]: omitted fields are left untouched, `null` clears them and a
/// value sets them.
#[derive(Debug, Deserialize)]
pub struct UpdateWeightAndNutritionBody {
    #[serde(flatten)]
    patch: APIWeightAndNutritionPatch,
}

#[tracing::instrument(skip_all, err)]
pub async fn save_weight_and_nutrition<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(date): Path<String>,
    Json(payload): Json<UpdateWeightAndNutritionBody>,
) -> Result<StatusCode, StatusCode> {
    let date = NaiveDate::from_str(&date).map_err(|_| StatusCode::BAD_REQUEST)?;
    let patch = WeightAndNutritionPatch::from(payload.patch);
    let req = SaveWeightAndNutritionRequest::new(user.user().clone(), date, patch);

    match state
        .training_metrics_service
        .save_weight_and_nutrition(req)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            if matches!(&err, WeightAndNutritionError::Unknown(_)) {
                tracing::error!("Error updating weight and nutrition: {}", err);
            }
            Err(StatusCode::from(err))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum RejectionReason {
    CannotReadContent,
    CannotProcessFile,
    EmptyContent,
    UnsupportedFileExtension,
    Unknown,
}

#[derive(Serialize, Deserialize)]
struct BulkWeightAndNutritionResponse {
    unprocessable_files: Vec<(String, RejectionReason)>,
}

#[tracing::instrument(skip_all, err)]
pub async fn upload_bulk_weight_and_nutrition<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    mut multipart: Multipart,
) -> Result<impl axum::response::IntoResponse, StatusCode> {
    let mut unprocessable_files = Vec::new();
    let mut history = Vec::new();
    let mut range: Option<DateRange> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        let Some(name) = field.name().map(|n| n.to_string()) else {
            continue;
        };

        if !is_csv(&name) {
            unprocessable_files.push((name.to_string(), RejectionReason::UnsupportedFileExtension));
            continue;
        }

        let Ok(file_content) = extract_content(&name, field).await else {
            unprocessable_files.push((name.to_string(), RejectionReason::CannotReadContent));
            continue;
        };

        let Ok((mut values, new_range)) = parse_csv(file_content) else {
            unprocessable_files.push((name.to_string(), RejectionReason::CannotProcessFile));
            continue;
        };
        range = match range {
            None => Some(new_range),
            Some(range) => Some(range.union(new_range)),
        };
        history.append(&mut values);
    }

    let Some(range) = range else {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    };

    let req = SaveBulkWeightAndNutritionRequest::new(user.user().clone(), history, range);

    state
        .training_metrics_service
        .save_bulk_weight_and_nutrition(req)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(BulkWeightAndNutritionResponse {
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

fn is_csv(filename: &str) -> bool {
    let mut parts = filename.split('.').rev();
    let mut part = parts.next();

    if part.is_some_and(|ext| ext.eq_ignore_ascii_case("gz")) {
        part = parts.next();
    }

    part.is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
}

enum CSVHeader {
    Date,
    TotalWeight,
    Fat,
    Muscle,
    Calories,
    Lipid,
    Carbs,
    Protein,
    Water,
    Alcohol,
}

fn parse_csv(
    content: Vec<u8>,
) -> Result<(Vec<(chrono::NaiveDate, WeightAndNutritionPatch)>, DateRange), anyhow::Error> {
    let content = String::from_utf8(content).map_err(|err| anyhow!(err))?;
    let mut rows = content.split("\n");
    let headers = parse_headers(rows.next().ok_or(anyhow!("Empty file"))?)
        .ok_or(anyhow!("No valid header"))?;

    let mut values = vec![];
    let mut start: Option<chrono::NaiveDate> = None;
    let mut end: Option<chrono::NaiveDate> = None;

    for row in rows {
        let Some((date, patch)) = parse_row(row, &headers) else {
            continue;
        };
        start = start.map(|s| s.min(date)).or(Some(date));
        end = end.map(|s| s.max(date)).or(Some(date));
        values.push((date, patch))
    }

    let (Some(start), Some(end)) = (start, end) else {
        return Err(anyhow!("Invalid date range"));
    };

    Ok((values, DateRange::new(start, end)))
}

fn parse_headers(row: &str) -> Option<Vec<Option<CSVHeader>>> {
    let mut date_found = false;
    let mut headers = Vec::new();

    for col in row.split(",") {
        match col {
            "date" => {
                date_found = true;
                headers.push(Some(CSVHeader::Date));
            }
            "weight" => headers.push(Some(CSVHeader::TotalWeight)),
            "muscle" => headers.push(Some(CSVHeader::Muscle)),
            "fat" => headers.push(Some(CSVHeader::Fat)),
            "calories" => headers.push(Some(CSVHeader::Calories)),
            "lipid" => headers.push(Some(CSVHeader::Lipid)),
            "carbs" => headers.push(Some(CSVHeader::Carbs)),
            "protein" => headers.push(Some(CSVHeader::Protein)),
            "water" => headers.push(Some(CSVHeader::Water)),
            "alcohol" => headers.push(Some(CSVHeader::Alcohol)),
            _ => headers.push(None),
        }
    }

    if !date_found {
        return None;
    }

    Some(headers)
}

/// Tolerant date parsing: accepts a plain date, a datetime without timezone,
/// or a datetime with timezone (e.g. RFC 3339), and extracts the naive date.
fn parse_date(s: &str) -> Option<chrono::NaiveDate> {
    // chrono's FromStr implementations require the ISO 8601 'T' separator,
    // so normalize a single space separator (e.g. "2024-03-01 12:34:56") first.
    let s = s.trim().trim_matches('"').replacen(' ', "T", 1);
    s.parse::<chrono::NaiveDate>()
        .ok()
        .or_else(|| s.parse::<chrono::NaiveDateTime>().ok().map(|dt| dt.date()))
        .or_else(|| {
            s.parse::<chrono::DateTime<chrono::FixedOffset>>()
                .ok()
                .map(|dt| dt.date_naive())
        })
        .or_else(|| {
            s.parse::<chrono::DateTime<chrono::Utc>>()
                .ok()
                .map(|dt| dt.date_naive())
        })
}

fn parse_row(
    row: &str,
    headers: &[Option<CSVHeader>],
) -> Option<(chrono::NaiveDate, WeightAndNutritionPatch)> {
    let mut date: Option<chrono::NaiveDate> = None;
    let mut patch = WeightAndNutritionPatch::default();

    for (idx, col) in row.split(",").enumerate() {
        let Some(Some(header)) = headers.get(idx) else {
            continue;
        };
        match header {
            CSVHeader::Date => date = parse_date(col),
            // field absent or can't be parsed -> None
            // field present and can be parsed -> Some(Some()) as per the patch convention
            CSVHeader::TotalWeight => patch.weight = col.parse::<f32>().ok().map(Some),
            CSVHeader::Fat => patch.fat = col.parse::<f32>().ok().map(Some),
            CSVHeader::Muscle => patch.muscle = col.parse::<f32>().ok().map(Some),
            CSVHeader::Calories => patch.calories = col.parse::<f32>().ok().map(Some),
            CSVHeader::Lipid => patch.lipid = col.parse::<f32>().ok().map(Some),
            CSVHeader::Carbs => patch.carbs = col.parse::<f32>().ok().map(Some),
            CSVHeader::Protein => patch.protein = col.parse::<f32>().ok().map(Some),
            CSVHeader::Water => patch.water = col.parse::<f32>().ok().map(Some),
            CSVHeader::Alcohol => patch.alcohol = col.parse::<f32>().ok().map(Some),
        }
    }

    date.map(|date| (date, patch))
}

#[cfg(test)]
mod tests {
    use std::{io::Write, sync::Arc};

    use axum::{Router, middleware::from_extractor, routing::patch};
    use axum_test::TestServer;
    use chrono::NaiveDate;

    use crate::{
        domain::{
            models::UserId,
            services::{
                preferences::tests_utils::MockPreferencesService,
                training::test_utils::MockTrainingService,
            },
        },
        inbound::{auth::no_auth::DefaultUserExtractor, parser::test_utils::MockFileParser},
    };

    use crate::inbound::http::shared::PatchField;

    use super::*;

    #[test]
    fn test_parse_date_accepts_date_datetime_and_tz() {
        let expected = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();

        assert_eq!(parse_date("2024-03-01"), Some(expected));
        assert_eq!(parse_date("2024-03-01T12:34:56"), Some(expected));
        assert_eq!(parse_date("2024-03-01 12:34:56"), Some(expected));
        assert_eq!(parse_date("2024-03-01 12:34:56+02:00"), Some(expected));
        assert_eq!(parse_date("2024-03-01 12:34:56Z"), Some(expected));
        assert_eq!(parse_date("2024-03-01T12:34:56+02:00"), Some(expected));
        assert_eq!(parse_date("2024-03-01T12:34:56Z"), Some(expected));
        assert_eq!(parse_date("  2024-03-01  "), Some(expected));
        assert_eq!(parse_date("\"2024-03-01\""), Some(expected));
        assert_eq!(parse_date("\"2024-03-01 12:34:56+02:00\""), Some(expected));
        assert_eq!(parse_date("not a date"), None);
    }

    #[test]
    fn test_payload_distinguishes_absent_null_and_value() {
        let body: UpdateWeightAndNutritionBody =
            serde_json::from_str(r#"{ "weight": 70.5, "fat": null }"#).unwrap();

        assert_eq!(body.patch.weight, PatchField::Set(70.5));
        assert_eq!(body.patch.fat, PatchField::Clear);
        assert_eq!(body.patch.muscle, PatchField::Absent);
        assert_eq!(body.patch.calories, PatchField::Absent);
        assert_eq!(body.patch.alcohol, PatchField::Absent);
    }

    #[test]
    fn test_payload_accepts_empty_object() {
        let body: UpdateWeightAndNutritionBody = serde_json::from_str(r#"{}"#).unwrap();

        assert_eq!(body.patch, APIWeightAndNutritionPatch::default());
    }

    // --- is_csv ---

    #[test]
    fn test_is_csv_accepts_plain_csv() {
        assert!(is_csv("weight.csv"));
    }

    #[test]
    fn test_is_csv_accepts_gzipped_csv() {
        assert!(is_csv("weight.csv.gz"));
    }

    #[test]
    fn test_is_csv_rejects_gzip_only() {
        assert!(!is_csv("weight.gz"));
    }

    #[test]
    fn test_is_csv_rejects_other_extensions() {
        assert!(!is_csv("weight.txt"));
        assert!(!is_csv("weight.xlsx"));
        assert!(!is_csv("weight.csv.zip"));
    }

    #[test]
    fn test_is_csv_rejects_no_extension() {
        assert!(!is_csv("weight"));
    }

    #[test]
    fn test_is_csv_is_case_insensitive() {
        assert!(is_csv("weight.CSV"));
        assert!(is_csv("weight.CSV.GZ"));
        assert!(is_csv("weight.Csv.Gz"));
    }

    // --- parse_headers ---

    #[test]
    fn test_parse_headers_maps_all_known_columns() {
        let headers =
            parse_headers("date,weight,fat,muscle,calories,lipid,carbs,protein,water,alcohol")
                .unwrap();

        assert_eq!(headers.len(), 10);
        assert!(matches!(headers[0], Some(CSVHeader::Date)));
        assert!(matches!(headers[1], Some(CSVHeader::TotalWeight)));
        assert!(matches!(headers[2], Some(CSVHeader::Fat)));
        assert!(matches!(headers[3], Some(CSVHeader::Muscle)));
        assert!(matches!(headers[4], Some(CSVHeader::Calories)));
        assert!(matches!(headers[5], Some(CSVHeader::Lipid)));
        assert!(matches!(headers[6], Some(CSVHeader::Carbs)));
        assert!(matches!(headers[7], Some(CSVHeader::Protein)));
        assert!(matches!(headers[8], Some(CSVHeader::Water)));
        assert!(matches!(headers[9], Some(CSVHeader::Alcohol)));
    }

    #[test]
    fn test_parse_headers_ignores_unknown_columns() {
        let headers = parse_headers("date,foo,weight,bar").unwrap();

        assert_eq!(headers.len(), 4);
        assert!(matches!(headers[0], Some(CSVHeader::Date)));
        assert!(headers[1].is_none());
        assert!(matches!(headers[2], Some(CSVHeader::TotalWeight)));
        assert!(headers[3].is_none());
    }

    #[test]
    fn test_parse_headers_requires_date_column() {
        assert!(parse_headers("weight,fat").is_none());
        assert!(parse_headers("").is_none());
    }

    // --- parse_row ---

    #[test]
    fn test_parse_row_parses_all_fields() {
        let headers =
            parse_headers("date,weight,fat,muscle,calories,lipid,carbs,protein,water,alcohol")
                .unwrap();

        let (date, patch) = parse_row(
            "2024-01-15,70.5,15.2,55.1,2000,60,250,120,1.5,0.5",
            &headers,
        )
        .unwrap();

        assert_eq!(date, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(patch.weight, Some(Some(70.5)));
        assert_eq!(patch.fat, Some(Some(15.2)));
        assert_eq!(patch.muscle, Some(Some(55.1)));
        assert_eq!(patch.calories, Some(Some(2000.0)));
        assert_eq!(patch.lipid, Some(Some(60.0)));
        assert_eq!(patch.carbs, Some(Some(250.0)));
        assert_eq!(patch.protein, Some(Some(120.0)));
        assert_eq!(patch.water, Some(Some(1.5)));
        assert_eq!(patch.alcohol, Some(Some(0.5)));
    }

    #[test]
    fn test_parse_row_leaves_unlisted_fields_absent() {
        let headers = parse_headers("date,weight").unwrap();

        let (date, patch) = parse_row("2024-01-15,70.5", &headers).unwrap();

        assert_eq!(date, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(patch.weight, Some(Some(70.5)));
        assert_eq!(patch.fat, None);
        assert_eq!(patch.muscle, None);
        assert_eq!(patch.calories, None);
    }

    #[test]
    fn test_parse_row_treats_unparseable_value_as_absent() {
        let headers = parse_headers("date,weight,calories").unwrap();

        let (_, patch) = parse_row("2024-01-15,not-a-number,x", &headers).unwrap();

        assert_eq!(patch.weight, None);
        assert_eq!(patch.calories, None);
    }

    #[test]
    fn test_parse_row_with_unparseable_date_returns_none() {
        let headers = parse_headers("date,weight").unwrap();

        assert!(parse_row("15/01/2024,70.5", &headers).is_none());
        assert!(parse_row(",70.5", &headers).is_none());
    }

    #[test]
    fn test_parse_row_ignores_columns_beyond_headers() {
        let headers = parse_headers("date,weight").unwrap();

        let (date, patch) = parse_row("2024-01-15,70.5,extra,columns", &headers).unwrap();

        assert_eq!(date, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(patch.weight, Some(Some(70.5)));
        assert_eq!(patch.fat, None);
    }

    // --- parse_csv ---

    #[test]
    fn test_parse_csv_parses_rows_and_computes_range() {
        let content = b"date,weight\n\
2024-01-03,71.0\n\
2024-01-01,70.0\n\
2024-01-02,70.5\n"
            .to_vec();

        let (values, range) = parse_csv(content).unwrap();

        assert_eq!(values.len(), 3);
        assert_eq!(values[0].0, NaiveDate::from_ymd_opt(2024, 1, 3).unwrap());
        assert_eq!(values[0].1.weight, Some(Some(71.0)));
        assert_eq!(values[1].1.weight, Some(Some(70.0)));
        assert_eq!(values[2].1.weight, Some(Some(70.5)));
        assert_eq!(range.start(), &NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(range.end(), &NaiveDate::from_ymd_opt(2024, 1, 3).unwrap());
    }

    #[test]
    fn test_parse_csv_skips_invalid_rows() {
        let content = b"date,weight\n\
2024-01-01,70.0\n\
not-a-date,71.0\n\
\n"
        .to_vec();

        let (values, range) = parse_csv(content).unwrap();

        assert_eq!(values.len(), 1);
        assert_eq!(values[0].0, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(range.start(), &NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(range.end(), &NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    }

    #[test]
    fn test_parse_csv_errors_on_empty_content() {
        assert!(parse_csv(Vec::new()).is_err());
        assert!(parse_csv(b"\n2024-01-01,70.0\n".to_vec()).is_err());
    }

    #[test]
    fn test_parse_csv_errors_when_no_date_column() {
        let content = b"weight,fat\n70.0,15.0\n".to_vec();

        assert!(parse_csv(content).is_err());
    }

    #[test]
    fn test_parse_csv_errors_on_headers_only() {
        let content = b"date,weight\n".to_vec();

        assert!(parse_csv(content).is_err());
    }

    #[test]
    fn test_parse_csv_errors_on_invalid_utf8() {
        let content = vec![0xff, 0xfe, b'x'];

        assert!(parse_csv(content).is_err());
    }

    // --- upload_bulk_weight_and_nutrition handler ---

    fn test_server(metrics: MockTrainingService) -> TestServer {
        let state = AppState {
            activity_service: Arc::new(
                crate::domain::services::activity::test_utils::MockActivityService::new(),
            ),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(MockFileParser::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        let app = Router::new()
            .route("/test_upload_bulk", patch(upload_bulk_weight_and_nutrition))
            .route_layer(from_extractor::<DefaultUserExtractor>())
            .with_state(state);
        TestServer::new(app)
    }

    fn multipart_form(parts: Vec<(&str, Vec<u8>)>) -> axum_test::multipart::MultipartForm {
        let mut form = axum_test::multipart::MultipartForm::new();
        for (name, content) in parts {
            form = form.add_part(name.to_string(), axum_test::multipart::Part::bytes(content));
        }
        form
    }

    #[tokio::test]
    async fn test_upload_bulk_happy_path() {
        let expected_user = UserId::default();
        let expected_dates = [
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        ];

        let mut metrics = MockTrainingService::new();
        metrics
            .expect_save_bulk_weight_and_nutrition()
            .times(1)
            .withf(move |req: &SaveBulkWeightAndNutritionRequest| {
                req.user() == &expected_user
                    && req.values().len() == 2
                    && req.values()[0].0 == expected_dates[0]
                    && req.values()[0].1.weight == Some(Some(70.0))
                    && req.values()[1].0 == expected_dates[1]
                    && req.values()[1].1.weight == Some(Some(70.5))
                    && req.range().start() == &expected_dates[0]
                    && req.range().end() == &expected_dates[1]
            })
            .returning(|_| Ok(()));

        let server = test_server(metrics);
        let response = server
            .patch("/test_upload_bulk")
            .multipart(multipart_form(vec![(
                "weight.csv",
                b"date,weight\n2024-01-01,70.0\n2024-01-02,70.5\n".to_vec(),
            )]))
            .await;

        response.assert_status(StatusCode::CREATED);
        let json: BulkWeightAndNutritionResponse = response.json();
        assert!(json.unprocessable_files.is_empty());
    }

    #[tokio::test]
    async fn test_upload_bulk_merges_ranges_across_files() {
        let expected_user = UserId::default();
        let d1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();

        let mut metrics = MockTrainingService::new();
        metrics
            .expect_save_bulk_weight_and_nutrition()
            .times(1)
            .withf(move |req: &SaveBulkWeightAndNutritionRequest| {
                req.user() == &expected_user
                    && req.values().len() == 3
                    && req.range().start() == &d1
                    && req.range().end() == &d2
            })
            .returning(|_| Ok(()));

        let server = test_server(metrics);
        let response = server
            .patch("/test_upload_bulk")
            .multipart(multipart_form(vec![
                (
                    "january.csv",
                    b"date,weight\n2024-01-01,70.0\n2024-01-02,70.5\n".to_vec(),
                ),
                ("february.csv", b"date,weight\n2024-01-05,71.0\n".to_vec()),
            ]))
            .await;

        response.assert_status(StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_upload_bulk_reports_unprocessable_files() {
        let expected_user = UserId::default();

        let mut metrics = MockTrainingService::new();
        metrics
            .expect_save_bulk_weight_and_nutrition()
            .times(1)
            .withf(move |req: &SaveBulkWeightAndNutritionRequest| {
                req.user() == &expected_user && req.values().len() == 1
            })
            .returning(|_| Ok(()));

        let server = test_server(metrics);
        let response = server
            .patch("/test_upload_bulk")
            .multipart(multipart_form(vec![
                ("weight.csv", b"date,weight\n2024-01-01,70.0\n".to_vec()),
                ("notes.txt", b"not a csv".to_vec()),
                ("archive.zip", b"PK".to_vec()),
            ]))
            .await;

        response.assert_status(StatusCode::CREATED);
        let json: BulkWeightAndNutritionResponse = response.json();
        assert_eq!(json.unprocessable_files.len(), 2);
        assert_eq!(json.unprocessable_files[0].0, "notes.txt");
        assert!(matches!(
            json.unprocessable_files[0].1,
            RejectionReason::UnsupportedFileExtension
        ));
        assert_eq!(json.unprocessable_files[1].0, "archive.zip");
        assert!(matches!(
            json.unprocessable_files[1].1,
            RejectionReason::UnsupportedFileExtension
        ));
    }

    #[tokio::test]
    async fn test_upload_bulk_reports_unprocessable_gz_file() {
        let mut csv_content = Vec::new();
        flate2::write::GzEncoder::new(&mut csv_content, flate2::Compression::default())
            .write_all(b"date,weight\n2024-01-01,70.0\n")
            .unwrap();

        let mut metrics = MockTrainingService::new();
        metrics
            .expect_save_bulk_weight_and_nutrition()
            .times(1)
            .withf(|req: &SaveBulkWeightAndNutritionRequest| {
                req.values().len() == 1
                    && req.values()[0].0 == NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
            })
            .returning(|_| Ok(()));

        let server = test_server(metrics);
        let response = server
            .patch("/test_upload_bulk")
            .multipart(multipart_form(vec![
                ("weight.csv.gz", csv_content),
                ("broken.csv.gz", b"this is not gzip".to_vec()),
            ]))
            .await;

        response.assert_status(StatusCode::CREATED);
        let json: BulkWeightAndNutritionResponse = response.json();
        assert_eq!(json.unprocessable_files.len(), 1);
        assert_eq!(json.unprocessable_files[0].0, "broken.csv.gz");
        assert!(matches!(
            json.unprocessable_files[0].1,
            RejectionReason::CannotReadContent
        ));
    }

    #[tokio::test]
    async fn test_upload_bulk_rejects_malformed_csv() {
        let mut metrics = MockTrainingService::new();
        metrics.expect_save_bulk_weight_and_nutrition().times(0);

        let server = test_server(metrics);
        let response = server
            .patch("/test_upload_bulk")
            .multipart(multipart_form(vec![(
                "weight.csv",
                b"weight,fat\n70.0,15.0\n".to_vec(),
            )]))
            .await;

        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_upload_bulk_rejects_empty_body() {
        let mut metrics = MockTrainingService::new();
        metrics.expect_save_bulk_weight_and_nutrition().times(0);

        let server = test_server(metrics);
        let response = server
            .patch("/test_upload_bulk")
            .multipart(multipart_form(vec![]))
            .await;

        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_upload_bulk_maps_service_error_to_422() {
        let mut metrics = MockTrainingService::new();
        metrics
            .expect_save_bulk_weight_and_nutrition()
            .times(1)
            .returning(|_| {
                Err(WeightAndNutritionError::Unknown(anyhow!(
                    "database exploded"
                )))
            });

        let server = test_server(metrics);
        let response = server
            .patch("/test_upload_bulk")
            .multipart(multipart_form(vec![(
                "weight.csv",
                b"date,weight\n2024-01-01,70.0\n".to_vec(),
            )]))
            .await;

        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }
}
