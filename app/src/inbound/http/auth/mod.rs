use std::fmt::Debug;

use derive_more::Constructor;

use crate::domain::models::UserId;

pub mod email_based;
pub mod single_password;

#[derive(Debug, Clone)]
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
