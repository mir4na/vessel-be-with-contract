use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::{BalanceTransaction, Transaction};

#[derive(Clone)]
pub struct TransactionRepository {
    pool: PgPool,
}

impl TransactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        invoice_id: Option<Uuid>,
        user_id: Option<Uuid>,
        tx_type: &str,
        amount: Decimal,
        currency: &str,
        tx_hash: Option<&str>,
        from_address: Option<&str>,
        to_address: Option<&str>,
        notes: Option<&str>,
    ) -> AppResult<Transaction> {
        let tx = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transactions (invoice_id, user_id, type, amount, currency, tx_hash, from_address, to_address, notes, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending')
            RETURNING *
            "#,
        )
        .bind(invoice_id)
        .bind(user_id)
        .bind(tx_type)
        .bind(amount)
        .bind(currency)
        .bind(tx_hash)
        .bind(from_address)
        .bind(to_address)
        .bind(notes)
        .fetch_one(&self.pool)
        .await?;

        Ok(tx)
    }

    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Transaction>> {
        let tx = sqlx::query_as::<_, Transaction>("SELECT * FROM transactions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(tx)
    }

    pub async fn find_by_tx_hash(&self, tx_hash: &str) -> AppResult<Option<Transaction>> {
        let tx = sqlx::query_as::<_, Transaction>("SELECT * FROM transactions WHERE tx_hash = $1")
            .bind(tx_hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(tx)
    }

    pub async fn find_by_user(
        &self,
        user_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<Transaction>, i64)> {
        let offset = (page - 1) * per_page;

        let txs = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(user_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok((txs, total.0))
    }

    pub async fn find_by_invoice(&self, invoice_id: Uuid) -> AppResult<Vec<Transaction>> {
        let txs = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE invoice_id = $1 ORDER BY created_at DESC",
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(txs)
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> AppResult<Transaction> {
        let tx = sqlx::query_as::<_, Transaction>(
            "UPDATE transactions SET status = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok(tx)
    }

    pub async fn confirm_with_block_info(
        &self,
        id: Uuid,
        block_number: i64,
        gas_used: i64,
    ) -> AppResult<Transaction> {
        let tx = sqlx::query_as::<_, Transaction>(
            r#"
            UPDATE transactions
            SET status = 'confirmed', block_number = $2, gas_used = $3, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(block_number)
        .bind(gas_used)
        .fetch_one(&self.pool)
        .await?;

        Ok(tx)
    }

    // Balance transaction methods
    pub async fn create_balance_transaction(
        &self,
        user_id: Uuid,
        tx_type: &str,
        amount: Decimal,
        balance_before: Decimal,
        balance_after: Decimal,
        reference_id: Option<Uuid>,
        reference_type: Option<&str>,
        description: Option<&str>,
    ) -> AppResult<BalanceTransaction> {
        let tx = sqlx::query_as::<_, BalanceTransaction>(
            r#"
            INSERT INTO balance_transactions (user_id, type, amount, balance_before, balance_after, reference_id, reference_type, description)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(tx_type)
        .bind(amount)
        .bind(balance_before)
        .bind(balance_after)
        .bind(reference_id)
        .bind(reference_type)
        .bind(description)
        .fetch_one(&self.pool)
        .await?;

        Ok(tx)
    }

    pub async fn find_balance_transactions_by_user(
        &self,
        user_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<BalanceTransaction>, i64)> {
        let offset = (page - 1) * per_page;

        let txs = sqlx::query_as::<_, BalanceTransaction>(
            "SELECT * FROM balance_transactions WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(user_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM balance_transactions WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;

        Ok((txs, total.0))
    }

    pub async fn get_platform_revenue(&self) -> AppResult<Decimal> {
        let revenue: (Decimal,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE type = 'platform_fee' AND status = 'confirmed'"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(revenue.0)
    }

    /// Create a verified blockchain transaction record
    /// Used for on-chain IDRX transactions that have been verified
    pub async fn create_blockchain_transaction(
        &self,
        user_id: Uuid,
        tx_type: &str,
        amount: Decimal,
        tx_hash: &str,
        block_number: i64,
        reference_id: Option<Uuid>,
        reference_type: Option<&str>,
        description: Option<&str>,
        explorer_url: &str,
    ) -> AppResult<Transaction> {
        // Create the transaction record
        let tx = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transactions (
                user_id, type, amount, currency, tx_hash, block_number,
                status, notes, explorer_url, invoice_id
            )
            VALUES ($1, $2, $3, 'IDRX', $4, $5, 'confirmed', $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(tx_type)
        .bind(amount)
        .bind(tx_hash)
        .bind(block_number)
        .bind(description)
        .bind(explorer_url)
        .bind(if reference_type == Some("pool") { reference_id } else { None })
        .fetch_one(&self.pool)
        .await?;

        Ok(tx)
    }

    /// Find blockchain transactions by user with explorer URLs
    pub async fn find_blockchain_transactions_by_user(
        &self,
        user_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<Transaction>, i64)> {
        let offset = (page - 1) * per_page;

        let txs = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE user_id = $1 AND tx_hash IS NOT NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(user_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE user_id = $1 AND tx_hash IS NOT NULL",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((txs, total.0))
    }

    /// Get all on-chain transactions for a pool (for transparency)
    pub async fn find_blockchain_transactions_by_pool(
        &self,
        pool_id: Uuid,
    ) -> AppResult<Vec<Transaction>> {
        // First find the invoice associated with the pool
        let invoice_id: (Uuid,) = sqlx::query_as("SELECT invoice_id FROM funding_pools WHERE id = $1")
            .bind(pool_id)
            .fetch_one(&self.pool)
            .await?;

        let txs = sqlx::query_as::<_, Transaction>(
            r#"
            SELECT * FROM transactions
            WHERE invoice_id = $1 AND tx_hash IS NOT NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(invoice_id.0)
        .fetch_all(&self.pool)
        .await?;

        Ok(txs)
    }
}
