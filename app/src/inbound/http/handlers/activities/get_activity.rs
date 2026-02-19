use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    domain::{
        models::activity::ActivityId,
        ports::{IActivityService, IPreferencesService, ITrainingService},
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::ParseFile,
    },
};

use super::activity_schema::PublicActivityWithTimeseries;

pub async fn get_activity<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(_user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Path(activity_id): Path<String>,
) -> Result<Json<PublicActivityWithTimeseries>, StatusCode> {
    let Ok(res) = state
        .activity_service
        .get_activity_with_timeseries(&ActivityId::from(&activity_id))
        .await
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    Ok(Json(PublicActivityWithTimeseries::from(&res)))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, vec};

    use axum::extract::Path;
    use chrono::{DateTime, FixedOffset};
    use mockall::predicate::eq;

    use crate::{
        domain::{
            models::{
                UserId,
                activity::{
                    ActiveTime, Activity, ActivityId, ActivityStartTime, ActivityStatistic,
                    ActivityStatistics, ActivityTimeseries, ActivityWithTimeseries, Sport,
                    Timeseries, TimeseriesActiveTime, TimeseriesMetric, TimeseriesTime,
                    TimeseriesValue,
                },
            },
            ports::GetActivityError,
            services::{
                activity::test_utils::MockActivityService,
                preferences::tests_utils::MockPreferencesService,
                training::test_utils::MockTrainingService,
            },
        },
        inbound::{
            http::{
                CookieConfig,
                auth::test_utils::MockUserService,
                handlers::activities::activity_schema::{
                    PublicActivity, PublicActivityTimeseries, PublicTimeseries,
                    PublicTimeseriesValue,
                },
            },
            parser::test_utils::MockFileParser,
        },
    };

    use super::{PublicActivityWithTimeseries, *};

    #[tokio::test]
    async fn test_get_activity_exists() {
        let target_id = "target_id".to_string();
        let mut service = MockActivityService::new();
        service
            .expect_get_activity_with_timeseries()
            .returning(|_| {
                Ok(ActivityWithTimeseries::new(
                    Activity::new(
                        ActivityId::from("target_id"),
                        UserId::test_default(),
                        None,
                        ActivityStartTime::new(
                            "2025-09-03T00:00:00Z"
                                .parse::<DateTime<FixedOffset>>()
                                .unwrap(),
                        ),
                        Sport::IndoorCycling,
                        ActivityStatistics::new(HashMap::from([(
                            ActivityStatistic::Duration,
                            1200.,
                        )])),
                        None,
                        None,
                        None,
                        None,
                    ),
                    ActivityTimeseries::new(
                        TimeseriesTime::new(vec![0, 1, 2]),
                        TimeseriesActiveTime::new(vec![
                            ActiveTime::Running(0),
                            ActiveTime::Running(1),
                            ActiveTime::Running(2),
                        ]),
                        vec![],
                        vec![Timeseries::new(
                            TimeseriesMetric::Power,
                            vec![
                                Some(TimeseriesValue::Int(120)),
                                None,
                                Some(TimeseriesValue::Int(130)),
                            ],
                        )],
                    )
                    .unwrap(),
                ))
            });
        let file_parser = MockFileParser::test_default();
        let metrics = MockTrainingService::test_default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(
            Extension(AuthenticatedUser::new(UserId::test_default())),
            state,
            path,
        )
        .await;
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(
            response.0,
            PublicActivityWithTimeseries {
                activity: PublicActivity {
                    id: target_id,
                    name: None,
                    sport: "IndoorCycling".to_string(),
                    sport_category: Some("Cycling".to_string()),
                    start_time: "2025-09-03T00:00:00Z"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                    rpe: None,
                    workout_type: None,
                    nutrition: None,
                    feedback: None,
                    statistics: HashMap::from([("Duration".to_string(), 1200.)]),
                },
                timeseries: PublicActivityTimeseries {
                    time: vec![0, 1, 2],
                    active_time: vec![Some(0), Some(1), Some(2)],
                    metrics: HashMap::from([(
                        "Power".to_string(),
                        PublicTimeseries {
                            unit: "W".to_string(),
                            values: vec![
                                Some(PublicTimeseriesValue::Int(120)),
                                None,
                                Some(PublicTimeseriesValue::Int(130))
                            ]
                        }
                    )]),
                    laps: vec![]
                }
            }
        );
    }

    #[tokio::test]
    async fn test_get_activity_does_not_exist() {
        let mut service = MockActivityService::new();
        service
            .expect_get_activity_with_timeseries()
            .with(eq(ActivityId::from("target_id")))
            .returning(|_| {
                Err(GetActivityError::ActivityDoesNotExist(ActivityId::from(
                    "target_id",
                )))
            });

        let file_parser = MockFileParser::test_default();
        let metrics = MockTrainingService::test_default();

        let state = axum::extract::State(AppState {
            activity_service: Arc::new(service),
            training_metrics_service: Arc::new(metrics),
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(MockUserService::new()),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        });
        let path = Path("target_id".to_string());

        let response = get_activity(
            Extension(AuthenticatedUser::new(UserId::test_default())),
            state,
            path,
        )
        .await;
        assert!(response.is_err());
        let response = response.unwrap_err();
        assert_eq!(response, StatusCode::NOT_FOUND);
    }
}
