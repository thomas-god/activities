use axum::{
    extract::{Path, State},
    http::{StatusCode, header::SET_COOKIE},
    response::{AppendHeaders, IntoResponse},
};
use axum_extra::extract::cookie::Cookie;
use cookie::time::OffsetDateTime;

use crate::{
    domain::ports::{IActivityService, ITrainingService},
    inbound::{
        http::{
            AppState, MagicLinkValidationResult,
            auth::{IUserService, MagicToken},
        },
        parser::ParseFile,
    },
};

pub async fn validate_login<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Path(magic_token): Path<String>,
) -> impl IntoResponse {
    let token = MagicToken::from(magic_token);

    match state.user_service.validate_magic_link(token).await {
        Ok(MagicLinkValidationResult::Success(session)) => {
            let Ok(expire_at) =
                OffsetDateTime::from_unix_timestamp(session.expire_at().timestamp())
            else {
                tracing::warn!(
                    "Unable to build cookie expire_at from datetime: {:?}",
                    session.expire_at()
                );
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };

            let mut builder = Cookie::build(("session_token", session.token().to_string()))
                .expires(expire_at)
                .secure(state.cookie_config.secure)
                .http_only(state.cookie_config.http_only)
                .same_site(state.cookie_config.same_site)
                .path("/");
            if let Some(domain) = state.cookie_config.domain.clone() {
                builder = builder.domain(domain);
            }
            let cookie = builder.build();

            let headers = AppendHeaders([(SET_COOKIE, cookie.encoded().to_string())]);
            (headers, StatusCode::OK).into_response()
        }
        Ok(MagicLinkValidationResult::Invalid) => StatusCode::UNAUTHORIZED.into_response(),
        Err(()) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
