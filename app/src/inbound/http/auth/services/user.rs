use crate::inbound::http::auth::IUserService;

#[derive(Debug, Clone)]
pub struct UserService {}

impl IUserService for UserService {
    async fn check_session_token(
        &self,
        _token: &str,
    ) -> Result<crate::domain::models::UserId, crate::inbound::http::auth::SessionTokenError> {
        todo!()
    }
}
