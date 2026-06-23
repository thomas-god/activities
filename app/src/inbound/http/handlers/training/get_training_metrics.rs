use std::collections::HashMap;

use axum::{
    Extension,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, FixedOffset, Local};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    domain::{
        models::{
            activity::{ActivityMetricSource, ActivityStatistic, TimeseriesMetric},
            training::{
                SportFilter, TrainingMetric, TrainingMetricDefinition, TrainingMetricScope,
                TrainingMetricSummaryAverage, TrainingMetricValues, TrainingPeriodId,
                TrainingPeriodSports,
            },
        },
        ports::{
            DateRange,
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{GetTrainingMetricValuesError, ITrainingService},
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{
            AppState,
            handlers::training::{
                types::ScopePayload,
                utils::{
                    GranuleValues, GroupedMetricValues, MetricsDateRange,
                    convert_metric_values_unit, fill_missing_granules, group_metric_values,
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

impl From<GetTrainingMetricValuesError> for StatusCode {
    fn from(value: GetTrainingMetricValuesError) -> Self {
        match value {
            GetTrainingMetricValuesError::TrainingMetricDoesNotExist(_) => Self::NOT_FOUND,
            GetTrainingMetricValuesError::TrainingPeriodDoesNotExist(_) => Self::NOT_FOUND,
            GetTrainingMetricValuesError::Unknown(_) => Self::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseSports {
    categories: Vec<String>,
    sports: Vec<String>,
}

impl From<&Option<Vec<SportFilter>>> for ResponseSports {
    fn from(value: &Option<Vec<SportFilter>>) -> Self {
        let Some(items) = value else {
            return Self {
                categories: vec![],
                sports: vec![],
            };
        };

        let mut sports = Vec::new();
        let mut categories = Vec::new();

        for sport in items {
            match sport {
                SportFilter::Sport(sport) => sports.push(sport.to_string()),
                SportFilter::SportCategory(category) => categories.push(category.to_string()),
            }
        }

        Self { categories, sports }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBodyItem {
    id: String,
    name: Option<String>,
    metric: String,
    metric_formated: String,
    unit: String,
    granularity: Option<String>,
    aggregate: Option<String>,
    sports: ResponseSports,
    workout_types: Option<Vec<String>>,
    bonked: Option<String>,
    rpes: Option<Vec<u8>>,
    show_average: Option<TrainingMetricSummaryAverage>,
    values: HashMap<String, GranuleValues>,
    group_by: Option<String>,
    scope: ScopePayload,
    summary: HashMap<String, f64>,
}

fn to_response_body_item(
    metric: (TrainingMetric, TrainingMetricValues),
    range: &MetricsDateRange,
) -> ResponseBodyItem {
    let (metric, metric_values) = metric;
    let definition = metric.definition();
    let values = convert_metric_values_unit(group_metric_values(metric_values));
    let values = match metric.definition().window() {
        Some(window) => fill_missing_granules(values, window, range),
        None => values,
    };
    let unit = values.unit();
    let (values, summary) = values.values_and_summary();

    ResponseBodyItem {
        id: metric.id().to_string(),
        name: metric.name().as_ref().map(|n| n.as_str().to_string()),
        metric: metric.definition().metric().to_string(),
        metric_formated: format_source_metric(&definition.metric().source()),
        unit: unit.to_string(),
        granularity: definition
            .window()
            .as_ref()
            .map(|w| w.granularity().to_string()),
        aggregate: definition
            .window()
            .as_ref()
            .map(|w| w.aggregate().to_string()),
        sports: ResponseSports::from(definition.filters().sports()),
        workout_types: definition
            .filters()
            .workout_types()
            .as_ref()
            .map(|types| types.iter().map(|wt| wt.to_string()).collect()),
        bonked: definition
            .filters()
            .bonked()
            .as_ref()
            .map(|status| status.to_string()),
        rpes: definition
            .filters()
            .rpes()
            .as_ref()
            .map(|rpes| rpes.iter().map(|rpe| rpe.value()).collect()),
        show_average: definition.summary().average().clone(),
        values: values,
        group_by: definition
            .window()
            .as_ref()
            .and_then(|w| w.group_by().as_ref().map(|g| format!("{:?}", g))),
        scope: metric.scope().into(),
        summary,
    }
}

fn format_source_metric(source: &ActivityMetricSource) -> String {
    match source {
        ActivityMetricSource::Statistic(stat) => stat.to_string(),
        ActivityMetricSource::Timeseries((metric, aggregate)) => {
            format!("Activity {aggregate:?} {metric:?}")
        }
        ActivityMetricSource::ActiveDuration => "ActiveDuration".into(),
        ActivityMetricSource::NumberOfActivities => "Number of activities".into(),
    }
}

pub async fn get_training_metrics<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Query(query): Query<MetricsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = state
        .training_metrics_service
        .get_training_metrics_values(
            user.user(),
            &DateRange::from(query.date_range()),
            &query.scope(),
        )
        .await?;

    let body = ResponseBody(
        res.into_iter()
            .map(|metric| to_response_body_item(metric, query.date_range()))
            .collect(),
    );

    Ok(json!(body).to_string())
}

/// Get all training metrics for a specific training period.
///
/// Returns all computed metrics that fall within the date range of the specified training period.
/// For open-ended periods (no end date), includes today's activities.
pub async fn get_training_period_metrics<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Path(period_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let period_id = TrainingPeriodId::from(&period_id);

    let period = state
        .training_metrics_service
        .get_training_period(user.user(), &period_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let date_range_domain = period.range_default_tomorrow();
    let date_range = MetricsDateRange {
        start: date_range_domain
            .start()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
            .fixed_offset(),
        end: Some(
            date_range_domain
                .end()
                .and_hms_opt(23, 59, 59)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap()
                .fixed_offset(),
        ),
    };

    let res = state
        .training_metrics_service
        .get_training_period_metrics_values(user.user(), &period_id)
        .await?;

    let body = ResponseBody(
        res.into_iter()
            .map(|metric| to_response_body_item(metric, &date_range))
            .collect(),
    );

    Ok(json!(body).to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::domain::models::activity::{
        ActivityStatistic, TimeseriesAggregate, TimeseriesMetric,
    };

    use super::*;

    #[test]
    fn test_format_definition_source() {
        assert_eq!(
            format_source_metric(&ActivityMetricSource::Statistic(
                ActivityStatistic::Calories
            )),
            "Calories".to_string()
        );
        assert_eq!(
            format_source_metric(&ActivityMetricSource::Timeseries((
                TimeseriesMetric::Distance,
                TimeseriesAggregate::Max
            ))),
            "Activity Max Distance".to_string()
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

    #[test]
    fn test_response_body_shape() {
        let body = ResponseBody(vec![ResponseBodyItem {
            id: "metric-id-1".to_string(),
            name: Some("My Metric".to_string()),
            metric: "Calories".to_string(),
            metric_formated: "Activity average calories".to_string(),
            unit: "kcal".to_string(),
            granularity: Some("Daily".to_string()),
            aggregate: Some("Average".to_string()),
            sports: ResponseSports {
                sports: vec!["TrailRunning".to_string()],
                categories: vec!["Cycling".to_string()],
            },
            workout_types: Some(vec!["tempo".to_string()]),
            bonked: Some("bonked".to_string()),
            rpes: Some(vec![1, 2]),
            show_average: Some(TrainingMetricSummaryAverage::new(false)),
            values: HashMap::from([(
                "Running".to_string(),
                HashMap::from([("2025-09-24".to_string(), 10.5)]),
            )]),
            group_by: Some("Sport".to_string()),
            scope: ScopePayload::TrainingPeriod {
                training_period_id: "period-1".to_string(),
            },
            summary: HashMap::new(),
        }]);

        let serialized = serde_json::to_value(body).unwrap();

        assert_eq!(
            serialized,
            json!([
                {
                    "id": "metric-id-1",
                    "name": "My Metric",
                    "metric": "Calories",
                    "metric_formated": "Activity average calories",
                    "unit": "kcal",
                    "granularity": "Daily",
                    "aggregate": "Average",
                    "sports": {
                        "sports": ["TrailRunning"],
                        "categories": ["Cycling"]
                    },
                    "workout_types": ["tempo"],
                    "bonked": "bonked",
                    "rpes": [1, 2],
                    "show_average": {"include_zeros": false},
                    "values": {
                        "Running": {
                            "2025-09-24": 10.5
                        }
                    },
                    "group_by": "Sport",
                    "scope": {
                        "type": "trainingPeriod",
                        "trainingPeriodId": "period-1"
                    },
                    "summary": {}
                }
            ])
        );
    }
}
