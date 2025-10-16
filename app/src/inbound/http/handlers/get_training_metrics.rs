use std::collections::HashMap;

use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize, de};
use serde_json::json;

use crate::{
    domain::{
        models::{
            activity::{ActivityStatistic, TimeseriesMetric, ToUnit, Unit},
            training::{
                ActivityMetricSource, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricValues,
            },
        },
        ports::{IActivityService, ITrainingService},
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
    unit: String,
    granularity: String,
    aggregate: String,
    sports: Vec<String>,
    values: HashMap<String, f64>,
}

fn to_response_body_item(
    metric: (TrainingMetricDefinition, TrainingMetricValues),
    range: &MetricsDateRange,
) -> ResponseBodyItem {
    let (def, metric_values) = metric;
    let values = fill_metric_values(def.granularity(), metric_values, range);
    let (unit, values) = convert_metric_values(values, def.source());

    ResponseBodyItem {
        id: def.id().to_string(),
        metric: format_source(def.source()),
        unit: unit.to_string(),
        granularity: def.granularity().to_string(),
        aggregate: def.aggregate().to_string(),
        sports: def
            .filters()
            .sports()
            .as_ref()
            .map(|sports| sports.iter().map(|sport| sport.to_string()).collect())
            .unwrap_or_default(),
        values,
    }
}

fn fill_metric_values(
    granularity: &TrainingMetricGranularity,
    values: TrainingMetricValues,
    range: &MetricsDateRange,
) -> HashMap<String, f64> {
    let bins = granularity.bins_keys(
        &range.start,
        &range.end.unwrap_or(Local::now().fixed_offset()),
    );
    HashMap::from_iter(bins.iter().map(|bin| {
        (
            bin.to_string(),
            values
                .get(bin)
                .cloned()
                .map(|value| *value.value())
                .unwrap_or(0.),
        )
    }))
}

fn convert_metric_values(
    values: HashMap<String, f64>,
    source: &ActivityMetricSource,
) -> (Unit, HashMap<String, f64>) {
    match source {
        ActivityMetricSource::Statistic(stat) => match stat {
            ActivityStatistic::Distance => (
                Unit::Kilometer,
                values
                    .iter()
                    .map(|(k, val)| (k.clone(), *val / 1000.))
                    .collect(),
            ),
            _ => (stat.unit(), values),
        },
        ActivityMetricSource::Timeseries((metric, _)) => match metric {
            TimeseriesMetric::Distance => (
                Unit::Kilometer,
                values
                    .iter()
                    .map(|(k, val)| (k.clone(), *val / 1000.))
                    .collect(),
            ),
            TimeseriesMetric::Speed => (
                Unit::KilometerPerHour,
                values
                    .iter()
                    .map(|(k, val)| (k.clone(), *val * 3.6))
                    .collect(),
            ),
            _ => (metric.unit(), values),
        },
    }
}

fn format_source(source: &ActivityMetricSource) -> String {
    match source {
        ActivityMetricSource::Statistic(stat) => stat.to_string(),
        ActivityMetricSource::Timeseries((metric, aggregate)) => {
            format!("{aggregate:?} {metric:?}")
        }
    }
}

pub async fn get_training_metrics<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Query(date_range): Query<MetricsDateRange>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = state
        .training_metrics_service
        .get_training_metrics(user.user())
        .await;

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
        training::{TimeseriesAggregate, TrainingMetricAggregate},
    };

    use super::*;

    #[test]
    fn test_format_definition_source() {
        assert_eq!(
            format_source(&ActivityMetricSource::Statistic(
                ActivityStatistic::Calories
            )),
            "Calories".to_string()
        );
        assert_eq!(
            format_source(&ActivityMetricSource::Timeseries((
                TimeseriesMetric::Distance,
                TimeseriesAggregate::Max
            ))),
            "Max Distance".to_string()
        );
    }
}
