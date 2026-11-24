use derive_more::Constructor;

use crate::multi_user::AuthLink;
use crate::multi_user::EmailAddress;
use crate::multi_user::EmailChangeToken;
use crate::multi_user::MailProvider;

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

/// Runtime choice between an enabled and a disabled [`MailProvider`].
#[derive(Debug, Clone)]
pub enum MaybeMailProvider<Enabled> {
    Enabled(Enabled),
    Disabled(DoNothingMailProvider),
}

impl<Enabled> MailProvider for MaybeMailProvider<Enabled>
where
    Enabled: MailProvider,
{
    async fn send_auth_link_email(
        &self,
        email: &EmailAddress,
        auth_link: &AuthLink,
    ) -> Result<(), ()> {
        match self {
            Self::Enabled(service) => service.send_auth_link_email(email, auth_link).await,
            Self::Disabled(service) => service.send_auth_link_email(email, auth_link).await,
        }
    }

    async fn send_email_change_email(
        &self,
        email: &EmailAddress,
        email_change_token: &EmailChangeToken,
    ) -> Result<(), ()> {
        match self {
            Self::Enabled(service) => {
                service
                    .send_email_change_email(email, email_change_token)
                    .await
            }
            Self::Disabled(service) => {
                service
                    .send_email_change_email(email, email_change_token)
                    .await
            }
        }
    }
}
