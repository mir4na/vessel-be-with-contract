use chrono::{Duration, Utc};
use std::sync::Arc;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::{SendOtpResponse, VerifyOtpResponse};
use crate::repository::OtpRepository;
use crate::utils::{generate_otp, JwtManager};

use super::EmailService;

pub struct OtpService {
    otp_repo: Arc<OtpRepository>,
    email_service: Arc<EmailService>,
    config: Arc<Config>,
    jwt_manager: Arc<JwtManager>,
}

impl OtpService {
    pub fn new(
        otp_repo: Arc<OtpRepository>,
        email_service: Arc<EmailService>,
        config: Arc<Config>,
        jwt_manager: Arc<JwtManager>,
    ) -> Self {
        Self {
            otp_repo,
            email_service,
            config,
            jwt_manager,
        }
    }

    pub async fn send_otp(&self, email: &str, purpose: &str) -> AppResult<SendOtpResponse> {
        // Validate purpose
        if !["registration", "login", "password_reset"].contains(&purpose) {
            return Err(AppError::ValidationError("Invalid OTP purpose".to_string()));
        }

        // Generate OTP
        let code = generate_otp();
        let expires_at = Utc::now() + Duration::minutes(self.config.otp_expiry_minutes);

        // Log OTP for development/debugging
        tracing::info!("Generated OTP for {}: {}", email, code);

        // Delete any existing OTPs for this email and purpose
        self.otp_repo.delete_by_email(email, purpose).await?;

        // Save new OTP
        self.otp_repo
            .create(email, &code, purpose, expires_at)
            .await?;

        // Send email
        let subject = match purpose {
            "registration" => "VESSEL - Verify Your Email",
            "login" => "VESSEL - Login Verification",
            "password_reset" => "VESSEL - Reset Your Password",
            _ => "VESSEL - Verification Code",
        };

        let body = format!(
            r#"
            <html>
            <body style="font-family: Arial, sans-serif; padding: 20px;">
                <h2>VESSEL Verification Code</h2>
                <p>Your verification code is:</p>
                <h1 style="font-size: 32px; letter-spacing: 5px; color: #2563eb;">{}</h1>
                <p>This code will expire in {} minutes.</p>
                <p>If you didn't request this code, please ignore this email.</p>
                <hr>
                <p style="color: #666; font-size: 12px;">VESSEL - Invoice Factoring Platform on Base Network</p>
            </body>
            </html>
            "#,
            code, self.config.otp_expiry_minutes
        );

        self.email_service.send_email(email, subject, &body).await?;

        Ok(SendOtpResponse {
            message: format!("OTP sent to {}", email),
            expires_in_minutes: self.config.otp_expiry_minutes,
        })
    }

    pub async fn verify_otp(
        &self,
        email: &str,
        code: &str,
        purpose: &str,
    ) -> AppResult<VerifyOtpResponse> {
        // First, find the latest OTP for this email and purpose
        let otp = self
            .otp_repo
            .find_latest(email, purpose)
            .await?
            .ok_or_else(|| {
                AppError::ValidationError("No OTP found. Please request a new OTP.".to_string())
            })?;

        // Check if already verified
        if otp.verified {
            return Err(AppError::ValidationError(
                "OTP already used. Please request a new OTP.".to_string(),
            ));
        }

        // Check max attempts first
        if otp.attempts >= self.config.otp_max_attempts {
            return Err(AppError::ValidationError(
                "Maximum attempts exceeded. Please request a new OTP.".to_string(),
            ));
        }

        // Check if expired
        // Check if expired
        if otp.expires_at < Utc::now() {
            return Err(AppError::ValidationError(
                "OTP has expired. Please request a new OTP.".to_string(),
            ));
        }

        // Check if code matches
        if otp.code != code {
            // Increment attempts on wrong code
            self.otp_repo.increment_attempts(otp.id).await?;
            let remaining = self.config.otp_max_attempts - otp.attempts - 1;
            if remaining > 0 {
                return Err(AppError::ValidationError(format!(
                    "Invalid OTP code. {} attempts remaining.",
                    remaining
                )));
            } else {
                return Err(AppError::ValidationError(
                    "Invalid OTP code. Maximum attempts exceeded. Please request a new OTP."
                        .to_string(),
                ));
            }
        }

        // Mark as verified
        self.otp_repo.mark_verified(otp.id).await?;

        // Generate OTP token
        let otp_token = self.jwt_manager.generate_otp_token(email, purpose)?;

        Ok(VerifyOtpResponse {
            message: "OTP verified successfully".to_string(),
            otp_token,
            expires_in_minutes: 30,
        })
    }

    pub fn verify_otp_token(&self, token: &str, expected_purpose: &str) -> AppResult<String> {
        self.jwt_manager.verify_otp_token(token, expected_purpose)
    }

    pub async fn increment_attempts(&self, email: &str, purpose: &str) -> AppResult<()> {
        if let Some(otp) = self.otp_repo.find_latest(email, purpose).await? {
            self.otp_repo.increment_attempts(otp.id).await?;
        }
        Ok(())
    }
}
