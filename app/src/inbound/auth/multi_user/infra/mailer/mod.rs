use derive_more::Constructor;

use crate::inbound::auth::multi_user::{AuthLink, EmailAddress, EmailChangeToken, MailProvider};

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

    async fn send_email_change_email(
        &self,
        email: &EmailAddress,
        email_change_token: &EmailChangeToken,
    ) -> Result<(), ()> {
        tracing::info!(
            "Dummy send to {email:?} for email change token: {:?} to user {:?}",
            email_change_token.token(),
            email_change_token.user()
        );
        Ok(())
    }
}
