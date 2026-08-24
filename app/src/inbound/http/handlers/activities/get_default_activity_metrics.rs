use axum::{Extension, Json, response::IntoResponse};

use crate::{domain::models::activity::DEFAULT_METRICS, inbound::auth::AuthenticatedUser};

use super::activity_schema::PublicActivityWithTimeseries;

#[tracing::instrument(skip_all)]
pub async fn get_default_activity_metrics(
    Extension(_user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    Json(&DEFAULT_METRICS)
}
