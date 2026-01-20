use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::OtpCode;

#[derive(Clone)]
pub struct OtpRepository {
    pool: PgPool,
}

impl OtpRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, email: &str, code: &str, purpose: &str, expires_at: DateTime<Utc>) -> AppResult<OtpCode> {
        let otp = sqlx::query_as::<_, OtpCode>(
            r#"
            INSERT INTO otp_codes (email, code, purpose, expires_at, verified, attempts)
            VALUES ($1, $2, $3, $4, false, 0)
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(code)
        .bind(purpose)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(otp)
    }

    pub async fn find_latest(&self, email: &str, purpose: &str) -> AppResult<Option<OtpCode>> {
        let otp = sqlx::query_as::<_, OtpCode>(
            r#"
            SELECT * FROM otp_codes
            WHERE email = $1 AND purpose = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(email)
        .bind(purpose)
        .fetch_optional(&self.pool)
        .await?;

        Ok(otp)
    }

    pub async fn find_valid(&self, email: &str, code: &str, purpose: &str) -> AppResult<Option<OtpCode>> {
        let otp = sqlx::query_as::<_, OtpCode>(
            r#"
            SELECT * FROM otp_codes
            WHERE email = $1 AND code = $2 AND purpose = $3
              AND verified = false AND expires_at > NOW()
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(email)
        .bind(code)
        .bind(purpose)
        .fetch_optional(&self.pool)
        .await?;

        Ok(otp)
    }

    pub async fn mark_verified(&self, id: Uuid) -> AppResult<OtpCode> {
        let otp = sqlx::query_as::<_, OtpCode>(
            "UPDATE otp_codes SET verified = true WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(otp)
    }

    pub async fn increment_attempts(&self, id: Uuid) -> AppResult<OtpCode> {
        let otp = sqlx::query_as::<_, OtpCode>(
            "UPDATE otp_codes SET attempts = attempts + 1 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(otp)
    }

    pub async fn delete_expired(&self) -> AppResult<u64> {
        let result = sqlx::query("DELETE FROM otp_codes WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete_by_email(&self, email: &str, purpose: &str) -> AppResult<u64> {
        let result = sqlx::query("DELETE FROM otp_codes WHERE email = $1 AND purpose = $2")
            .bind(email)
            .bind(purpose)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
