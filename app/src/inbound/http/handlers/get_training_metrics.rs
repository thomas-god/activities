use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    domain::{
        models::training_metrics::{
            TrainingMetricDefinition, TrainingMetricGranularity, TrainingMetricSource,
            TrainingMetricValues,
        },
        ports::{IActivityService, ITrainingMetricService},
    },
    inbound::{http::AppState, parser::ParseFile},
};

#[derive(Debug, Deserialize)]
pub struct MetricsDateRange {
    start: DateTime<FixedOffset>,
    end: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<ResponseBodyItem>);

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBodyItem {
    id: String,
    metric: String,
    granularity: String,
    aggregate: String,
    values: HashMap<String, f64>,
}

fn to_response_body_item(
    metric: (TrainingMetricDefinition, TrainingMetricValues),
    range: &MetricsDateRange,
) -> ResponseBodyItem {
    let (def, metric_values) = metric;
    let values = fill_metric_values(def.granularity(), metric_values, range);

    ResponseBodyItem {
        id: def.id().to_string(),
        metric: format_source(def.source()),
        granularity: def.granularity().to_string(),
        aggregate: def.aggregate().to_string(),
        values,
    }
}

fn fill_metric_values(
    granularity: &TrainingMetricGranularity,
    values: TrainingMetricValues,
    range: &MetricsDateRange,
) -> HashMap<String, f64> {
    match granularity.bins(
        &range.start,
        &range.end.unwrap_or(Local::now().fixed_offset()),
    ) {
        Some(bins) => HashMap::from_iter(
            bins.iter()
                .map(|bin| (bin.to_string(), values.get(bin).cloned().unwrap_or(0.))),
        ),
        None => values.as_hash_map(),
    }
}

fn format_source(source: &TrainingMetricSource) -> String {
    match source {
        TrainingMetricSource::Statistic(stat) => stat.to_string(),
        TrainingMetricSource::Timeseries((metric, aggregate)) => {
            format!("{aggregate:?} {metric:?}")
        }
    }
}

pub async fn get_training_metrics<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
>(
    State(state): State<AppState<AS, PF, TMS>>,
    Query(date_range): Query<MetricsDateRange>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = state.training_metrics_service.get_training_metrics().await;

    let body = ResponseBody(
        res.into_iter()
            .map(|metric| to_response_body_item(metric, &date_range))
            .collect(),
    );

    Ok(json!(body).to_string())
}

#[cfg(test)]
mod tests {
    use crate::domain::models::{
        activity::{ActivityStatistic, TimeseriesMetric},
        training_metrics::TrainingMetricAggregate,
    };

    use super::*;

    #[test]
    fn test_format_definition_source() {
        assert_eq!(
            format_source(&TrainingMetricSource::Statistic(
                ActivityStatistic::Calories
            )),
            "Calories".to_string()
        );
        assert_eq!(
            format_source(&TrainingMetricSource::Timeseries((
                TimeseriesMetric::Distance,
                TrainingMetricAggregate::Max
            ))),
            "Max Distance".to_string()
        );
    }
}
