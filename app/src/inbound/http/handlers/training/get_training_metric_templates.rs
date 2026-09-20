use itertools::Itertools;
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
            activity::{ActivityMetric, ToUnit, Unit},
            training::{
                HooperIndexSource, TrainingMetricAggregate, TrainingMetricSource,
                WeightAndNutritionSource,
            },
        },
        ports::{
            activity::IActivityService, preferences::IPreferencesService,
            training::ITrainingService,
        },
    },
    inbound::{
        auth::{AuthenticatedUser, email_based::IUserService},
        http::{AppState, handlers::training::types::APITrainingMetricSource},
        parser::ParseFile,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<TrainingMetricTemplateBody>);

#[derive(Debug, Clone, Copy, Serialize)]
pub enum TrainingMetricTemplateSource {
    Activity(ActivityMetric),
    HooperIndex(HooperIndexSource),
    WeightAndNutrition(WeightAndNutritionSource),
}

impl TrainingMetricTemplateSource {
    pub fn unit(&self) -> Unit {
        match self {
            Self::Activity(source) => source.unit(),
            Self::HooperIndex(source) => source.unit(),
            Self::WeightAndNutrition(source) => source.unit(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrainingMetricTemplate {
    display_name: String,
    source: TrainingMetricTemplateSource,
    aggregate: TrainingMetricAggregate,
    category: TrainingMetricTemplateCategory,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainingMetricTemplateBody {
    display_name: String,
    source: TrainingMetricTemplateSource,
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
    Feedback,
    Weight,
    Nutrition,
    Other,
}

impl From<&TrainingMetricTemplate> for TrainingMetricTemplateBody {
    fn from(value: &TrainingMetricTemplate) -> Self {
        Self {
            display_name: value.display_name.to_string(),
            source: value.source,
            unit: value.source.unit().to_string(),
            aggregate: value.aggregate,
            category: value.category,
        }
    }
}

static TRAINING_METRIC_TEMPLATES: LazyLock<Vec<TrainingMetricTemplate>> = LazyLock::new(|| {
    let mut templates = vec![];

    // Sum and average
    for metric in [
        ActivityMetric::ActiveDuration,
        ActivityMetric::Calories,
        ActivityMetric::Elevation,
        ActivityMetric::Distance,
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
                source: TrainingMetricTemplateSource::Activity(metric),
                aggregate,
                category: metric_category(&metric),
            });
        }
    }

    // Min metrics
    let aggregate = TrainingMetricAggregate::Min;
    for metric in [
        ActivityMetric::MinSpeed,
        ActivityMetric::MinPower,
        ActivityMetric::MinHeartRate,
        ActivityMetric::MinCadence,
        ActivityMetric::MinAltitude,
        ActivityMetric::MinPace,
    ] {
        templates.push(TrainingMetricTemplate {
            display_name: format_metric(&metric),
            source: TrainingMetricTemplateSource::Activity(metric),
            aggregate,
            category: metric_category(&metric),
        });
    }

    // Max metrics
    let aggregate = TrainingMetricAggregate::Max;
    for metric in [
        ActivityMetric::MaxSpeed,
        ActivityMetric::MaxPower,
        ActivityMetric::MaxHeartRate,
        ActivityMetric::MaxCadence,
        ActivityMetric::MaxAltitude,
        ActivityMetric::MaxPace,
    ] {
        templates.push(TrainingMetricTemplate {
            display_name: format_metric(&metric),
            source: TrainingMetricTemplateSource::Activity(metric),
            aggregate,
            category: metric_category(&metric),
        });
    }

    // Average metrics
    let aggregate = TrainingMetricAggregate::Average;
    for metric in [
        ActivityMetric::AvgSpeed,
        ActivityMetric::AvgPower,
        ActivityMetric::AvgHeartRate,
        ActivityMetric::AvgCadence,
        ActivityMetric::AvgAltitude,
        ActivityMetric::AvgPace,
    ] {
        templates.push(TrainingMetricTemplate {
            display_name: format_metric(&metric),
            source: TrainingMetricTemplateSource::Activity(metric),
            aggregate,
            category: metric_category(&metric),
        });
    }

    // Normalized power
    let metric = ActivityMetric::NormalizedPower;
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
            source: TrainingMetricTemplateSource::Activity(metric),
            aggregate,
            category: metric_category(&metric),
        });
    }

    // Number of activities
    let metric = ActivityMetric::NumberOfActivity;
    let aggregate = TrainingMetricAggregate::Sum;
    templates.push(TrainingMetricTemplate {
        display_name: format_metric(&metric),
        source: TrainingMetricTemplateSource::Activity(metric),
        aggregate,
        category: metric_category(&metric),
    });

    // Subjective measures/Hooper index
    let aggregate = TrainingMetricAggregate::Average;
    let metrics = [
        HooperIndexSource::Fatigue,
        HooperIndexSource::Sleep,
        HooperIndexSource::Stress,
        HooperIndexSource::Mood,
        HooperIndexSource::Pain,
    ];
    for source in metrics.iter() {
        templates.push(TrainingMetricTemplate {
            display_name: source.to_string(),
            source: TrainingMetricTemplateSource::HooperIndex(*source),
            aggregate,
            category: TrainingMetricTemplateCategory::Feedback,
        })
    }

    // Weight and nutrition metrics
    let metrics = [
        WeightAndNutritionSource::Weight,
        WeightAndNutritionSource::Fat,
        WeightAndNutritionSource::Muscle,
        WeightAndNutritionSource::BMI,
        WeightAndNutritionSource::Calories,
        WeightAndNutritionSource::Carbs,
        WeightAndNutritionSource::Lipid,
        WeightAndNutritionSource::Protein,
        WeightAndNutritionSource::Water,
        WeightAndNutritionSource::Alcohol,
    ];
    let aggregate = TrainingMetricAggregate::Average;
    for source in metrics.iter() {
        templates.push(TrainingMetricTemplate {
            display_name: weight_and_nutrition_format_source(source),
            source: TrainingMetricTemplateSource::WeightAndNutrition(*source),
            aggregate,
            category: weight_and_nutrition_category(source),
        })
    }
    templates
});

#[tracing::instrument(skip_all, err)]
pub async fn get_training_metric_templates() -> Result<impl IntoResponse, StatusCode> {
    let body = ResponseBody(
        TRAINING_METRIC_TEMPLATES
            .iter()
            .map(|template| template.into())
            .collect(),
    );

    Ok(serde_json::json!(body).to_string())
}

fn metric_category(metric: &ActivityMetric) -> TrainingMetricTemplateCategory {
    match metric {
        ActivityMetric::Duration | ActivityMetric::ActiveDuration => {
            TrainingMetricTemplateCategory::Duration
        }
        ActivityMetric::Elevation => TrainingMetricTemplateCategory::Elevation,
        ActivityMetric::Calories => TrainingMetricTemplateCategory::Calories,
        ActivityMetric::Distance => TrainingMetricTemplateCategory::Distance,
        ActivityMetric::MaxSpeed | ActivityMetric::MinSpeed | ActivityMetric::AvgSpeed => {
            TrainingMetricTemplateCategory::Speed
        }
        ActivityMetric::MaxHeartRate
        | ActivityMetric::MinHeartRate
        | ActivityMetric::AvgHeartRate => TrainingMetricTemplateCategory::HeartRate,
        ActivityMetric::MaxCadence | ActivityMetric::MinCadence | ActivityMetric::AvgCadence => {
            TrainingMetricTemplateCategory::Cadence
        }
        ActivityMetric::MaxAltitude | ActivityMetric::MinAltitude | ActivityMetric::AvgAltitude => {
            TrainingMetricTemplateCategory::Altitude
        }
        ActivityMetric::MaxPace | ActivityMetric::MinPace | ActivityMetric::AvgPace => {
            TrainingMetricTemplateCategory::Pace
        }
        ActivityMetric::MaxPower
        | ActivityMetric::MinPower
        | ActivityMetric::AvgPower
        | ActivityMetric::NormalizedPower => TrainingMetricTemplateCategory::Power,
        ActivityMetric::NumberOfActivity => TrainingMetricTemplateCategory::Other,
    }
}

fn weight_and_nutrition_format_source(source: &WeightAndNutritionSource) -> String {
    match source {
        WeightAndNutritionSource::Weight => String::from("Total weight"),
        source => source.to_string(),
    }
}

fn weight_and_nutrition_category(
    source: &WeightAndNutritionSource,
) -> TrainingMetricTemplateCategory {
    match source {
        WeightAndNutritionSource::Weight
        | WeightAndNutritionSource::Fat
        | WeightAndNutritionSource::Muscle
        | WeightAndNutritionSource::BMI => TrainingMetricTemplateCategory::Weight,
        WeightAndNutritionSource::Calories
        | WeightAndNutritionSource::Lipid
        | WeightAndNutritionSource::Carbs
        | WeightAndNutritionSource::Protein
        | WeightAndNutritionSource::Water
        | WeightAndNutritionSource::Alcohol => TrainingMetricTemplateCategory::Nutrition,
    }
}

fn format_metric(metric: &ActivityMetric) -> String {
    match metric {
        ActivityMetric::Duration => "duration",
        ActivityMetric::Calories => "calories",
        ActivityMetric::Elevation => "elevation",
        ActivityMetric::Distance => "distance",
        ActivityMetric::NormalizedPower => "normalized power",

        ActivityMetric::ActiveDuration => "active duration",

        ActivityMetric::MaxSpeed => "Maximum speed",
        ActivityMetric::MinSpeed => "Minimum speed",
        ActivityMetric::AvgSpeed => "Average speed",

        ActivityMetric::MaxPower => "Maximum power",
        ActivityMetric::MinPower => "Minimum power",
        ActivityMetric::AvgPower => "Average power",

        ActivityMetric::MaxHeartRate => "Maximum heart rate",
        ActivityMetric::MinHeartRate => "Minimum heart rate",
        ActivityMetric::AvgHeartRate => "Average heart rate",

        ActivityMetric::MaxCadence => "Maximum cadence",
        ActivityMetric::MinCadence => "Minimum cadence",
        ActivityMetric::AvgCadence => "Average cadence",

        ActivityMetric::MaxAltitude => "Maximum altitude",
        ActivityMetric::MinAltitude => "Minimum altitude",
        ActivityMetric::AvgAltitude => "Average altitude",

        ActivityMetric::MaxPace => "Maximum pace",
        ActivityMetric::MinPace => "Minimum pace",
        ActivityMetric::AvgPace => "Average pace",

        ActivityMetric::NumberOfActivity => "Number of activities",
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
