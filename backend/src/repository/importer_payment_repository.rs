use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::ImporterPayment;

#[derive(Clone)]
pub struct ImporterPaymentRepository {
    pool: PgPool,
}

impl ImporterPaymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        invoice_id: Uuid,
        pool_id: Uuid,
        buyer_email: &str,
        buyer_name: &str,
        amount_due: Decimal,
        currency: &str,
        due_date: DateTime<Utc>,
    ) -> AppResult<ImporterPayment> {
        let payment = sqlx::query_as::<_, ImporterPayment>(
            r#"
            INSERT INTO importer_payments (invoice_id, pool_id, buyer_email, buyer_name, amount_due, currency, due_date, payment_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')
            RETURNING *
            "#,
        )
        .bind(invoice_id)
        .bind(pool_id)
        .bind(buyer_email)
        .bind(buyer_name)
        .bind(amount_due)
        .bind(currency)
        .bind(due_date)
        .fetch_one(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<ImporterPayment>> {
        let payment =
            sqlx::query_as::<_, ImporterPayment>("SELECT * FROM importer_payments WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(payment)
    }

    pub async fn find_by_invoice(&self, invoice_id: Uuid) -> AppResult<Option<ImporterPayment>> {
        let payment = sqlx::query_as::<_, ImporterPayment>(
            "SELECT * FROM importer_payments WHERE invoice_id = $1",
        )
        .bind(invoice_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn find_by_pool(&self, pool_id: Uuid) -> AppResult<Option<ImporterPayment>> {
        let payment = sqlx::query_as::<_, ImporterPayment>(
            "SELECT * FROM importer_payments WHERE pool_id = $1",
        )
        .bind(pool_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn update_payment(
        &self,
        id: Uuid,
        amount_paid: Decimal,
        tx_hash: &str,
    ) -> AppResult<ImporterPayment> {
        let payment = sqlx::query_as::<_, ImporterPayment>(
            r#"
            UPDATE importer_payments
            SET amount_paid = amount_paid + $2, tx_hash = $3, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(amount_paid)
        .bind(tx_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn mark_paid(&self, id: Uuid, tx_hash: &str) -> AppResult<ImporterPayment> {
        let payment = sqlx::query_as::<_, ImporterPayment>(
            r#"
            UPDATE importer_payments
            SET payment_status = 'paid', paid_at = NOW(), tx_hash = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tx_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn mark_overdue(&self, id: Uuid) -> AppResult<ImporterPayment> {
        let payment = sqlx::query_as::<_, ImporterPayment>(
            "UPDATE importer_payments SET payment_status = 'overdue', updated_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn find_pending_overdue(&self) -> AppResult<Vec<ImporterPayment>> {
        let payments = sqlx::query_as::<_, ImporterPayment>(
            "SELECT * FROM importer_payments WHERE payment_status = 'pending' AND due_date < NOW()",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(payments)
    }
}
