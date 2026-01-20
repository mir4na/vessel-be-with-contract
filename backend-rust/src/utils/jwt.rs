use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct JwtManager {
    secret: String,
    expiry_hours: i64,
    refresh_expiry_hours: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    #[serde(rename = "type")]
    pub token_type: String, // "access" or "refresh"
}

impl Claims {
    pub fn user_id(&self) -> Uuid {
        Uuid::parse_str(&self.sub).unwrap_or_default()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OtpTokenClaims {
    pub sub: String, // Email
    pub purpose: String,
    pub exp: i64,
    pub iat: i64,
}

impl JwtManager {
    pub fn new(secret: &str, expiry_hours: i64, refresh_expiry_hours: i64) -> Self {
        Self {
            secret: secret.to_string(),
            expiry_hours,
            refresh_expiry_hours,
        }
    }

    pub fn generate_access_token(&self, user_id: Uuid, email: &str, role: &str) -> AppResult<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.expiry_hours);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            token_type: "access".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AppError::InternalError(format!("Failed to generate token: {}", e)))
    }

    pub fn generate_refresh_token(&self, user_id: Uuid, email: &str, role: &str) -> AppResult<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.refresh_expiry_hours);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            token_type: "refresh".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AppError::InternalError(format!("Failed to generate refresh token: {}", e)))
    }

    pub fn verify_token(&self, token: &str) -> AppResult<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    pub fn verify_access_token(&self, token: &str) -> AppResult<Claims> {
        let claims = self.verify_token(token)?;
        if claims.token_type != "access" {
            return Err(AppError::InvalidToken);
        }
        Ok(claims)
    }

    pub fn verify_refresh_token(&self, token: &str) -> AppResult<Claims> {
        let claims = self.verify_token(token)?;
        if claims.token_type != "refresh" {
            return Err(AppError::InvalidToken);
        }
        Ok(claims)
    }

    pub fn generate_otp_token(&self, email: &str, purpose: &str) -> AppResult<String> {
        let now = Utc::now();
        let exp = now + Duration::minutes(30); // OTP token valid for 30 minutes

        let claims = OtpTokenClaims {
            sub: email.to_string(),
            purpose: purpose.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AppError::InternalError(format!("Failed to generate OTP token: {}", e)))
    }

    pub fn verify_otp_token(&self, token: &str, expected_purpose: &str) -> AppResult<String> {
        let token_data = decode::<OtpTokenClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )?;

        if token_data.claims.purpose != expected_purpose {
            return Err(AppError::InvalidToken);
        }

        Ok(token_data.claims.sub)
    }

    pub fn get_expiry_hours(&self) -> i64 {
        self.expiry_hours
    }
}
