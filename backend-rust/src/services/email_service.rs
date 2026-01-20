use std::sync::Arc;
use lettre::{
    Message, SmtpTransport, Transport,
    transport::smtp::authentication::Credentials,
    message::{header::ContentType, Mailbox},
};

use crate::config::Config;
use crate::error::{AppError, AppResult};

pub struct EmailService {
    config: Arc<Config>,
}

impl EmailService {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub async fn send_email(&self, to: &str, subject: &str, body: &str) -> AppResult<()> {
        if self.config.smtp_username.is_empty() || self.config.smtp_password.is_empty() {
            tracing::warn!("SMTP not configured, skipping email to {}", to);
            return Ok(());
        }

        let from_mailbox: Mailbox = self.config.smtp_from.parse()
            .unwrap_or_else(|_| format!("VESSEL <{}>", self.config.smtp_username).parse().unwrap());

        let to_mailbox: Mailbox = to.parse()
            .map_err(|_| AppError::ValidationError("Invalid email address".to_string()))?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
            .map_err(|e| AppError::EmailError(e.to_string()))?;

        let creds = Credentials::new(
            self.config.smtp_username.clone(),
            self.config.smtp_password.clone(),
        );

        let mailer = SmtpTransport::starttls_relay(&self.config.smtp_host)
            .map_err(|e| AppError::EmailError(e.to_string()))?
            .credentials(creds)
            .port(self.config.smtp_port)
            .build();

        mailer.send(&email)
            .map_err(|e| AppError::EmailError(e.to_string()))?;

        tracing::info!("Email sent to {}", to);
        Ok(())
    }

    pub async fn send_pool_funded_notification(
        &self,
        to: &str,
        invoice_number: &str,
        amount: f64,
    ) -> AppResult<()> {
        let subject = "VESSEL - Your Invoice is Fully Funded!";
        let body = format!(
            r#"
            <html>
            <body style="font-family: Arial, sans-serif; padding: 20px;">
                <h2>Great News!</h2>
                <p>Your invoice <strong>{}</strong> has been fully funded.</p>
                <p>Funded Amount: <strong>Rp {:.2}</strong></p>
                <p>You can now request disbursement from your VESSEL dashboard.</p>
                <hr>
                <p style="color: #666; font-size: 12px;">VESSEL - Invoice Factoring Platform on Base Network</p>
            </body>
            </html>
            "#,
            invoice_number,
            amount
        );

        self.send_email(to, subject, &body).await
    }

    pub async fn send_investment_confirmation(
        &self,
        to: &str,
        invoice_number: &str,
        amount: f64,
        tranche: &str,
        expected_return: f64,
    ) -> AppResult<()> {
        let subject = "VESSEL - Investment Confirmed";
        let body = format!(
            r#"
            <html>
            <body style="font-family: Arial, sans-serif; padding: 20px;">
                <h2>Investment Confirmed</h2>
                <p>Your investment in invoice <strong>{}</strong> has been confirmed.</p>
                <table>
                    <tr><td>Amount:</td><td><strong>Rp {:.2}</strong></td></tr>
                    <tr><td>Tranche:</td><td><strong>{}</strong></td></tr>
                    <tr><td>Expected Return:</td><td><strong>Rp {:.2}</strong></td></tr>
                </table>
                <p>Track your investment in your VESSEL portfolio.</p>
                <hr>
                <p style="color: #666; font-size: 12px;">VESSEL - Invoice Factoring Platform on Base Network</p>
            </body>
            </html>
            "#,
            invoice_number,
            amount,
            tranche,
            expected_return
        );

        self.send_email(to, subject, &body).await
    }

    pub async fn send_mitra_approval_notification(&self, to: &str, company_name: &str) -> AppResult<()> {
        let subject = "VESSEL - Mitra Application Approved!";
        let body = format!(
            r#"
            <html>
            <body style="font-family: Arial, sans-serif; padding: 20px;">
                <h2>Congratulations!</h2>
                <p>Your mitra application for <strong>{}</strong> has been approved.</p>
                <p>You can now create invoices and request funding on VESSEL.</p>
                <hr>
                <p style="color: #666; font-size: 12px;">VESSEL - Invoice Factoring Platform on Base Network</p>
            </body>
            </html>
            "#,
            company_name
        );

        self.send_email(to, subject, &body).await
    }
}
