use derive_more::Constructor;

use crate::inbound::http::auth::{AuthLink, EmailAddress, services::auth_link::MailProvider};

#[cfg(feature = "multi-user")]
pub mod smtp;

#[derive(Debug, Clone, Constructor)]
pub struct DoNothingMailProvider {}

impl MailProvider for DoNothingMailProvider {
    async fn send_auth_link_email(
        &self,
        email: &EmailAddress,
        _auth_link: &AuthLink,
    ) -> Result<(), ()> {
        tracing::info!(
            "Dummy send to {email:?} for token: {:?} to user {:?}",
            _auth_link.token(),
            _auth_link.user()
        );
        Ok(())
    }
}
