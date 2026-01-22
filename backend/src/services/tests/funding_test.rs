use chrono::{Duration, Utc};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::Config;
use crate::models::{CreateInvoiceRequest, InvestRequest, RepayInvoiceRequest};
use crate::repository::{
    FundingRepository, InvoiceRepository, MitraRepository, RiskQuestionnaireRepository,
    TransactionRepository, UserRepository,
};
use crate::services::blockchain_service::BlockchainService;
use crate::services::email_service::EmailService;
use crate::services::escrow_service::EscrowService;
use crate::services::pinata_service::PinataService;
use crate::services::{FundingService, InvoiceService, MitraService};

use super::auth_test::get_test_config;

pub async fn setup_funding_service(
    pool: PgPool,
) -> (
    Arc<FundingService>,
    Arc<InvoiceService>,
    Arc<MitraService>,
    PgPool,
) {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true; // Enable test mode
    let config = Arc::new(config);

    let funding_repo = Arc::new(FundingRepository::new(pool.clone()));
    let invoice_repo = Arc::new(InvoiceRepository::new(pool.clone()));
    let tx_repo = Arc::new(TransactionRepository::new(pool.clone()));
    let user_repo = Arc::new(UserRepository::new(pool.clone()));
    let rq_repo = Arc::new(RiskQuestionnaireRepository::new(pool.clone()));
    let mitra_repo = Arc::new(MitraRepository::new(pool.clone()));

    let email_service = Arc::new(EmailService::new(config.clone()));
    let pinata_service = Arc::new(PinataService::new(config.clone()));
    let escrow_service = Arc::new(EscrowService::new());

    let blockchain_service = Arc::new(
        BlockchainService::new(
            config.clone(),
            invoice_repo.clone(),
            funding_repo.clone(),
            pinata_service.clone(),
        )
        .await
        .expect("Failed to init blockchain service"),
    );

    let funding_service = Arc::new(FundingService::new(
        funding_repo.clone(),
        invoice_repo.clone(),
        tx_repo,
        user_repo.clone(),
        rq_repo.clone(),
        email_service.clone(),
        escrow_service,
        blockchain_service.clone(),
        config.clone(),
    ));

    let mitra_service = Arc::new(MitraService::new(
        mitra_repo.clone(),
        user_repo.clone(),
        email_service.clone(),
        pinata_service.clone(),
    ));

    let invoice_service = Arc::new(InvoiceService::new(
        invoice_repo,
        funding_repo,
        user_repo,
        mitra_repo,
        pinata_service,
        config.clone(),
    ));

    (funding_service, invoice_service, mitra_service, pool)
}

// Helpers
async fn create_investor(pool: &PgPool, base_email: &str) -> Uuid {
    let user_id = Uuid::new_v4();
    let email = format!("{}_{}", user_id.simple(), base_email);
    let username = format!("investor_{}", user_id.simple());
    let wallet = format!("0xInvest_{}", user_id.simple());

    sqlx::query(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, wallet_address, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'investor', 'individual', true, true, $4, 1000000000)"#
    )
    .bind(user_id)
    .bind(email)
    .bind(username)
    .bind(wallet)
    .execute(pool)
    .await
    .expect("Failed to create investor");

    user_id
}

async fn create_mitra_and_invoice(
    pool: &PgPool,
    invoice_service: &Arc<InvoiceService>,
    base_email: &str,
) -> (Uuid, Uuid) {
    // 1. Create Mitra
    let user_id = Uuid::new_v4();
    let email = format!("{}_{}", user_id.simple(), base_email);
    let username = format!("mitra_{}", user_id.simple());
    let wallet = format!("0xMitra_{}", user_id.simple());

    sqlx::query(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, wallet_address, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'mitra', 'member_mitra', true, true, $4, 0)"#
    )
    .bind(user_id)
    .bind(email)
    .bind(username)
    .bind(wallet.clone()) // Clone wallet here
    .execute(pool)
    .await
    .expect("Failed to create mitra");

    // 2. Create Invoice
    // 2. Create Invoice
    let invoice_number = format!("INV-{}", Uuid::new_v4().simple());
    let invoice = sqlx::query_as::<_, crate::models::Invoice>(
        r#"
        INSERT INTO invoices (
            exporter_id, buyer_name, buyer_country, buyer_email, invoice_number,
            currency, amount, issue_date, due_date, description, status,
            exporter_wallet_address,
            priority_ratio, catalyst_ratio, funding_limit_percentage, is_repeat_buyer,
            is_insured, document_complete_score, grade, buffer_rate, funding_duration_days,
            priority_interest_rate, catalyst_interest_rate, idrx_amount, advance_percentage
        )
        VALUES (
            $1, 'Buyer PT', 'ID', 'buyer@test.com', $2,
            'IDRX', 100000000.0, NOW(), NOW() + INTERVAL '30 days', NULL, 'draft',
            $3,
            70.0, 30.0, 80.0, false,
            false, 100, 'A', 0.02, 30,
            12.0, 15.0, 100000000.0, 80.0
        )
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(invoice_number)
    .bind(wallet)
    .fetch_one(pool)
    .await
    .expect("Create invoice failed");

    // 3. Approve Invoice (Mocking admin approval usually done via handler)
    // We can use invoice_service not exposed method, or update DB directly.
    // Ideally use service method if available or repo.
    // Let's assume we need to manually transition it to approved then tokenized/funding for pool creation.
    // BUT FundingService tests usually start with an existing Pool.

    (user_id, invoice.id)
}

async fn setup_pool(
    pool: &PgPool,
    funding_service: &Arc<FundingService>,
    invoice_id: Uuid,
) -> Uuid {
    // Manually force invoice to 'approved' then 'tokenized' so we can create pool
    sqlx::query!(
        "UPDATE invoices SET status = 'tokenized' WHERE id = $1",
        invoice_id
    )
    .execute(pool)
    .await
    .expect("Failed to update invoice status");

    // Create NFT record stub
    sqlx::query!(
        r#"INSERT INTO invoice_nfts (invoice_id, token_id, contract_address, chain_id, owner_address, mint_tx_hash, metadata_uri, minted_at)
           VALUES ($1, 123, '0xContract', 8453, '0xOwner', '0xTx', 'ipfs://', NOW())"#,
        invoice_id
    )
    .execute(pool)
    .await
    .expect("Failed to create NFT record");

    let pool_obj = funding_service
        .create_pool(invoice_id)
        .await
        .expect("Failed to create pool");
    pool_obj.id
}

#[tokio::test]
async fn test_invest_limits_success() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true; // Use Test Mode
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;

    let (_, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra_invest_ok@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;
    let investor_id = create_investor(&pool, "investor_ok@test.com").await;

    // Target: 100M. Min: 10M. Max: 90M.
    // Invest 20M (20%) -> Should pass
    let req = InvestRequest {
        pool_id,
        amount: 20_000_000.0,
        tranche: "priority".to_string(),
        tnc_accepted: true,
        catalyst_consents: None,
        tx_hash: "0xTransferHash".to_string(),
    };

    let result = funding_service.invest(investor_id, req).await;
    assert!(
        result.is_ok(),
        "Investment 20% should succeed: {:?}",
        result.err()
    );

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE email = 'mitra_invest_ok@test.com'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM users WHERE email = 'investor_ok@test.com'")
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_invest_limits_fail_min() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;
    let (_, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra_invest_min@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;
    let investor_id = create_investor(&pool, "investor_min@test.com").await;

    // Target: 100M. Min: 10M.
    // Invest 5M (5%) -> Should fail
    let req = InvestRequest {
        pool_id,
        amount: 5_000_000.0,
        tranche: "priority".to_string(),
        tnc_accepted: true,
        catalyst_consents: None,
        tx_hash: "0xTransferHash".to_string(),
    };

    let result = funding_service.invest(investor_id, req).await;
    assert!(result.is_err(), "Investment 5% should fail");

    // Parse error checking? For now just ensure failure.

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE email = 'mitra_invest_min@test.com'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM users WHERE email = 'investor_min@test.com'")
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_invest_limits_fail_max() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;
    let (_, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra_invest_max@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;
    let investor_id = create_investor(&pool, "investor_max@test.com").await;

    // Target: 100M. Max: 90M.
    // Invest 95M (95%) -> Should fail
    let req = InvestRequest {
        pool_id,
        amount: 95_000_000.0,
        tranche: "priority".to_string(),
        tnc_accepted: true,
        catalyst_consents: None,
        tx_hash: "0xTransferHash".to_string(),
    };

    let result = funding_service.invest(investor_id, req).await;
    assert!(result.is_err(), "Investment 95% should fail");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE email = 'mitra_invest_max@test.com'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM users WHERE email = 'investor_max@test.com'")
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_repay_invoice_success() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;
    let (mitra_id, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra_repay@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;
    let investor_id = create_investor(&pool, "investor_repay@test.com").await;

    // 1. Invest 20M
    let req = InvestRequest {
        pool_id,
        amount: 20_000_000.0,
        tranche: "priority".to_string(),
        tnc_accepted: true,
        catalyst_consents: None,
        tx_hash: "0xTransferHash".to_string(),
    };
    funding_service
        .invest(investor_id, req)
        .await
        .expect("Investment failed");

    // 2. Set Status to Disbursed (Prereq for repayment)
    sqlx::query!(
        "UPDATE funding_pools SET status = 'disbursed' WHERE id = $1",
        pool_id
    )
    .execute(&pool)
    .await
    .ok();

    // 3. Repay 21M (Principal + Interest approx)
    let repay_req = RepayInvoiceRequest {
        tx_hash: "0xRepayHash".to_string(),
        amount: 21_000_000.0,
    };

    let result = funding_service
        .repay_invoice(mitra_id, invoice_id, repay_req)
        .await;
    assert!(
        result.is_ok(),
        "Repayment should succeed: {:?}",
        result.err()
    );

    // Verify status closed
    let pool_row = sqlx::query("SELECT status FROM funding_pools WHERE id = $1")
        .bind(pool_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let pool_status: String = pool_row.get("status");
    assert_eq!(pool_status, "closed");

    let inv_row = sqlx::query("SELECT status FROM invoices WHERE id = $1")
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let inv_status: String = inv_row.get("status");
    assert_eq!(inv_status, "repaid");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE email = 'mitra_repay@test.com'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM users WHERE email = 'investor_repay@test.com'")
        .execute(&pool)
        .await
        .ok();
}
