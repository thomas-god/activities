use std::fmt::Debug;

use derive_more::Constructor;

use crate::domain::models::UserId;

pub mod email_based;
pub mod no_auth;

#[derive(Clone)]
pub enum AuthStrategy {
    NoAuth,
    EmailBased,
}

#[derive(Debug, Clone, Constructor)]
pub struct AuthenticatedUser(UserId);

impl AuthenticatedUser {
    pub fn user(&self) -> &UserId {
        &self.0
    }
}

#[cfg(test)]
mod test_auth {
    use super::*;

    #[test]
    fn test_do_not_leak_single_password_content_when_debug() {
        let strategy = AuthStrategy::SinglePassword("secret".to_string());

        assert_eq!(format!("{strategy:?}"), "SinglePassword")
    }
}
