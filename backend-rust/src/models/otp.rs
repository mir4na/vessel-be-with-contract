use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OtpPurpose {
    #[serde(rename = "registration")]
    Registration,
    #[serde(rename = "login")]
    Login,
    #[serde(rename = "password_reset")]
    PasswordReset,
}

impl std::fmt::Display for OtpPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OtpPurpose::Registration => write!(f, "registration"),
            OtpPurpose::Login => write!(f, "login"),
            OtpPurpose::PasswordReset => write!(f, "password_reset"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OtpCode {
    pub id: Uuid,
    pub email: String,
    pub code: String,
    pub purpose: String,
    pub expires_at: DateTime<Utc>,
    pub verified: bool,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SendOtpRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    pub purpose: String, // "registration", "login", "password_reset"
}

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyOtpRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(equal = 6, message = "OTP code must be 6 digits"))]
    pub code: String,
    pub purpose: String,
}

#[derive(Debug, Serialize)]
pub struct SendOtpResponse {
    pub message: String,
    pub expires_in_minutes: i64,
}

#[derive(Debug, Serialize)]
pub struct VerifyOtpResponse {
    pub message: String,
    pub otp_token: String,
    pub expires_in_minutes: i64,
}
