use handlebars::{
    Context, Handlebars, Helper, HelperResult, JsonRender, Output, RenderContext, RenderErrorReason,
};
use lettre::{
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    address::AddressError,
    message::{
        Mailbox, MultiPart, SinglePart,
        header::{self},
    },
    transport::smtp::authentication::Credentials,
};
use serde_json::json;
use thiserror::Error;

use crate::inbound::http::auth::{EmailAddress, MagicLink, services::magic_link::MailProvider};

#[derive(Debug, Clone)]
pub struct SMTPEmailProvider {
    from: Address,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    handlebars: Handlebars<'static>,
    domain: String,
}

#[derive(Debug, Clone, Error)]
pub enum SMTPEmailProviderCreationError {
    #[error("Invalid sender email address")]
    InvalidEmailAddress(#[from] AddressError),
    #[error("Relay error")]
    RelayError,
    #[error("Error when loading template files")]
    TemplateFileError,
}

impl SMTPEmailProvider {
    pub fn new(
        from: &str,
        username: &str,
        password: &str,
        relay: &str,
        domain: &str,
    ) -> Result<Self, SMTPEmailProviderCreationError> {
        // Load mails templates
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("magic_link", include_str!("templates/magic_link_email.hbs"))
            .map_err(|_| SMTPEmailProviderCreationError::TemplateFileError)?;
        handlebars.register_helper("link-helper", Box::new(link_helper));

        // Create async mailer
        let creds = Credentials::new(username.to_string(), password.to_string());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(relay)
            .map_err(|_| SMTPEmailProviderCreationError::RelayError)?
            .credentials(creds)
            .build();

        Ok(Self {
            from: from.parse()?,
            mailer,
            domain: domain.to_string(),
            handlebars,
        })
    }

    fn render_magic_link_template(
        &self,
        magic_link_url: &str,
        user_email: &str,
    ) -> Result<String, ()> {
        let data = json!({
            "MAGIC_LINK_URL": magic_link_url,
            "USER_EMAIL": user_email,
        });

        let rendered = self
            .handlebars
            .render("magic_link", &data)
            .map_err(|_| ())?;
        Ok(rendered)
    }
}

impl MailProvider for SMTPEmailProvider {
    async fn send_magic_link_email(
        &self,
        email: &EmailAddress,
        magic_link: &MagicLink,
    ) -> Result<(), ()> {
        let link = format!("{}/login/{}", self.domain, magic_link.token());

        let text_body = format!(
            "Hello!\n\nClick this link to sign in to your account:\n{}\n\nThis link expires in 15 minutes.\n\nIf you didn't request this, you can ignore this email.",
            link
        );

        let html_body = self.render_magic_link_template(&link, &email.to_string())?;

        let email = Message::builder()
            .from(Mailbox::new(None, self.from.clone()))
            .to(Mailbox::new(None, email.value().parse().map_err(|_| ())?))
            .subject("Your magic link for activities.training")
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN)
                            .body(text_body),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(html_body),
                    ),
            )
            .map_err(|_| ())?;

        match self.mailer.send(email).await {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::warn!("{err:?}");
                Err(())
            }
        }
    }
}

fn link_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let text = h
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("link-helper", 0))?
        .value()
        .render();

    let url = h
        .param(1)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("link-helper", 1))?
        .value()
        .render();

    let escaped_text = handlebars::html_escape(&text);
    let escaped_url = handlebars::html_escape(&url);

    let html = format!(
        r#"<a href="{}" class="magic-link-button">{}</a>"#,
        escaped_url, escaped_text
    );

    out.write(&html)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use handlebars::Handlebars;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_link_helper() {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("link-helper", Box::new(link_helper));

        handlebars
            .register_template_string(
                "test",
                "{{link-helper  \"Sign In to Your Account\" MAGIC_LINK_URL}}",
            )
            .unwrap();

        assert_eq!(
            handlebars
                .render(
                    "test",
                    &json!({"MAGIC_LINK_URL": "127.0.0.1:5173/login/alongandsecuretoken"})
                )
                .unwrap(),
            "<a href=\"127.0.0.1:5173/login/alongandsecuretoken\" class=\"magic-link-button\">Sign In to Your Account</a>"
        );
    }
}
