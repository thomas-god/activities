use axum::{
    Router,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

use crate::domain::models::UserId;

use crate::inbound::auth::AuthenticatedUser;

/// Dummy extractor that always returns the default [UserId], regardless of the request.
pub struct DefaultUserExtractor;

impl<S> FromRequestParts<S> for DefaultUserExtractor
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::new(UserId::default());

        parts.extensions.insert(user);

        Ok(Self)
    }
}

pub fn no_auth_login_routes<S>(base_router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    base_router.route_layer(axum::middleware::from_extractor::<DefaultUserExtractor>())
}

#[cfg(test)]
mod test {
    use axum::{Extension, Router, middleware::from_extractor, routing::get};
    use axum_test::TestServer;

    use super::*;

    #[tokio::test]
    async fn test_default_user_extractor() {
        async fn test_route(
            Extension(user): Extension<AuthenticatedUser>,
        ) -> impl axum::response::IntoResponse {
            user.user().to_string()
        }

        let app = Router::new()
            .route("/", get(test_route))
            .route_layer(from_extractor::<DefaultUserExtractor>());
        let server = TestServer::new(app).expect("unable to create test server");

        let response = server.get("/").await;
        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), UserId::default().to_string());
    }
}
