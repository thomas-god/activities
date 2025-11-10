use std::collections::HashMap;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::training::{TrainingMetricBin, TrainingMetricId},
        ports::{
            DateRange, GetTrainingMetricValuesError, IActivityService, IPreferencesService,
            ITrainingService,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct MetricValuesQuery {
    start: DateTime<FixedOffset>,
    end: Option<DateTime<FixedOffset>>,
}

impl From<&MetricValuesQuery> for DateRange {
    fn from(value: &MetricValuesQuery) -> Self {
        let start_date = value.start.date_naive();
        let end_date = value
            .end
            .map(|e| e.date_naive())
            .unwrap_or_else(|| Local::now().date_naive());
        Self::new(start_date, end_date)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody {
    values: HashMap<String, MetricValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricValue {
    value: f64,
    group: Option<String>,
}

impl From<GetTrainingMetricValuesError> for StatusCode {
    fn from(value: GetTrainingMetricValuesError) -> Self {
        match value {
            GetTrainingMetricValuesError::TrainingMetricDoesNotExists(_) => Self::NOT_FOUND,
            GetTrainingMetricValuesError::Unknown(_) => Self::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn get_training_metric_values<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Path(metric_id): Path<String>,
    Query(query): Query<MetricValuesQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let metric_id = TrainingMetricId::from(&metric_id);
    let date_range = DateRange::from(&query);

    let values = state
        .training_metrics_service
        .get_training_metric_values(user.user(), &metric_id, &date_range)
        .await
        .map_err(StatusCode::from)?;

    let response_values: HashMap<String, MetricValue> = values
        .into_iter()
        .map(|(bin, value)| {
            (
                bin.granule().to_string(),
                MetricValue {
                    value: value.value(),
                    group: bin.group().clone(),
                },
            )
        })
        .collect();

    Ok(Json(ResponseBody {
        values: response_values,
    }))
}
