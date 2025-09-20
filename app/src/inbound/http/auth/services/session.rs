use derive_more::Constructor;

use crate::{
    domain::models::UserId,
    inbound::http::auth::{ISessionService, SessionTokenError},
};

#[derive(Debug, Clone, Constructor)]
pub struct SessionService {}

impl ISessionService for SessionService {
    async fn generate_session_token(
        &self,
        user: &UserId,
    ) -> Result<crate::inbound::http::auth::SessionToken, ()> {
        todo!()
    }

    async fn check_session_token(&self, _token: &str) -> Result<UserId, SessionTokenError> {
        todo!()
    }
}
