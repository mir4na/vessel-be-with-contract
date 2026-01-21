use super::auth_test::get_test_config;
use crate::models::OtpPurpose;
use crate::services::otp_service::OtpService;
use sqlx::PgPool;
use std::sync::Arc;
use chrono::{Utc, Duration};

#[tokio::test]
async fn test_send_and_verify_otp_success() {
    let config = get_test_config();
    let pool = PgPool::connect(&config.database_url).await.expect("Failed to connect to DB");
    
    let email_service = Arc::new(crate::services::EmailService::new(Arc::new(config.clone())));
    let otp_repo = Arc::new(crate::repository::OtpRepository::new(pool.clone()));
    let jwt_manager = Arc::new(crate::utils::JwtManager::new(&config.jwt_secret, config.jwt_expiry_hours, config.jwt_refresh_expiry_hours));
    
    let otp_service = OtpService::new(
        otp_repo.clone(),
        email_service.clone(),
        Arc::new(config.clone()),
        jwt_manager.clone(),
    );

    let email = "test_otp_success@example.com";
    
    // Manually insert an OTP code
    let code = "123456";
    // otp_repo.create takes &str for purpose in current implementation based on error log
    // Wait, the error log says: method create ... purpose: &str
    // But in otp_test.rs I used OtpPurpose::Registration
    // Let's check OtpPurpose again. It implements Display.
    // We should pass purpose.to_string().as_str() or just a string literal.
    
    let purpose = OtpPurpose::Registration;
    let expires_at = Utc::now() + Duration::minutes(15);
    
    let _ = otp_repo.create(email, code, &purpose.to_string(), expires_at).await;
    
    // 2. Verify OTP
    let result = otp_service.verify_otp(email, code, &purpose.to_string()).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.otp_token.is_empty());
}

#[tokio::test]
async fn test_verify_otp_invalid_code() {
    let config = get_test_config();
    let pool = PgPool::connect(&config.database_url).await.expect("Failed to connect to DB");
    
    let email_service = Arc::new(crate::services::EmailService::new(Arc::new(config.clone())));
    let otp_repo = Arc::new(crate::repository::OtpRepository::new(pool.clone()));
    let jwt_manager = Arc::new(crate::utils::JwtManager::new(&config.jwt_secret, config.jwt_expiry_hours, config.jwt_refresh_expiry_hours));
    
    let otp_service = OtpService::new(
        otp_repo.clone(),
        email_service.clone(),
        Arc::new(config.clone()),
        jwt_manager.clone(),
    );

    let email = "test_otp_fail@example.com";
    let code = "123456";
    let purpose = OtpPurpose::Registration;
    let expires_at = Utc::now() + Duration::minutes(15);

    let _ = otp_repo.create(email, code, &purpose.to_string(), expires_at).await;
    
    // Verify with wrong code
    let result = otp_service.verify_otp(email, "000000", &purpose.to_string()).await;
    assert!(result.is_err());
}
