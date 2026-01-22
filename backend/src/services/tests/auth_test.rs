use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::Config;
use crate::models::{MitraApplication, RegisterRequest};
use crate::repository::{MitraRepository, OtpRepository, UserRepository};
use crate::services::email_service::EmailService;
use crate::services::pinata_service::PinataService;
use crate::services::{AuthService, OtpService};
use crate::utils::JwtManager;

// Mock implementations or helpers could go here if we were using mockall fully,
// but for integration logic with DB, we setup the service with real repos.

pub fn get_test_config() -> Config {
    // Load from environment (supports .env) to correct credentials
    Config::from_env().expect("Failed to load configuration from environment")
}

pub async fn setup_services(pool: PgPool) -> Arc<AuthService> {
    // Run migrations to ensure schema exists
    crate::database::run_migrations(&pool)
        .await
        .expect("Failed to run migrations in test setup");

    let user_repo = Arc::new(UserRepository::new(pool.clone()));
    let mitra_repo = Arc::new(MitraRepository::new(pool.clone()));
    let otp_repo = Arc::new(OtpRepository::new(pool.clone()));

    let config = Arc::new(get_test_config());
    let jwt_manager = Arc::new(JwtManager::new(&config.jwt_secret, 24, 24));

    let email_service = Arc::new(EmailService::new(config.clone()));
    let _pinata_service = Arc::new(PinataService::new(config.clone()));

    let otp_service = Arc::new(OtpService::new(
        otp_repo,
        email_service,
        config.clone(),
        jwt_manager.clone(),
    ));

    Arc::new(AuthService::new(
        user_repo,
        mitra_repo,
        jwt_manager,
        otp_service,
        config,
    ))
}

#[tokio::test]
async fn test_register_mitra_success() {
    // Manually connect
    let config = get_test_config();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    // FIX SCHEMA MISMATCH (Test Environment Hack) - Removed as migrations now handle this
    // The run_migrations call in setup_services should handle the scheme update.

    let auth_service = setup_services(pool.clone()).await;
    // ...
    let email = "test_mitra_integration@example.com";

    // Generate valid OTP Token (JWT) matching the secret
    let jwt_manager = JwtManager::new(&config.jwt_secret, 24, 24);
    let otp_token = jwt_manager
        .generate_otp_token(email, "registration")
        .expect("Failed to generate OTP token");

    // CLEANUP START (Ensure clear state)
    sqlx::query!("DELETE FROM otp_codes WHERE email = $1", email)
        .execute(&pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM users WHERE email = $1", email)
        .execute(&pool)
        .await
        .ok(); // cascading delete should handle mitra_applications if set up, otherwise separate delete needed
    sqlx::query!(
        "DELETE FROM mitra_applications WHERE user_id IN (SELECT id FROM users WHERE email = $1)",
        email
    )
    .execute(&pool)
    .await
    .ok();

    // 1. No need to insert into otp_codes because register verifies the JWT token signature

    // 2. Prepare RegisterRequest
    let req = RegisterRequest {
        email: email.to_string(),
        username: "testmitra_int".to_string(),
        password: "password123".to_string(),
        confirm_password: "password123".to_string(),
        cooperative_agreement: true,
        otp_token, // JWT token
        company_name: Some("Test Mitra Integration PT".to_string()),
        company_type: Some("PT".to_string()),
        npwp: Some("123456789012345".to_string()),
        annual_revenue: Some("1000000".to_string()),
        address: None,
        business_description: None,
        website_url: None,
        year_founded: None,
        key_products: None,
        export_markets: None,
    };

    // 3. Call register
    let result = auth_service.register(req).await;

    // 4. Verify result
    assert!(
        result.is_ok(),
        "Registration should succeed: {:?}",
        result.err()
    );
    let response = result.unwrap();

    assert_eq!(response.user.email, email);
    assert_eq!(response.user.role, "mitra");

    // 5. Verify MitraApplication exists in DB
    let app = sqlx::query_as::<_, MitraApplication>(
        "SELECT * FROM mitra_applications WHERE user_id = $1",
    )
    .bind(response.user.id)
    .fetch_one(&pool)
    .await
    .expect("Mitra application should exist");

    assert_eq!(app.company_name, "Test Mitra Integration PT");
    assert_eq!(app.status, "pending");

    // CLEANUP END
    sqlx::query!("DELETE FROM users WHERE email = $1", email)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_register_fail_invalid_otp() {
    let config = get_test_config();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    let auth_service = setup_services(pool.clone()).await;

    let req = RegisterRequest {
        email: "fail@example.com".to_string(),
        username: "fail".to_string(),
        password: "password123".to_string(),
        confirm_password: "password123".to_string(),
        cooperative_agreement: true,
        otp_token: "wrong.jwt.token".to_string(), // Invalid OTP JWT
        company_name: Some("Fail PT".to_string()),
        company_type: None,
        npwp: Some("123".to_string()),
        annual_revenue: Some("0".to_string()),
        address: None,
        business_description: None,
        website_url: None,
        year_founded: None,
        key_products: None,
        export_markets: None,
    };

    let result = auth_service.register(req).await;
    assert!(result.is_err());
}
