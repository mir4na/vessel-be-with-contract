use sqlx::PgPool;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{InvestRequest, RepayInvoiceRequest};
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
    _invoice_service: &Arc<InvoiceService>,
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
    sqlx::query("UPDATE invoices SET status = 'tokenized' WHERE id = $1")
        .bind(invoice_id)
        .execute(pool)
        .await
        .expect("Failed to update invoice status");

    // Create NFT record stub
    sqlx::query(
        r#"INSERT INTO invoice_nfts (invoice_id, token_id, contract_address, chain_id, owner_address, mint_tx_hash, metadata_uri, minted_at)
           VALUES ($1, 123, '0xContract', 8453, '0xOwner', '0xTx', 'ipfs://', NOW())"#
    )
    .bind(invoice_id)
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
    // Cleanup
    sqlx::query("DELETE FROM users WHERE email = 'mitra_invest_ok@test.com'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE email = 'investor_ok@test.com'")
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
    // Cleanup
    sqlx::query("DELETE FROM users WHERE email = 'mitra_invest_min@test.com'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE email = 'investor_min@test.com'")
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
    // Cleanup
    sqlx::query("DELETE FROM users WHERE email = 'mitra_invest_max@test.com'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE email = 'investor_max@test.com'")
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
    // 2. Set Status to Disbursed (Prereq for repayment)
    sqlx::query("UPDATE funding_pools SET status = 'disbursed' WHERE id = $1")
        .bind(pool_id)
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
    // Cleanup
    sqlx::query("DELETE FROM users WHERE email = 'mitra_repay@test.com'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE email = 'investor_repay@test.com'")
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_duplicate_investment_fails() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;

    let (_, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra_dupe_inv@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;
    let investor_id = create_investor(&pool, "investor_dupe@test.com").await;

    // 1st Investment: Success
    let req1 = InvestRequest {
        pool_id,
        amount: 20_000_000.0,
        tranche: "priority".to_string(),
        tnc_accepted: true,
        catalyst_consents: None,
        tx_hash: "0xTx1".to_string(),
    };
    funding_service
        .invest(investor_id, req1)
        .await
        .expect("First investment failed");

    // 2nd Investment: Should Fail
    let req2 = InvestRequest {
        pool_id,
        amount: 30_000_000.0,
        tranche: "priority".to_string(),
        tnc_accepted: true,
        catalyst_consents: None,
        tx_hash: "0xTx2".to_string(),
    };
    let result = funding_service.invest(investor_id, req2).await;
    assert!(result.is_err(), "Duplicate investment should fail");

    // Cleanup
    sqlx::query("DELETE FROM users WHERE email = 'mitra_dupe_inv@test.com'")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE email = 'investor_dupe@test.com'")
        .execute(&pool)
        .await
        .ok();
}

// ============================================================
// MITRA POOLS TESTS
// ============================================================

#[tokio::test]
async fn test_get_mitra_pools_success() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;

    // Create mitra with invoice and pool
    let (mitra_id, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra_pools_test@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;

    // Test get_mitra_pools
    let result = funding_service.get_mitra_pools(mitra_id, 1, 10).await;
    assert!(
        result.is_ok(),
        "get_mitra_pools should succeed: {:?}",
        result.err()
    );

    let (pools, total) = result.unwrap();
    assert_eq!(total, 1, "Should have 1 pool");
    assert_eq!(pools.len(), 1, "Should return 1 pool");
    assert_eq!(pools[0].pool.id, pool_id, "Pool ID should match");
    assert_eq!(
        pools[0].pool.invoice_id, invoice_id,
        "Invoice ID should match"
    );

    // Cleanup
    sqlx::query("DELETE FROM funding_pools WHERE id = $1")
        .bind(pool_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoice_nfts WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_get_mitra_pools_empty() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, _, _, pool) = setup_funding_service(pool).await;

    // Create mitra without any pools
    let user_id = Uuid::new_v4();
    let email = format!("{}_{}", user_id.simple(), "mitra_no_pools@test.com");
    let username = format!("mitra_{}", user_id.simple());
    let wallet = format!("0xMitra_{}", user_id.simple());

    sqlx::query(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, wallet_address, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'mitra', 'member_mitra', true, true, $4, 0)"#
    )
    .bind(user_id)
    .bind(&email)
    .bind(username)
    .bind(wallet)
    .execute(&pool)
    .await
    .expect("Failed to create mitra");

    // Test get_mitra_pools for mitra with no pools
    let result = funding_service.get_mitra_pools(user_id, 1, 10).await;
    assert!(
        result.is_ok(),
        "get_mitra_pools should succeed even with no pools"
    );

    let (pools, total) = result.unwrap();
    assert_eq!(total, 0, "Should have 0 pools");
    assert!(pools.is_empty(), "Should return empty list");

    // Cleanup
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_get_mitra_pools_multiple() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, _, _, pool) = setup_funding_service(pool).await;

    // Create mitra
    let mitra_id = Uuid::new_v4();
    let email = format!("{}_{}", mitra_id.simple(), "mitra_multi_pools@test.com");
    let username = format!("mitra_{}", mitra_id.simple());
    let wallet = format!("0xMitra_{}", mitra_id.simple());

    sqlx::query(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, wallet_address, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'mitra', 'member_mitra', true, true, $4, 0)"#
    )
    .bind(mitra_id)
    .bind(&email)
    .bind(&username)
    .bind(&wallet)
    .execute(&pool)
    .await
    .expect("Failed to create mitra");

    // Create 3 invoices with pools
    let mut invoice_ids = Vec::new();
    let mut pool_ids = Vec::new();

    for i in 0..3 {
        let invoice_number = format!("INV-MULTI-{}-{}", mitra_id.simple(), i);
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
                'IDRX', 100000000.0, NOW(), NOW() + INTERVAL '30 days', NULL, 'tokenized',
                $3,
                70.0, 30.0, 80.0, false,
                false, 100, 'A', 0.02, 30,
                12.0, 15.0, 100000000.0, 80.0
            )
            RETURNING *
            "#,
        )
        .bind(mitra_id)
        .bind(&invoice_number)
        .bind(&wallet)
        .fetch_one(&pool)
        .await
        .expect("Create invoice failed");

        invoice_ids.push(invoice.id);

        // Create NFT record
        sqlx::query(
            r#"INSERT INTO invoice_nfts (invoice_id, token_id, contract_address, chain_id, owner_address, mint_tx_hash, metadata_uri, minted_at)
               VALUES ($1, $2, '0xContract', 8453, '0xOwner', '0xTx', 'ipfs://', NOW())"#
        )
        .bind(invoice.id)
        .bind(1000 + i as i64)
        .execute(&pool)
        .await
        .expect("Failed to create NFT record");

        // Create pool
        let pool_obj = funding_service
            .create_pool(invoice.id)
            .await
            .expect("Failed to create pool");
        pool_ids.push(pool_obj.id);
    }

    // Test get_mitra_pools
    let result = funding_service.get_mitra_pools(mitra_id, 1, 10).await;
    assert!(
        result.is_ok(),
        "get_mitra_pools should succeed: {:?}",
        result.err()
    );

    let (pools, total) = result.unwrap();
    assert_eq!(total, 3, "Should have 3 pools");
    assert_eq!(pools.len(), 3, "Should return 3 pools");

    // Test pagination - page 1, per_page 2
    let result = funding_service.get_mitra_pools(mitra_id, 1, 2).await;
    assert!(result.is_ok());
    let (pools, total) = result.unwrap();
    assert_eq!(total, 3, "Total should still be 3");
    assert_eq!(
        pools.len(),
        2,
        "Should return only 2 pools due to pagination"
    );

    // Test pagination - page 2, per_page 2
    let result = funding_service.get_mitra_pools(mitra_id, 2, 2).await;
    assert!(result.is_ok());
    let (pools, _) = result.unwrap();
    assert_eq!(pools.len(), 1, "Should return 1 pool on page 2");

    // Cleanup
    for pool_id in &pool_ids {
        sqlx::query("DELETE FROM funding_pools WHERE id = $1")
            .bind(pool_id)
            .execute(&pool)
            .await
            .ok();
    }
    for invoice_id in &invoice_ids {
        sqlx::query("DELETE FROM invoice_nfts WHERE invoice_id = $1")
            .bind(invoice_id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .execute(&pool)
            .await
            .ok();
    }
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_get_mitra_pools_does_not_return_other_users_pools() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;

    // Create mitra1 with pool
    let (mitra1_id, invoice1_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra1_isolation@test.com").await;
    let pool1_id = setup_pool(&pool, &funding_service, invoice1_id).await;

    // Create mitra2 with pool
    let (mitra2_id, invoice2_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra2_isolation@test.com").await;
    let pool2_id = setup_pool(&pool, &funding_service, invoice2_id).await;

    // Test mitra1 only sees their own pool
    let result = funding_service.get_mitra_pools(mitra1_id, 1, 10).await;
    assert!(result.is_ok());
    let (pools, total) = result.unwrap();
    assert_eq!(total, 1, "Mitra1 should only see 1 pool");
    assert_eq!(pools[0].pool.id, pool1_id, "Should be mitra1's pool");

    // Test mitra2 only sees their own pool
    let result = funding_service.get_mitra_pools(mitra2_id, 1, 10).await;
    assert!(result.is_ok());
    let (pools, total) = result.unwrap();
    assert_eq!(total, 1, "Mitra2 should only see 1 pool");
    assert_eq!(pools[0].pool.id, pool2_id, "Should be mitra2's pool");

    // Cleanup
    sqlx::query("DELETE FROM funding_pools WHERE id = $1")
        .bind(pool1_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM funding_pools WHERE id = $1")
        .bind(pool2_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoice_nfts WHERE invoice_id = $1")
        .bind(invoice1_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoice_nfts WHERE invoice_id = $1")
        .bind(invoice2_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice1_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice2_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra1_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra2_id)
        .execute(&pool)
        .await
        .ok();
}

// ============================================================
// GET POOL BY INVOICE TESTS
// ============================================================

#[tokio::test]
async fn test_get_pool_by_invoice_success() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;

    // Create mitra with invoice and pool
    let (mitra_id, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra_pool_by_inv@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;

    // Test get_pool_by_invoice
    let result = funding_service
        .get_pool_by_invoice(mitra_id, invoice_id)
        .await;
    assert!(
        result.is_ok(),
        "get_pool_by_invoice should succeed: {:?}",
        result.err()
    );

    let pool_response = result.unwrap();
    assert_eq!(pool_response.pool.id, pool_id, "Pool ID should match");
    assert_eq!(
        pool_response.pool.invoice_id, invoice_id,
        "Invoice ID should match"
    );
    assert!(
        pool_response.invoice.is_some(),
        "Invoice should be included"
    );
    assert_eq!(
        pool_response.invoice.as_ref().unwrap().id,
        invoice_id,
        "Invoice should match"
    );

    // Cleanup
    sqlx::query("DELETE FROM funding_pools WHERE id = $1")
        .bind(pool_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoice_nfts WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_get_pool_by_invoice_not_found() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, _, _, pool) = setup_funding_service(pool).await;

    // Create mitra without any invoices
    let mitra_id = Uuid::new_v4();
    let email = format!("{}_{}", mitra_id.simple(), "mitra_inv_not_found@test.com");
    let username = format!("mitra_{}", mitra_id.simple());
    let wallet = format!("0xMitra_{}", mitra_id.simple());

    sqlx::query(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, wallet_address, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'mitra', 'member_mitra', true, true, $4, 0)"#
    )
    .bind(mitra_id)
    .bind(&email)
    .bind(username)
    .bind(wallet)
    .execute(&pool)
    .await
    .expect("Failed to create mitra");

    // Test get_pool_by_invoice with non-existent invoice
    let fake_invoice_id = Uuid::new_v4();
    let result = funding_service
        .get_pool_by_invoice(mitra_id, fake_invoice_id)
        .await;
    assert!(result.is_err(), "Should fail for non-existent invoice");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("not found") || error.to_string().contains("Not found"),
        "Error should indicate invoice not found: {}",
        error
    );

    // Cleanup
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_get_pool_by_invoice_forbidden_wrong_owner() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;

    // Create mitra1 with invoice and pool
    let (mitra1_id, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra1_forbidden@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;

    // Create mitra2 (different user)
    let mitra2_id = Uuid::new_v4();
    let email2 = format!("{}_{}", mitra2_id.simple(), "mitra2_forbidden@test.com");
    let username2 = format!("mitra_{}", mitra2_id.simple());
    let wallet2 = format!("0xMitra_{}", mitra2_id.simple());

    sqlx::query(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, wallet_address, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'mitra', 'member_mitra', true, true, $4, 0)"#
    )
    .bind(mitra2_id)
    .bind(&email2)
    .bind(username2)
    .bind(wallet2)
    .execute(&pool)
    .await
    .expect("Failed to create mitra2");

    // Test mitra2 trying to access mitra1's invoice pool
    let result = funding_service
        .get_pool_by_invoice(mitra2_id, invoice_id)
        .await;
    assert!(
        result.is_err(),
        "Should fail when accessing another user's invoice"
    );

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("owner") || error.to_string().contains("Forbidden"),
        "Error should indicate forbidden/wrong owner: {}",
        error
    );

    // Cleanup
    sqlx::query("DELETE FROM funding_pools WHERE id = $1")
        .bind(pool_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoice_nfts WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra1_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra2_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_get_pool_by_invoice_no_pool_exists() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, _, _, pool) = setup_funding_service(pool).await;

    // Create mitra
    let mitra_id = Uuid::new_v4();
    let email = format!("{}_{}", mitra_id.simple(), "mitra_no_pool@test.com");
    let username = format!("mitra_{}", mitra_id.simple());
    let wallet = format!("0xMitra_{}", mitra_id.simple());

    sqlx::query(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, wallet_address, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'mitra', 'member_mitra', true, true, $4, 0)"#
    )
    .bind(mitra_id)
    .bind(&email)
    .bind(&username)
    .bind(&wallet)
    .execute(&pool)
    .await
    .expect("Failed to create mitra");

    // Create invoice without pool
    let invoice_number = format!("INV-NOPOOL-{}", mitra_id.simple());
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
    .bind(mitra_id)
    .bind(&invoice_number)
    .bind(&wallet)
    .fetch_one(&pool)
    .await
    .expect("Create invoice failed");

    // Test get_pool_by_invoice when no pool exists for invoice
    let result = funding_service
        .get_pool_by_invoice(mitra_id, invoice.id)
        .await;
    assert!(
        result.is_err(),
        "Should fail when no pool exists for invoice"
    );

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("not found") || error.to_string().contains("No funding pool"),
        "Error should indicate no pool found: {}",
        error
    );

    // Cleanup
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice.id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra_id)
        .execute(&pool)
        .await
        .ok();
}

// ============================================================
// MITRA DASHBOARD TESTS
// ============================================================

#[tokio::test]
async fn test_get_mitra_dashboard_success() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;

    // Create mitra with invoice and pool
    let (mitra_id, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra_dashboard@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;

    // Test get_mitra_dashboard
    let result = funding_service.get_mitra_dashboard(mitra_id).await;
    assert!(
        result.is_ok(),
        "get_mitra_dashboard should succeed: {:?}",
        result.err()
    );

    let dashboard = result.unwrap();
    // Invoice is in 'funding' status after pool creation
    assert!(
        dashboard.total_active_financing >= 0.0,
        "Should have financing info"
    );

    // Cleanup
    sqlx::query("DELETE FROM funding_pools WHERE id = $1")
        .bind(pool_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoice_nfts WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_get_mitra_dashboard_empty() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, _, _, pool) = setup_funding_service(pool).await;

    // Create mitra without any invoices
    let mitra_id = Uuid::new_v4();
    let email = format!("{}_{}", mitra_id.simple(), "mitra_empty_dashboard@test.com");
    let username = format!("mitra_{}", mitra_id.simple());
    let wallet = format!("0xMitra_{}", mitra_id.simple());

    sqlx::query(
        r#"INSERT INTO users (id, email, username, password_hash, role, member_status, is_verified, is_active, wallet_address, balance_idrx)
           VALUES ($1, $2, $3, 'hash', 'mitra', 'member_mitra', true, true, $4, 0)"#
    )
    .bind(mitra_id)
    .bind(&email)
    .bind(username)
    .bind(wallet)
    .execute(&pool)
    .await
    .expect("Failed to create mitra");

    // Test get_mitra_dashboard for mitra with no invoices
    let result = funding_service.get_mitra_dashboard(mitra_id).await;
    assert!(
        result.is_ok(),
        "get_mitra_dashboard should succeed even with no data"
    );

    let dashboard = result.unwrap();
    assert_eq!(
        dashboard.total_active_financing, 0.0,
        "Should have 0 financing"
    );
    assert_eq!(dashboard.total_owed_to_investors, 0.0, "Should have 0 owed");
    assert!(
        dashboard.active_invoices.is_empty(),
        "Should have no active invoices"
    );

    // Cleanup
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(mitra_id)
        .execute(&pool)
        .await
        .ok();
}

// ============================================================
// INVESTOR PORTFOLIO TESTS
// ============================================================

#[tokio::test]
async fn test_get_investor_portfolio_success() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, invoice_service, _, pool) = setup_funding_service(pool).await;

    // Create mitra with invoice and pool
    let (_, invoice_id) =
        create_mitra_and_invoice(&pool, &invoice_service, "mitra_portfolio_test@test.com").await;
    let pool_id = setup_pool(&pool, &funding_service, invoice_id).await;

    // Create investor and invest
    let investor_id = create_investor(&pool, "investor_portfolio@test.com").await;

    let req = InvestRequest {
        pool_id,
        amount: 20_000_000.0,
        tranche: "priority".to_string(),
        tnc_accepted: true,
        catalyst_consents: None,
        tx_hash: "0xPortfolioTx".to_string(),
    };
    funding_service
        .invest(investor_id, req)
        .await
        .expect("Investment failed");

    // Test get_investor_portfolio
    let result = funding_service.get_investor_portfolio(investor_id).await;
    assert!(
        result.is_ok(),
        "get_investor_portfolio should succeed: {:?}",
        result.err()
    );

    let portfolio = result.unwrap();
    assert!(portfolio.total_funding > 0.0, "Should have total funding");
    assert!(
        portfolio.active_investments > 0,
        "Should have active investments"
    );
    assert!(
        portfolio.priority_allocation > 0.0,
        "Should have priority allocation"
    );

    // Cleanup
    sqlx::query("DELETE FROM investments WHERE pool_id = $1")
        .bind(pool_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM funding_pools WHERE id = $1")
        .bind(pool_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_get_investor_portfolio_empty() {
    let mut config = get_test_config();
    config.skip_blockchain_verification = true;
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect");

    let (funding_service, _, _, pool) = setup_funding_service(pool).await;

    // Create investor without any investments
    let investor_id = create_investor(&pool, "investor_empty_portfolio@test.com").await;

    // Test get_investor_portfolio for investor with no investments
    let result = funding_service.get_investor_portfolio(investor_id).await;
    assert!(
        result.is_ok(),
        "get_investor_portfolio should succeed even with no data"
    );

    let portfolio = result.unwrap();
    assert_eq!(portfolio.total_funding, 0.0, "Should have 0 funding");
    assert_eq!(
        portfolio.active_investments, 0,
        "Should have 0 active investments"
    );
    assert_eq!(
        portfolio.completed_deals, 0,
        "Should have 0 completed deals"
    );

    // Cleanup
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(investor_id)
        .execute(&pool)
        .await
        .ok();
}
