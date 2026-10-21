//! Transactional email delivery (contact notifications, newsletters, resets).

use lettre::message::{header, Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::sync::Arc;

use crate::config::SmtpConfig;

/// A thin wrapper that never fails the request if the mail server is down.
#[derive(Clone)]
pub struct Mailer {
    inner: Option<Arc<AsyncSmtpTransport<Tokio1Executor>>>,
    config: SmtpConfig,
}

impl Mailer {
    /// Build a mailer from the SMTP configuration. Returns a mailer that only
    /// logs when SMTP is not configured.
    pub fn new(config: SmtpConfig) -> Self {
        let inner = config.host.as_ref().and_then(|host| {
            let mut builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
                .or_else(|_| AsyncSmtpTransport::<Tokio1Executor>::relay(host))
                .ok()?;
            if let (Some(user), Some(pass)) = (&config.username, &config.password) {
                builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
            }
            builder = builder.port(config.port);
            Some(Arc::new(builder.build()))
        });

        Self { inner, config }
    }

    pub fn is_configured(&self) -> bool {
        self.inner.is_some()
    }

    /// Send a plain-text email.
    pub async fn send_text(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        self.send(to, subject, body, None).await
    }

    /// Send an email with an optional HTML alternative.
    pub async fn send(
        &self,
        to: &str,
        subject: &str,
        text: &str,
        html: Option<&str>,
    ) -> Result<(), String> {
        let from_addr = self
            .config
            .from
            .parse::<lettre::Address>()
            .map_err(|_| "invalid SMTP_FROM address".to_string())?;
        let to_addr = to
            .parse::<lettre::Address>()
            .map_err(|_| "invalid recipient address".to_string())?;

        let from = Mailbox::new(Some(self.config.from_name.clone()), from_addr);
        let recipient = Mailbox::new(None, to_addr);

        let mut builder = Message::builder()
            .from(from)
            .to(recipient)
            .subject(subject.to_string());

        let message = match html {
            Some(html) => {
                let multipart =
                    MultiPart::alternative_plain_html(text.to_string(), html.to_string());
                builder.multipart(multipart)
            }
            None => {
                builder = builder.header(header::ContentType::TEXT_PLAIN);
                builder.body(text.to_string())
            }
        }
        .map_err(|e| e.to_string())?;

        let Some(transport) = &self.inner else {
            tracing::info!(to = %to, subject = %subject, "SMTP not configured — email not sent:\n{text}");
            return Ok(());
        };

        if self.config.dry_run {
            tracing::info!(to = %to, subject = %subject, "SMTP dry-run — email not delivered:\n{text}");
            return Ok(());
        }

        transport.send(message).await.map(|_| ()).map_err(|e| {
            tracing::error!(error = %e, to = %to, subject = %subject, "failed to send email");
            e.to_string()
        })
    }

    /// Notify the site owner about a new contact message.
    pub async fn notify_contact(
        &self,
        notify_to: &str,
        name: &str,
        reply_to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), String> {
        let text = format!("New message from {name} <{reply_to}>\n\nSubject: {subject}\n\n{body}");
        self.send_text(notify_to, &format!("[Contact] {subject}"), &text)
            .await
    }
}

/// Build a simple HTML email wrapper.
pub fn html_wrapper(title: &str, body_html: &str, accent: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
</head>
<body style="margin:0;padding:0;background:#0b0d12;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background:#0b0d12;padding:32px 16px;">
    <tr>
      <td align="center">
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;width:100%;background:#12151d;border:1px solid #1f2530;border-radius:16px;overflow:hidden;">
          <tr>
            <td style="height:4px;background:{accent};font-size:0;line-height:0;">&nbsp;</td>
          </tr>
          <tr>
            <td style="padding:32px;color:#e6e9ef;font-size:16px;line-height:1.65;">
              <h1 style="margin:0 0 20px;font-size:20px;line-height:1.4;color:#ffffff;">{title}</h1>
              {body_html}
              <p style="margin:32px 0 0;color:#6b7280;font-size:13px;">You are receiving this email because you subscribed to the newsletter.</p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#,
        title = crate::markdown::escape(title),
        body_html = body_html,
        accent = accent,
    )
}

/// Strip HTML from a body so it can be used as the plain-text alternative.
pub fn html_to_text(html: &str) -> String {
    crate::markdown::decode_entities(&crate::markdown::strip_tags(html))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
