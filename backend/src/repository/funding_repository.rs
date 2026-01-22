use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::{FundingPool, Investment};

#[derive(Clone)]
pub struct FundingRepository {
    pool: PgPool,
}

impl FundingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_pool(
        &self,
        invoice_id: Uuid,
        target_amount: Decimal,
        priority_target: Decimal,
        catalyst_target: Decimal,
        priority_interest_rate: Decimal,
        catalyst_interest_rate: Decimal,
        deadline: DateTime<Utc>,
    ) -> AppResult<FundingPool> {
        let pool = sqlx::query_as::<_, FundingPool>(
            r#"
            INSERT INTO funding_pools (
                invoice_id, target_amount, priority_target, catalyst_target,
                priority_interest_rate, catalyst_interest_rate, deadline,
                status, opened_at, pool_currency
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'open', NOW(), 'IDRX')
            RETURNING *
            "#,
        )
        .bind(invoice_id)
        .bind(target_amount)
        .bind(priority_target)
        .bind(catalyst_target)
        .bind(priority_interest_rate)
        .bind(catalyst_interest_rate)
        .bind(deadline)
        .fetch_one(&self.pool)
        .await?;

        Ok(pool)
    }

    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<FundingPool>> {
        let pool = sqlx::query_as::<_, FundingPool>("SELECT * FROM funding_pools WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(pool)
    }

    pub async fn find_by_invoice(&self, invoice_id: Uuid) -> AppResult<Option<FundingPool>> {
        let pool =
            sqlx::query_as::<_, FundingPool>("SELECT * FROM funding_pools WHERE invoice_id = $1")
                .bind(invoice_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(pool)
    }

    pub async fn find_open_pools(
        &self,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<FundingPool>, i64)> {
        let offset = (page - 1) * per_page;

        let pools = sqlx::query_as::<_, FundingPool>(
            "SELECT * FROM funding_pools WHERE status = 'open' ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM funding_pools WHERE status = 'open'")
                .fetch_one(&self.pool)
                .await?;

        Ok((pools, total.0))
    }

    pub async fn find_all(&self, page: i32, per_page: i32) -> AppResult<(Vec<FundingPool>, i64)> {
        let offset = (page - 1) * per_page;

        let pools = sqlx::query_as::<_, FundingPool>(
            "SELECT * FROM funding_pools ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM funding_pools")
            .fetch_one(&self.pool)
            .await?;

        Ok((pools, total.0))
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> AppResult<FundingPool> {
        let pool = sqlx::query_as::<_, FundingPool>(
            "UPDATE funding_pools SET status = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok(pool)
    }

    pub async fn update_funded_amount(
        &self,
        id: Uuid,
        funded_amount: Decimal,
        priority_funded: Decimal,
        catalyst_funded: Decimal,
        investor_count: i32,
    ) -> AppResult<FundingPool> {
        let pool = sqlx::query_as::<_, FundingPool>(
            r#"
            UPDATE funding_pools
            SET funded_amount = $2, priority_funded = $3, catalyst_funded = $4,
                investor_count = $5, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(funded_amount)
        .bind(priority_funded)
        .bind(catalyst_funded)
        .bind(investor_count)
        .fetch_one(&self.pool)
        .await?;

        Ok(pool)
    }

    pub async fn set_filled(&self, id: Uuid) -> AppResult<FundingPool> {
        let pool = sqlx::query_as::<_, FundingPool>(
            "UPDATE funding_pools SET status = 'filled', filled_at = NOW(), updated_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(pool)
    }

    pub async fn set_disbursed(&self, id: Uuid) -> AppResult<FundingPool> {
        let pool = sqlx::query_as::<_, FundingPool>(
            "UPDATE funding_pools SET status = 'disbursed', disbursed_at = NOW(), updated_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(pool)
    }

    pub async fn set_closed(&self, id: Uuid) -> AppResult<FundingPool> {
        let pool = sqlx::query_as::<_, FundingPool>(
            "UPDATE funding_pools SET status = 'closed', closed_at = NOW(), updated_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(pool)
    }

    // Investment methods
    pub async fn create_investment(
        &self,
        pool_id: Uuid,
        investor_id: Uuid,
        amount: Decimal,
        expected_return: Decimal,
        tranche: &str,
        tx_hash: &str,
    ) -> AppResult<Investment> {
        let investment = sqlx::query_as::<_, Investment>(
            r#"
            INSERT INTO investments (pool_id, investor_id, amount, expected_return, tranche, tx_hash, status)
            VALUES ($1, $2, $3, $4, $5, $6, 'active')
            RETURNING *
            "#,
        )
        .bind(pool_id)
        .bind(investor_id)
        .bind(amount)
        .bind(expected_return)
        .bind(tranche)
        .bind(tx_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(investment)
    }

    pub async fn find_investment_by_pool_and_investor(
        &self,
        pool_id: Uuid,
        investor_id: Uuid,
    ) -> AppResult<Option<Investment>> {
        let investment = sqlx::query_as::<_, Investment>(
            "SELECT * FROM investments WHERE pool_id = $1 AND investor_id = $2",
        )
        .bind(pool_id)
        .bind(investor_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(investment)
    }

    pub async fn find_investment_by_id(&self, id: Uuid) -> AppResult<Option<Investment>> {
        let investment = sqlx::query_as::<_, Investment>("SELECT * FROM investments WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(investment)
    }

    pub async fn find_investments_by_pool(&self, pool_id: Uuid) -> AppResult<Vec<Investment>> {
        let investments = sqlx::query_as::<_, Investment>(
            "SELECT * FROM investments WHERE pool_id = $1 ORDER BY invested_at DESC",
        )
        .bind(pool_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(investments)
    }

    pub async fn find_investments_by_investor(
        &self,
        investor_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<Investment>, i64)> {
        let offset = (page - 1) * per_page;

        let investments = sqlx::query_as::<_, Investment>(
            "SELECT * FROM investments WHERE investor_id = $1 ORDER BY invested_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(investor_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM investments WHERE investor_id = $1")
                .bind(investor_id)
                .fetch_one(&self.pool)
                .await?;

        Ok((investments, total.0))
    }

    pub async fn find_active_investments_by_investor(
        &self,
        investor_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<Investment>, i64)> {
        let offset = (page - 1) * per_page;

        let investments = sqlx::query_as::<_, Investment>(
            "SELECT * FROM investments WHERE investor_id = $1 AND status = 'active' ORDER BY invested_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(investor_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM investments WHERE investor_id = $1 AND status = 'active'",
        )
        .bind(investor_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((investments, total.0))
    }

    pub async fn update_investment_status(&self, id: Uuid, status: &str) -> AppResult<Investment> {
        let investment = sqlx::query_as::<_, Investment>(
            "UPDATE investments SET status = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok(investment)
    }

    pub async fn set_investment_repaid(
        &self,
        id: Uuid,
        actual_return: Decimal,
        return_tx_hash: &str,
    ) -> AppResult<Investment> {
        let investment = sqlx::query_as::<_, Investment>(
            r#"
            UPDATE investments
            SET status = 'repaid', actual_return = $2, return_tx_hash = $3, repaid_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(actual_return)
        .bind(return_tx_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(investment)
    }

    pub async fn get_investor_portfolio_stats(
        &self,
        investor_id: Uuid,
    ) -> AppResult<(Decimal, Decimal, Decimal, Decimal, Decimal, i64, i64)> {
        let stats: (Decimal, Decimal, Decimal, Decimal, Decimal, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN status = 'active' THEN amount ELSE 0 END), 0) as total_funding,
                COALESCE(SUM(CASE WHEN status = 'active' THEN expected_return - amount ELSE 0 END), 0) as total_expected_gain,
                COALESCE(SUM(CASE WHEN status = 'repaid' THEN actual_return - amount ELSE 0 END), 0) as total_realized_gain,
                COALESCE(SUM(CASE WHEN tranche = 'priority' AND status = 'active' THEN amount ELSE 0 END), 0) as priority_allocation,
                COALESCE(SUM(CASE WHEN tranche = 'catalyst' AND status = 'active' THEN amount ELSE 0 END), 0) as catalyst_allocation,
                COUNT(CASE WHEN status = 'active' THEN 1 END) as active_count,
                COUNT(CASE WHEN status = 'repaid' THEN 1 END) as completed_count
            FROM investments
            WHERE investor_id = $1
            "#,
        )
        .bind(investor_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    pub async fn count_investors_in_pool(&self, pool_id: Uuid) -> AppResult<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT investor_id) FROM investments WHERE pool_id = $1",
        )
        .bind(pool_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }
}
