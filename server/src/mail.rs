use anyhow::Result;

use crate::config::Config;

pub async fn send_password_reset(config: &Config, to: &str, token: &str) -> Result<()> {
    let Some(ref host) = config.smtp_host else {
        tracing::info!("(dev) password reset token for {to}: {token}");
        return Ok(());
    };
    let url = format!("{}/reset-password?token={token}", config.app_url);
    let body = format!("Click to reset your Whots password:\n\n{url}\n\nExpires in 1 hour.");
    send(config, host, to, "Reset your Whots password", &body).await
}

pub async fn send_verification(config: &Config, to: &str, token: &str) -> Result<()> {
    let Some(ref host) = config.smtp_host else {
        tracing::info!("(dev) email verification token for {to}: {token}");
        return Ok(());
    };
    let url = format!("{}/verify-email?token={token}", config.app_url);
    let body = format!("Click to verify your Whots email address:\n\n{url}");
    send(config, host, to, "Verify your Whots email", &body).await
}

async fn send(config: &Config, host: &str, to: &str, subject: &str, body: &str) -> Result<()> {
    use lettre::{
        message::header::ContentType, transport::smtp::authentication::Credentials,
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    };

    let from = config.smtp_from.as_deref().unwrap_or("noreply@whots.app");
    let port = config.smtp_port.unwrap_or(587);
    let user = config.smtp_user.as_deref().unwrap_or_default();
    let pass = config.smtp_password.as_deref().unwrap_or_default();

    let message = Message::builder()
        .from(from.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())?;

    let creds = Credentials::new(user.into(), pass.into());
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(host)?
        .port(port)
        .credentials(creds)
        .build();

    mailer.send(message).await?;
    Ok(())
}
