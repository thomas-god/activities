//! Admin HTTP surface: shared state, authorization extractor, error mapping and routes.

mod create_user;
mod list_users;
mod revoke_user_sessions;
mod set_admin;
mod set_user_password;

use std::{marker::PhantomData, sync::Arc, time::Duration};

use axum::{
    Router,
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
    middleware::from_fn_with_state,
    routing::{get, post},
};

use crate::{
    domain::models::UserId,
    inbound::{
        auth::{
            AuthenticatedUser,
            multi_user::{
                IMultiUserService,
                admin::{AdminError, IAdminService},
            },
        },
        http::{
            CookieConfig,
            middlewares::rate_limit::{IpRateLimitLayer, RateLimitStore},
        },
    },
};

use super::cookie_auth_middleware;

pub use create_user::create_user;
pub use list_users::list_users;
pub use revoke_user_sessions::revoke_user_sessions;
pub use set_admin::set_admin;
pub use set_user_password::set_user_password;

/// State shared by the admin routes: the user service used by the cookie middleware and the admin
/// service backing the handlers.
#[derive(Debug, Clone)]
pub struct AdminAppState<US: IMultiUserService, AS: IAdminService> {
    user_service: Arc<US>,
    admin_service: Arc<AS>,
    cookie_config: Arc<CookieConfig>,
}

impl<US: IMultiUserService, AS: IAdminService> FromRef<AdminAppState<US, AS>> for Arc<US> {
    fn from_ref(input: &AdminAppState<US, AS>) -> Self {
        input.user_service.clone()
    }
}

/// Wrapper around the admin service so `FromRef` can tell it apart from the user service in
/// [`AdminAppState`] (both are behind an `Arc`, and the two type parameters are not provably
/// distinct, so plain `Arc<AS>` impls would conflict with `Arc<US>`).
#[derive(Debug, Clone)]
pub struct AdminServiceHandle<AS: IAdminService>(Arc<AS>);

impl<AS: IAdminService> AdminServiceHandle<AS> {
    pub fn service(&self) -> &AS {
        &self.0
    }
}

impl<US: IMultiUserService, AS: IAdminService> FromRef<AdminAppState<US, AS>>
    for AdminServiceHandle<AS>
{
    fn from_ref(input: &AdminAppState<US, AS>) -> Self {
        Self(input.admin_service.clone())
    }
}

impl<US: IMultiUserService, AS: IAdminService> FromRef<AdminAppState<US, AS>>
    for Arc<CookieConfig>
{
    fn from_ref(input: &AdminAppState<US, AS>) -> Self {
        input.cookie_config.clone()
    }
}

/// Extractor that admits the request only when the authenticated user is an admin.
///
/// Rejects with `401` without a session, `403` for an authenticated non-admin, and `503` when the
/// admin check itself fails.
pub struct AdminGuard<AS>(UserId, PhantomData<AS>);

impl<AS> AdminGuard<AS> {
    pub fn user(&self) -> &UserId {
        &self.0
    }
}

impl<S, AS> FromRequestParts<S> for AdminGuard<AS>
where
    S: Send + Sync,
    AS: IAdminService,
    AdminServiceHandle<AS>: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Some(user) = parts.extensions.get::<AuthenticatedUser>() else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        let admin_service = AdminServiceHandle::<AS>::from_ref(state);
        match admin_service.service().is_admin(user.user().clone()).await {
            Ok(true) => Ok(Self(user.user().clone(), PhantomData)),
            Ok(false) => Err(StatusCode::FORBIDDEN),
            Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    }
}

/// Maps an [`AdminError`] to the HTTP status returned to the admin.
pub(crate) fn admin_error_status(err: AdminError) -> StatusCode {
    match err {
        AdminError::InvalidEmail | AdminError::EmptyPassword => StatusCode::BAD_REQUEST,
        AdminError::HashingFailed | AdminError::Storage => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// Builds the `/api/admin` router, mounted by the HTTP server under `/api`.
pub fn admin_routes<US, AS>(user_service: US, admin_service: AS) -> Router
where
    US: IMultiUserService,
    AS: IAdminService,
{
    let state = AdminAppState {
        user_service: Arc::new(user_service),
        admin_service: Arc::new(admin_service),
        cookie_config: Arc::new(CookieConfig::default()),
    };

    Router::new()
        .route(
            "/admin/users",
            get(list_users::<AS>).post(create_user::<AS>),
        )
        .route(
            "/admin/users/{user_id}/password",
            post(set_user_password::<AS>),
        )
        .route("/admin/users/{user_id}/admin", post(set_admin::<AS>))
        .route(
            "/admin/users/{user_id}/sessions/revoke",
            post(revoke_user_sessions::<AS>),
        )
        .route_layer(from_fn_with_state(
            state.clone(),
            cookie_auth_middleware::<US, AdminAppState<US, AS>>,
        ))
        .route_layer(IpRateLimitLayer::new(
            RateLimitStore::new(),
            30,
            Duration::from_secs(60),
        ))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::{
        Router, middleware,
        routing::{get, post},
    };
    use axum_test::TestServer;
    use mockall::mock;
    use serde_json::json;

    use crate::inbound::auth::multi_user::{
        EmailAddress, Password,
        admin::{
            AdminUser, CreateUserOutcome, IAdminService, SetAdminOutcome, SetUserPasswordOutcome,
        },
        test_utils::MockUserService,
    };

    use super::*;

    mock! {
        AdminService {}

        impl Clone for AdminService {
            fn clone(&self) -> Self;
        }

        impl IAdminService for AdminService {
            async fn is_admin(&self, user: UserId) -> Result<bool, AdminError>;

            async fn list_users(&self) -> Result<Vec<AdminUser>, AdminError>;

            async fn create_user(
                &self,
                email: EmailAddress,
                password: Password,
            ) -> Result<CreateUserOutcome, AdminError>;

            async fn set_user_password(
                &self,
                user: UserId,
                password: Password,
            ) -> Result<SetUserPasswordOutcome, AdminError>;

            async fn set_admin(
                &self,
                user: UserId,
                is_admin: bool,
            ) -> Result<SetAdminOutcome, AdminError>;

            async fn revoke_user_sessions(&self, user: UserId) -> Result<u64, AdminError>;
        }
    }

    /// Injects an authenticated user so handler tests can focus on routing and status mapping,
    /// independently of the cookie middleware (covered by `extractor.rs`).
    async fn authed_user_middleware(
        mut request: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request
            .extensions_mut()
            .insert(AuthenticatedUser::new(UserId::test_default()));
        next.run(request).await
    }

    fn build_server(admin_service: MockAdminService) -> TestServer {
        let state = AdminAppState {
            user_service: Arc::new(MockUserService::new()),
            admin_service: Arc::new(admin_service),
            cookie_config: Arc::new(CookieConfig::default()),
        };

        let app = Router::new()
            .route("/admin/users", get(list_users::<MockAdminService>))
            .route("/admin/users", post(create_user::<MockAdminService>))
            .route(
                "/admin/users/{user_id}/password",
                post(set_user_password::<MockAdminService>),
            )
            .route(
                "/admin/users/{user_id}/admin",
                post(set_admin::<MockAdminService>),
            )
            .route(
                "/admin/users/{user_id}/sessions/revoke",
                post(revoke_user_sessions::<MockAdminService>),
            )
            .layer(middleware::from_fn(authed_user_middleware))
            .with_state(state);

        TestServer::new(app)
    }

    fn admin_user() -> AdminUser {
        AdminUser::new(
            UserId::from("user_1"),
            Some("user@mail.test".to_string()),
            true,
            false,
            true,
            false,
        )
    }

    #[tokio::test]
    async fn test_list_users_returns_users_to_admin() {
        let mut admin = MockAdminService::new();
        admin.expect_is_admin().returning(|_| Ok(true));
        admin
            .expect_list_users()
            .times(1)
            .returning(|| Ok(vec![admin_user()]));

        let response = build_server(admin).get("/admin/users").await;

        response.assert_status(StatusCode::OK);
        response.assert_json(&json!([{
            "user_id": "user_1",
            "email": "user@mail.test",
            "email_valid": true,
            "verified": false,
            "has_password": true,
            "is_admin": false
        }]));
    }

    #[tokio::test]
    async fn test_list_users_rejects_non_admin() {
        let mut admin = MockAdminService::new();
        admin.expect_is_admin().returning(|_| Ok(false));
        admin.expect_list_users().times(0);

        let response = build_server(admin).get("/admin/users").await;

        response.assert_status(StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_create_user_returns_created() {
        let mut admin = MockAdminService::new();
        admin.expect_is_admin().returning(|_| Ok(true));
        admin
            .expect_create_user()
            .times(1)
            .returning(|_, _| Ok(CreateUserOutcome::Created(UserId::from("new_user"))));

        let response = build_server(admin)
            .post("/admin/users")
            .json(&json!({ "email": "new@mail.test", "password": "s3cret" }))
            .await;

        response.assert_status(StatusCode::CREATED);
        response.assert_json(&json!({ "user_id": "new_user" }));
    }

    #[tokio::test]
    async fn test_create_user_rejects_invalid_email() {
        let mut admin = MockAdminService::new();
        admin.expect_is_admin().returning(|_| Ok(true));
        admin.expect_create_user().times(0);

        let response = build_server(admin)
            .post("/admin/users")
            .json(&json!({ "email": "not-an-email", "password": "s3cret" }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_user_conflict_on_existing_email() {
        let mut admin = MockAdminService::new();
        admin.expect_is_admin().returning(|_| Ok(true));
        admin
            .expect_create_user()
            .times(1)
            .returning(|_, _| Ok(CreateUserOutcome::EmailAlreadyUsed));

        let response = build_server(admin)
            .post("/admin/users")
            .json(&json!({ "email": "used@mail.test", "password": "s3cret" }))
            .await;

        response.assert_status(StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_set_user_password_returns_ok() {
        let mut admin = MockAdminService::new();
        admin.expect_is_admin().returning(|_| Ok(true));
        admin
            .expect_set_user_password()
            .times(1)
            .returning(|_, _| Ok(SetUserPasswordOutcome::Updated));

        let response = build_server(admin)
            .post("/admin/users/user_1/password")
            .json(&json!({ "password": "s3cret" }))
            .await;

        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_set_user_password_unknown_user() {
        let mut admin = MockAdminService::new();
        admin.expect_is_admin().returning(|_| Ok(true));
        admin
            .expect_set_user_password()
            .times(1)
            .returning(|_, _| Ok(SetUserPasswordOutcome::UnknownUser));

        let response = build_server(admin)
            .post("/admin/users/ghost/password")
            .json(&json!({ "password": "s3cret" }))
            .await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_set_admin_last_admin_conflict() {
        let mut admin = MockAdminService::new();
        admin.expect_is_admin().returning(|_| Ok(true));
        admin
            .expect_set_admin()
            .times(1)
            .returning(|_, _| Ok(SetAdminOutcome::LastAdmin));

        let response = build_server(admin)
            .post("/admin/users/user_1/admin")
            .json(&json!({ "is_admin": false }))
            .await;

        response.assert_status(StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_revoke_user_sessions_returns_count() {
        let mut admin = MockAdminService::new();
        admin.expect_is_admin().returning(|_| Ok(true));
        admin
            .expect_revoke_user_sessions()
            .times(1)
            .returning(|_| Ok(2));

        let response = build_server(admin)
            .post("/admin/users/user_1/sessions/revoke")
            .json(&json!({}))
            .await;

        response.assert_status(StatusCode::OK);
        response.assert_json(&json!({ "revoked": 2 }));
    }
}
