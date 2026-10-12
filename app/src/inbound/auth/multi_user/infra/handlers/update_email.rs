use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;

use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::auth::multi_user::{
    AuthToken, EmailAddress, EmailChangeValidationResult, IMultiUserService, UpdateEmailRequest,
    UpdateEmailResult, infra::handlers::AuthAppState,
};

#[derive(Debug, Deserialize)]
pub struct UpdateEmailBody {
    new_email: String,
}

#[tracing::instrument(skip_all)]
pub async fn update_email<UR: IMultiUserService>(
    State(state): State<AuthAppState<UR>>,
    axum::Extension(user): Extension<AuthenticatedUser>,
    Json(body): Json<UpdateEmailBody>,
) -> impl IntoResponse {
    let Ok(new_email) = EmailAddress::try_from(body.new_email) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    match state
        .user_service
        .update_email(UpdateEmailRequest::new(user.user().clone(), new_email))
        .await
    {
        UpdateEmailResult::Changed => StatusCode::OK.into_response(),
        UpdateEmailResult::VerificationEmailSent => StatusCode::ACCEPTED.into_response(),
        UpdateEmailResult::Retry => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ValidateEmailChangeBody {
    token: String,
}

/// Validates an email change verification link. Unauthenticated on purpose: the user may open
/// the link from a different device than the one that requested the change. Possessing a valid
/// token proves control of the target address, so an email conflict is reported explicitly here
/// (unlike the request step).
// TODO: reconsider whether an email change should invalidate existing sessions once session
// revocation is implemented.
#[tracing::instrument(skip_all)]
pub async fn validate_email_change<UR: IMultiUserService>(
    State(state): State<AuthAppState<UR>>,
    Json(body): Json<ValidateEmailChangeBody>,
) -> impl IntoResponse {
    let token = AuthToken::from(body.token);

    match state.user_service.validate_email_change(token).await {
        Ok(EmailChangeValidationResult::Success(new_email)) => {
            Json(json!({ "email": new_email.value() })).into_response()
        }
        Ok(EmailChangeValidationResult::EmailAlreadyInUse) => StatusCode::CONFLICT.into_response(),
        Ok(EmailChangeValidationResult::Invalid) => StatusCode::UNAUTHORIZED.into_response(),
        Err(()) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
