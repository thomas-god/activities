use derive_more::Constructor;

use crate::inbound::http::auth::{EmailAddress, MagicLink, services::magic_link::MailProvider};

#[cfg(feature = "multi-user")]
pub mod smtp;

#[derive(Debug, Clone, Constructor)]
pub struct DoNothingMailProvider {}

impl MailProvider for DoNothingMailProvider {
    async fn send_magic_link_email(
        &self,
        email: &EmailAddress,
        _magic_link: &MagicLink,
    ) -> Result<(), ()> {
        tracing::info!(
            "Dummy send to {email:?} for token: {:?} to user {:?}",
            _magic_link.token(),
            _magic_link.user()
        );
        Ok(())
    }
}
