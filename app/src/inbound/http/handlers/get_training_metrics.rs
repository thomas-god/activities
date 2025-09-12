use std::collections::HashMap;

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use serde_json::json;

use crate::{
    domain::{
        models::training_metrics::{
            TrainingMetricDefinition, TrainingMetricSource, TrainingMetricValues,
        },
        ports::{IActivityService, ITrainingMetricService},
    },
    inbound::{http::AppState, parser::ParseFile},
};

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

impl From<(TrainingMetricDefinition, TrainingMetricValues)> for ResponseBodyItem {
    fn from(value: (TrainingMetricDefinition, TrainingMetricValues)) -> Self {
        let (def, values) = value;
        Self {
            id: def.id().to_string(),
            metric: format_source(def.source()),
            granularity: def.granularity().to_string(),
            aggregate: def.aggregate().to_string(),
            values: values.as_hash_map(),
        }
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
) -> Result<impl IntoResponse, StatusCode> {
    let res = state.training_metrics_service.get_training_metrics().await;

    let body = ResponseBody(res.into_iter().map(ResponseBodyItem::from).collect());

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
