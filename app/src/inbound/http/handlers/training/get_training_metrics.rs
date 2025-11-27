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
                TrainingMetricScope, TrainingMetricValues,
            },
        },
        ports::{DateRange, IActivityService, IPreferencesService, ITrainingService},
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
            handlers::training::utils::{
                GroupedMetricValues, MetricsDateRange, convert_metric_values, fill_metric_values,
            },
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<ResponseBodyItem>);

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
    Query(date_range): Query<MetricsDateRange>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = state
        .training_metrics_service
        .get_training_metrics_values(
            user.user(),
            &Some(DateRange::from(&date_range)),
            // TODO: takes value from request
            &TrainingMetricScope::Global,
        )
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
}
