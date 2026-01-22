use sqlx::PgPool;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::Config;
use crate::models::MitraApplyRequest;
use crate::repository::{MitraRepository, UserRepository};
use crate::services::email_service::EmailService;
use crate::services::pinata_service::PinataService;
use crate::services::MitraService;

use super::auth_test::get_test_config;

pub async fn setup_mitra_service(pool: PgPool) -> (Arc<MitraService>, PgPool) {
    crate::database::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let config = Arc::new(get_test_config());
    let mitra_repo = Arc::new(MitraRepository::new(pool.clone()));
    let user_repo = Arc::new(UserRepository::new(pool.clone()));
    let email_service = Arc::new(EmailService::new(config.clone()));
    let pinata_service = Arc::new(PinataService::new(config.clone()));

    let service = Arc::new(MitraService::new(
        mitra_repo,
        user_repo,
        email_service,
        pinata_service,
    ));

    (service, pool)
}

async fn create_test_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query!(
        "DELETE FROM mitra_applications WHERE user_id IN (SELECT id FROM users WHERE email = $1)",
        email
    )
    .execute(pool)
    .await
    .ok();
    sqlx::query!("DELETE FROM users WHERE email = $1", email)
        .execute(pool)
        .await
        .ok();

    let user_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, cooperative_agreement, email_verified, profile_completed, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'mitra', 'calon_mitra', true, true, true, true, false, 0)"#,
        user_id, email, &format!("user_{}", user_id.simple())
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

async fn create_test_admin(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query!("DELETE FROM users WHERE email = $1", email)
        .execute(pool)
        .await
        .ok();

    let admin_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, cooperative_agreement, email_verified, profile_completed, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'admin', 'admin', true, true, true, true, true, 0)"#,
        admin_id, email, &format!("admin_{}", admin_id.simple())
    )
    .execute(pool)
    .await
    .expect("Failed to create test admin");

    admin_id
}

#[tokio::test]
async fn test_mitra_apply_success() {
    let config = get_test_config();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");
    let (service, pool) = setup_mitra_service(pool).await;

    let user_id = create_test_user(&pool, "mitra_apply_test@example.com").await;

    let req = MitraApplyRequest {
        company_name: "Test Company PT".to_string(),
        company_type: Some("PT".to_string()),
        npwp: "1234567890123456".to_string(),
        annual_revenue: "1M-5M".to_string(),
        address: Some("Jl. Test No. 1".to_string()),
        business_description: Some("Test business".to_string()),
        website_url: None,
        year_founded: Some(2020),
        key_products: Some("Coffee".to_string()),
        export_markets: Some("USA".to_string()),
    };

    let result = service.apply(user_id, req).await;
    assert!(result.is_ok(), "Apply should succeed: {:?}", result.err());

    let app = result.unwrap();
    assert_eq!(app.company_name, "Test Company PT");
    assert_eq!(app.status, "pending");

    // Cleanup
    sqlx::query!(
        "DELETE FROM users WHERE email = $1",
        "mitra_apply_test@example.com"
    )
    .execute(&pool)
    .await
    .ok();
}

#[tokio::test]
async fn test_get_pending_applications() {
    let config = get_test_config();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");
    let (service, pool) = setup_mitra_service(pool).await;

    let user_id = create_test_user(&pool, "mitra_pending_test@example.com").await;

    // Apply
    let req = MitraApplyRequest {
        company_name: "Pending Test PT".to_string(),
        company_type: Some("PT".to_string()),
        npwp: "9876543210123456".to_string(),
        annual_revenue: "<1M".to_string(),
        address: None,
        business_description: None,
        website_url: None,
        year_founded: None,
        key_products: None,
        export_markets: None,
    };

    let _ = service
        .apply(user_id, req)
        .await
        .expect("Apply should succeed");

    // Get pending
    let (applications, total) = service
        .get_pending_applications(1, 10)
        .await
        .expect("Should get pending");

    assert!(total >= 1, "Should have at least 1 pending application");
    assert!(applications
        .iter()
        .any(|a| a.company_name == "Pending Test PT"));

    // Cleanup
    sqlx::query!(
        "DELETE FROM users WHERE email = $1",
        "mitra_pending_test@example.com"
    )
    .execute(&pool)
    .await
    .ok();
}

#[tokio::test]
async fn test_approve_mitra_application() {
    let config = get_test_config();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");
    let (service, pool) = setup_mitra_service(pool).await;

    let user_id = create_test_user(&pool, "mitra_approve_test@example.com").await;
    let admin_id = create_test_admin(&pool, "admin_approve_test@example.com").await;

    // Apply
    let req = MitraApplyRequest {
        company_name: "Approve Test PT".to_string(),
        company_type: Some("PT".to_string()),
        npwp: "1111222233334444".to_string(),
        annual_revenue: "5M-25M".to_string(),
        address: Some("Jl. Approve".to_string()),
        business_description: Some("Approved biz".to_string()),
        website_url: None,
        year_founded: Some(2015),
        key_products: None,
        export_markets: None,
    };

    let app = service
        .apply(user_id, req)
        .await
        .expect("Apply should succeed");
    assert_eq!(app.status, "pending");

    // Approve
    let approved = service
        .approve(app.id, admin_id)
        .await
        .expect("Approve should succeed");
    assert_eq!(approved.status, "approved");
    assert!(approved.reviewed_by.is_some());
    assert!(approved.reviewed_at.is_some());

    // Verify user role and profile_completed updated
    // Verify user role and profile_completed updated
    let row = sqlx::query("SELECT role, member_status, profile_completed FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("User should exist");

    let role: String = row.get("role");
    let member_status: String = row.get("member_status");
    let profile_completed: bool = row.get("profile_completed");

    assert_eq!(role, "mitra");
    assert_eq!(member_status, "member_mitra");
    assert!(
        profile_completed,
        "profile_completed should be true after approval"
    );

    // Cleanup
    sqlx::query!(
        "DELETE FROM users WHERE email = $1",
        "mitra_approve_test@example.com"
    )
    .execute(&pool)
    .await
    .ok();
    sqlx::query!(
        "DELETE FROM users WHERE email = $1",
        "admin_approve_test@example.com"
    )
    .execute(&pool)
    .await
    .ok();
}

#[tokio::test]
async fn test_reject_mitra_application() {
    let config = get_test_config();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");
    let (service, pool) = setup_mitra_service(pool).await;

    let user_id = create_test_user(&pool, "mitra_reject_test@example.com").await;
    let admin_id = create_test_admin(&pool, "admin_reject_test@example.com").await;

    // Apply
    let req = MitraApplyRequest {
        company_name: "Reject Test PT".to_string(),
        company_type: Some("CV".to_string()),
        npwp: "5555666677778888".to_string(),
        annual_revenue: "<1M".to_string(),
        address: None,
        business_description: None,
        website_url: None,
        year_founded: None,
        key_products: None,
        export_markets: None,
    };

    let app = service
        .apply(user_id, req)
        .await
        .expect("Apply should succeed");

    // Reject
    let rejected = service
        .reject(app.id, admin_id, "Incomplete documents")
        .await
        .expect("Reject should succeed");
    assert_eq!(rejected.status, "rejected");
    assert_eq!(
        rejected.rejection_reason.as_deref(),
        Some("Incomplete documents")
    );

    // Cleanup
    sqlx::query!(
        "DELETE FROM users WHERE email = $1",
        "mitra_reject_test@example.com"
    )
    .execute(&pool)
    .await
    .ok();
    sqlx::query!(
        "DELETE FROM users WHERE email = $1",
        "admin_reject_test@example.com"
    )
    .execute(&pool)
    .await
    .ok();
}

#[tokio::test]
async fn test_duplicate_application_fails() {
    let config = get_test_config();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");
    let (service, pool) = setup_mitra_service(pool).await;

    let user_id = create_test_user(&pool, "mitra_dupe_test@example.com").await;

    let req = MitraApplyRequest {
        company_name: "Dupe Test PT".to_string(),
        company_type: Some("UD".to_string()),
        npwp: "9999888877776666".to_string(),
        annual_revenue: ">100M".to_string(),
        address: None,
        business_description: None,
        website_url: None,
        year_founded: None,
        key_products: None,
        export_markets: None,
    };

    // First apply
    let _ = service
        .apply(user_id, req.clone())
        .await
        .expect("First apply should succeed");

    // Second apply should fail
    let result = service.apply(user_id, req).await;
    assert!(result.is_err(), "Duplicate application should fail");

    // Cleanup
    sqlx::query!(
        "DELETE FROM users WHERE email = $1",
        "mitra_dupe_test@example.com"
    )
    .execute(&pool)
    .await
    .ok();
}
