use axum::{
    extract::{Path, State},
    http::{StatusCode, header::SET_COOKIE},
    response::{AppendHeaders, IntoResponse},
};
use axum_extra::extract::cookie::Cookie;
use cookie::time::OffsetDateTime;

use crate::inbound::http::{
    AppState, AuthAppState, AuthLinkValidationResult,
    auth::{AuthToken, IUserService},
    handlers::auth::build_session_cookie,
};

pub async fn validate_login<UR: IUserService>(
    State(state): State<AuthAppState<UR>>,
    Path(auth_token): Path<String>,
) -> impl IntoResponse {
    let token = AuthToken::from(auth_token);

    match state.user_service.validate_auth_link(token).await {
        Ok(AuthLinkValidationResult::Success(session)) => {
            let Some(cookie) = build_session_cookie(&state.cookie_config, &session) else {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };

            let headers = AppendHeaders([(SET_COOKIE, cookie.encoded().to_string())]);
            (headers, StatusCode::OK).into_response()
        }
        Ok(AuthLinkValidationResult::Invalid) => StatusCode::UNAUTHORIZED.into_response(),
        Err(()) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
