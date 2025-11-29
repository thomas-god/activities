use std::collections::HashMap;

use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, FixedOffset, Local};
use derive_more::Constructor;
use serde::{Deserialize, Serialize, de};
use serde_json::json;

use crate::{
    domain::{
        models::{
            activity::{ActivityStatistic, TimeseriesMetric},
            training::{
                ActivityMetricSource, TrainingMetric, TrainingMetricDefinition,
                TrainingMetricScope, TrainingMetricValues, TrainingPeriodId,
            },
        },
        ports::{DateRange, IActivityService, IPreferencesService, ITrainingService},
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
            handlers::training::{
                types::ScopePayload,
                utils::{
                    GroupedMetricValues, MetricsDateRange, convert_metric_values,
                    fill_metric_values,
                },
            },
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<ResponseBodyItem>);

#[derive(Debug, Deserialize)]
#[serde(tag = "scope", rename_all = "lowercase")]
pub enum MetricsQuery {
    /// Query for global metrics only
    /// Example: ?scope=global&start=2025-01-01T00:00:00Z&end=2025-01-31T23:59:59Z
    Global {
        #[serde(flatten)]
        date_range: MetricsDateRange,
    },
    /// Query for training period metrics (includes both global and period-specific metrics)
    /// Example: ?scope=period&period_id=abc123&start=2025-01-01T00:00:00Z&end=2025-01-31T23:59:59Z
    Period {
        #[serde(flatten)]
        date_range: MetricsDateRange,
        period_id: String,
    },
}

impl MetricsQuery {
    pub fn scope(&self) -> TrainingMetricScope {
        match self {
            MetricsQuery::Global { .. } => TrainingMetricScope::Global,
            MetricsQuery::Period { period_id, .. } => {
                TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from(period_id.as_str()))
            }
        }
    }

    pub fn date_range(&self) -> &MetricsDateRange {
        match self {
            MetricsQuery::Global { date_range } => date_range,
            MetricsQuery::Period { date_range, .. } => date_range,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBodyItem {
    id: String,
    name: Option<String>,
    metric: String,
    unit: String,
    granularity: String,
    aggregate: String,
    sports: Vec<String>,
    values: GroupedMetricValues,
    group_by: Option<String>,
    scope: ScopePayload,
}

fn to_response_body_item(
    metric: (TrainingMetric, TrainingMetricValues),
    range: &MetricsDateRange,
) -> ResponseBodyItem {
    let (metric, metric_values) = metric;
    let definition = metric.definition();
    let values = fill_metric_values(definition.granularity(), metric_values, range);
    let (unit, values) = convert_metric_values(values, definition.source(), definition.aggregate());

    ResponseBodyItem {
        id: metric.id().to_string(),
        name: metric.name().as_ref().map(|n| n.as_str().to_string()),
        metric: format_source(definition.source()),
        unit: unit.to_string(),
        granularity: definition.granularity().to_string(),
        aggregate: definition.aggregate().to_string(),
        sports: definition
            .filters()
            .sports()
            .as_ref()
            .map(|sports| sports.iter().map(|sport| sport.to_string()).collect())
            .unwrap_or_default(),
        values,
        group_by: definition.group_by().as_ref().map(|g| format!("{:?}", g)),
        scope: metric.scope().into(),
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
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Query(query): Query<MetricsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = state
        .training_metrics_service
        .get_training_metrics_values(
            user.user(),
            &Some(DateRange::from(query.date_range())),
            &query.scope(),
        )
        .await;

    let body = ResponseBody(
        res.into_iter()
            .map(|metric| to_response_body_item(metric, query.date_range()))
            .collect(),
    );

    Ok(json!(body).to_string())
}

#[cfg(test)]
mod tests {
    use crate::domain::models::{
        activity::{ActivityStatistic, TimeseriesMetric},
        training::TimeseriesAggregate,
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

    #[test]
    fn test_metrics_query_scope_global_when_no_period_id() {
        let query = MetricsQuery::Global {
            date_range: MetricsDateRange {
                start: "2025-01-01T00:00:00+00:00".parse().unwrap(),
                end: None,
            },
        };

        assert_eq!(query.scope(), TrainingMetricScope::Global);
    }

    #[test]
    fn test_metrics_query_scope_period_when_period_id_provided() {
        let period_id = "test-period-123";
        let query = MetricsQuery::Period {
            date_range: MetricsDateRange {
                start: "2025-01-01T00:00:00+00:00".parse().unwrap(),
                end: None,
            },
            period_id: period_id.to_string(),
        };

        match query.scope() {
            TrainingMetricScope::TrainingPeriod(id) => {
                assert_eq!(id.to_string(), period_id);
            }
            TrainingMetricScope::Global => panic!("Expected TrainingPeriod scope"),
        }
    }
}
