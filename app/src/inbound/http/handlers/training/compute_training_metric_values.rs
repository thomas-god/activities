use std::collections::HashMap;

use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::training::{TrainingMetricDefinition, TrainingMetricFilters},
        ports::{
            ComputeTrainingMetricValuesError, DateRange, IActivityService, IPreferencesService,
            ITrainingService,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
            handlers::training::types::{
                APITrainingMetricAggregate, APITrainingMetricFilters, APITrainingMetricGranularity,
                APITrainingMetricGroupBy, APITrainingMetricSource,
            },
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct ComputeMetricValuesQuery {
    source: APITrainingMetricSource,
    granularity: APITrainingMetricGranularity,
    aggregate: APITrainingMetricAggregate,
    #[serde(default)]
    filters: Option<APITrainingMetricFilters>,
    #[serde(default)]
    group_by: Option<APITrainingMetricGroupBy>,
    start: DateTime<FixedOffset>,
    end: Option<DateTime<FixedOffset>>,
}

impl From<&ComputeMetricValuesQuery> for DateRange {
    fn from(value: &ComputeMetricValuesQuery) -> Self {
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

impl From<ComputeTrainingMetricValuesError> for StatusCode {
    fn from(value: ComputeTrainingMetricValuesError) -> Self {
        match value {
            ComputeTrainingMetricValuesError::Unknown(_) => Self::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn compute_training_metric_values<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Query(query): Query<ComputeMetricValuesQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let date_range = DateRange::from(&query);

    // Build the training metric definition from the query parameters
    let filters = query
        .filters
        .map(TrainingMetricFilters::from)
        .unwrap_or_else(TrainingMetricFilters::empty);

    let group_by = query.group_by.map(|gb| gb.into());

    let definition = TrainingMetricDefinition::new(
        user.user().clone(),
        query.source.into(),
        query.granularity.into(),
        query.aggregate.into(),
        filters,
        group_by,
    );

    let values = state
        .training_metrics_service
        .compute_training_metric_values(&definition, &date_range)
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
