use std::{borrow::Cow, sync::LazyLock};

use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;

use crate::{
    domain::{
        models::{
            activity::{ActivityMetricV2, ToUnit, Unit},
            training::TrainingMetricAggregate,
        },
        ports::{
            activity::IActivityService, preferences::IPreferencesService,
            training::ITrainingService,
        },
    },
    inbound::{
        http::{AppState, IUserService, auth::AuthenticatedUser},
        parser::ParseFile,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<TrainingMetricTemplateBody>);

#[derive(Debug, Clone, Serialize)]
pub struct TrainingMetricTemplate {
    display_name: String,
    metric: ActivityMetricV2,
    aggregate: TrainingMetricAggregate,
    category: TrainingMetricTemplateCategory,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainingMetricTemplateBody {
    display_name: String,
    metric: ActivityMetricV2,
    unit: String,
    aggregate: TrainingMetricAggregate,
    category: TrainingMetricTemplateCategory,
}

#[derive(Debug, Clone, Serialize, Copy)]
pub enum TrainingMetricTemplateCategory {
    Duration,
    Calories,
    Elevation,
    Distance,
    Speed,
    Power,
    HeartRate,
    Cadence,
    Altitude,
    Pace,
    Other,
}

impl From<&TrainingMetricTemplate> for TrainingMetricTemplateBody {
    fn from(value: &TrainingMetricTemplate) -> Self {
        Self {
            display_name: value.display_name.to_string(),
            metric: value.metric.clone(),
            unit: value.metric.source().unit().to_string(),
            aggregate: value.aggregate.clone(),
            category: value.category.clone(),
        }
    }
}

static TRAINING_METRIC_TEMPLATES: LazyLock<Vec<TrainingMetricTemplate>> = LazyLock::new(|| {
    let mut templates = vec![];

    // Sum and average
    for metric in [
        ActivityMetricV2::ActiveDuration,
        ActivityMetricV2::Calories,
        ActivityMetricV2::Elevation,
        ActivityMetricV2::Distance,
    ] {
        for aggregate in [
            TrainingMetricAggregate::Sum,
            TrainingMetricAggregate::Average,
        ] {
            templates.push(TrainingMetricTemplate {
                display_name: format!(
                    "{} {}",
                    format_aggregate(&aggregate),
                    format_metric(&metric)
                ),
                metric,
                aggregate,
                category: metric_category(&metric),
            });
        }
    }

    // Min metrics
    let aggregate = TrainingMetricAggregate::Min;
    for metric in [
        ActivityMetricV2::MinSpeed,
        ActivityMetricV2::MinPower,
        ActivityMetricV2::MinHeartRate,
        ActivityMetricV2::MinCadence,
        ActivityMetricV2::MinAltitude,
        ActivityMetricV2::MinPace,
    ] {
        templates.push(TrainingMetricTemplate {
            display_name: format!("{}", format_metric(&metric)),
            metric,
            aggregate,
            category: metric_category(&metric),
        });
    }

    // Max metrics
    let aggregate = TrainingMetricAggregate::Max;
    for metric in [
        ActivityMetricV2::MaxSpeed,
        ActivityMetricV2::MaxPower,
        ActivityMetricV2::MaxHeartRate,
        ActivityMetricV2::MaxCadence,
        ActivityMetricV2::MaxAltitude,
        ActivityMetricV2::MaxPace,
    ] {
        templates.push(TrainingMetricTemplate {
            display_name: format!("{}", format_metric(&metric)),
            metric,
            aggregate,
            category: metric_category(&metric),
        });
    }

    // Average metrics
    let aggregate = TrainingMetricAggregate::Average;
    for metric in [
        ActivityMetricV2::AvgSpeed,
        ActivityMetricV2::AvgPower,
        ActivityMetricV2::AvgHeartRate,
        ActivityMetricV2::AvgCadence,
        ActivityMetricV2::AvgAltitude,
        ActivityMetricV2::AvgPace,
    ] {
        templates.push(TrainingMetricTemplate {
            display_name: format!("{}", format_metric(&metric)),
            metric,
            aggregate,
            category: metric_category(&metric),
        });
    }

    // Normalized power
    let metric = ActivityMetricV2::NormalizedPower;
    for aggregate in [
        TrainingMetricAggregate::Min,
        TrainingMetricAggregate::Max,
        TrainingMetricAggregate::Average,
    ] {
        templates.push(TrainingMetricTemplate {
            display_name: format!(
                "{} {}",
                format_aggregate(&aggregate),
                format_metric(&metric)
            ),
            metric,
            aggregate,
            category: metric_category(&metric),
        });
    }

    // Number of activities
    let metric = ActivityMetricV2::NumberOfActivity;
    let aggregate = TrainingMetricAggregate::Sum;
    templates.push(TrainingMetricTemplate {
        display_name: format!("{}", format_metric(&metric)),
        metric,
        aggregate,
        category: metric_category(&metric),
    });

    templates
});

pub async fn get_training_metric_templates() -> Result<impl IntoResponse, StatusCode> {
    let body = ResponseBody(
        TRAINING_METRIC_TEMPLATES
            .iter()
            .map(|template| template.into())
            .collect(),
    );

    Ok(serde_json::json!(body).to_string())
}

fn metric_category(metric: &ActivityMetricV2) -> TrainingMetricTemplateCategory {
    match metric {
        ActivityMetricV2::Duration | ActivityMetricV2::ActiveDuration => {
            TrainingMetricTemplateCategory::Duration
        }
        ActivityMetricV2::Elevation => TrainingMetricTemplateCategory::Elevation,
        ActivityMetricV2::Calories => TrainingMetricTemplateCategory::Calories,
        ActivityMetricV2::Distance => TrainingMetricTemplateCategory::Distance,
        ActivityMetricV2::MaxSpeed | ActivityMetricV2::MinSpeed | ActivityMetricV2::AvgSpeed => {
            TrainingMetricTemplateCategory::Speed
        }
        ActivityMetricV2::MaxHeartRate
        | ActivityMetricV2::MinHeartRate
        | ActivityMetricV2::AvgHeartRate => TrainingMetricTemplateCategory::HeartRate,
        ActivityMetricV2::MaxCadence
        | ActivityMetricV2::MinCadence
        | ActivityMetricV2::AvgCadence => TrainingMetricTemplateCategory::Cadence,
        ActivityMetricV2::MaxAltitude
        | ActivityMetricV2::MinAltitude
        | ActivityMetricV2::AvgAltitude => TrainingMetricTemplateCategory::Altitude,
        ActivityMetricV2::MaxPace | ActivityMetricV2::MinPace | ActivityMetricV2::AvgPace => {
            TrainingMetricTemplateCategory::Pace
        }
        ActivityMetricV2::MaxPower
        | ActivityMetricV2::MinPower
        | ActivityMetricV2::AvgPower
        | ActivityMetricV2::NormalizedPower => TrainingMetricTemplateCategory::Power,
        ActivityMetricV2::NumberOfActivity => TrainingMetricTemplateCategory::Other,
    }
}

fn format_metric(metric: &ActivityMetricV2) -> String {
    match metric {
        ActivityMetricV2::Duration => "duration",
        ActivityMetricV2::Calories => "calories",
        ActivityMetricV2::Elevation => "elevation",
        ActivityMetricV2::Distance => "distance",
        ActivityMetricV2::NormalizedPower => "normalized power",

        ActivityMetricV2::ActiveDuration => "active duration",

        ActivityMetricV2::MaxSpeed => "Maximum speed",
        ActivityMetricV2::MinSpeed => "Minimum speed",
        ActivityMetricV2::AvgSpeed => "Average speed",

        ActivityMetricV2::MaxPower => "Maximum power",
        ActivityMetricV2::MinPower => "Minimum power",
        ActivityMetricV2::AvgPower => "Average power",

        ActivityMetricV2::MaxHeartRate => "Maximum heart rate",
        ActivityMetricV2::MinHeartRate => "Minimum heart rate",
        ActivityMetricV2::AvgHeartRate => "Average heart rate",

        ActivityMetricV2::MaxCadence => "Maximum cadence",
        ActivityMetricV2::MinCadence => "Minimum cadence",
        ActivityMetricV2::AvgCadence => "Average cadence",

        ActivityMetricV2::MaxAltitude => "Maximum altitude",
        ActivityMetricV2::MinAltitude => "Minimum altitude",
        ActivityMetricV2::AvgAltitude => "Average altitude",

        ActivityMetricV2::MaxPace => "Maximum pace",
        ActivityMetricV2::MinPace => "Minimum pace",
        ActivityMetricV2::AvgPace => "Average pace",

        ActivityMetricV2::NumberOfActivity => "Number of activities",
    }
    .to_string()
}

fn format_aggregate(aggregate: &TrainingMetricAggregate) -> String {
    match aggregate {
        TrainingMetricAggregate::Average => "Average",
        TrainingMetricAggregate::Max => "Maximum",
        TrainingMetricAggregate::Min => "Minimum",
        TrainingMetricAggregate::Sum => "Total",
        TrainingMetricAggregate::NumberOfActivities => "Number of activities",
    }
    .to_string()
}
