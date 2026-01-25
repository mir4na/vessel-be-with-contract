use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::{Invoice, InvoiceDocument, InvoiceNft};

#[derive(Clone)]
pub struct InvoiceRepository {
    pool: PgPool,
}

impl InvoiceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        exporter_id: Uuid,
        buyer_name: &str,
        buyer_country: &str,
        buyer_email: Option<&str>,
        invoice_number: &str,
        currency: &str,
        amount: Decimal,
        issue_date: NaiveDate,
        due_date: NaiveDate,
        description: Option<&str>,
        exporter_wallet_address: &str,
    ) -> AppResult<Invoice> {
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            INSERT INTO invoices (
                exporter_id, buyer_name, buyer_country, buyer_email, invoice_number,
                currency, amount, issue_date, due_date, description, status,
                exporter_wallet_address
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'draft', $11)
            RETURNING *
            "#,
        )
        .bind(exporter_id)
        .bind(buyer_name)
        .bind(buyer_country)
        .bind(buyer_email)
        .bind(invoice_number)
        .bind(currency)
        .bind(amount)
        .bind(issue_date)
        .bind(due_date)
        .bind(description)
        .bind(exporter_wallet_address)
        .fetch_one(&self.pool)
        .await?;

        Ok(invoice)
    }

    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Invoice>> {
        let invoice = sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(invoice)
    }

    pub async fn find_by_exporter(
        &self,
        exporter_id: Uuid,
        status: Option<String>,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<Invoice>, i64)> {
        let offset = (page - 1) * per_page;

        let invoices = if let Some(ref s) = status {
            sqlx::query_as::<_, Invoice>(
                "SELECT * FROM invoices WHERE exporter_id = $1 AND status = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            )
            .bind(exporter_id)
            .bind(s)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Invoice>(
                "SELECT * FROM invoices WHERE exporter_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(exporter_id)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        let total: (i64,) = if let Some(ref s) = status {
             sqlx::query_as("SELECT COUNT(*) FROM invoices WHERE exporter_id = $1 AND status = $2")
                .bind(exporter_id)
                .bind(s)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM invoices WHERE exporter_id = $1")
                .bind(exporter_id)
                .fetch_one(&self.pool)
                .await?
        };

        Ok((invoices, total.0))
    }

    pub async fn find_by_status(
        &self,
        status: &str,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<Invoice>, i64)> {
        let offset = (page - 1) * per_page;

        let invoices = sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(status)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM invoices WHERE status = $1")
            .bind(status)
            .fetch_one(&self.pool)
            .await?;

        Ok((invoices, total.0))
    }

    pub async fn find_fundable(&self, page: i32, per_page: i32) -> AppResult<(Vec<Invoice>, i64)> {
        let offset = (page - 1) * per_page;

        let invoices = sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE status IN ('approved', 'tokenized', 'funding') ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM invoices WHERE status IN ('approved', 'tokenized', 'funding')",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((invoices, total.0))
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> AppResult<Invoice> {
        let invoice = sqlx::query_as::<_, Invoice>(
            "UPDATE invoices SET status = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok(invoice)
    }

    pub async fn update_document_score(&self, id: Uuid, score: i32) -> AppResult<Invoice> {
        let invoice = sqlx::query_as::<_, Invoice>(
            "UPDATE invoices SET document_complete_score = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(score)
        .fetch_one(&self.pool)
        .await?;

        Ok(invoice)
    }

    pub async fn update_grade(
        &self,
        id: Uuid,
        grade: &str,
        grade_score: i32,
        funding_limit: Decimal,
    ) -> AppResult<Invoice> {
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoices
            SET grade = $2, grade_score = $3, funding_limit_percentage = $4, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(grade)
        .bind(grade_score)
        .bind(funding_limit)
        .fetch_one(&self.pool)
        .await?;

        Ok(invoice)
    }

    pub async fn update_interest_rates(
        &self,
        id: Uuid,
        priority_rate: Decimal,
        catalyst_rate: Decimal,
    ) -> AppResult<Invoice> {
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoices
            SET priority_interest_rate = $2, catalyst_interest_rate = $3, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(priority_rate)
        .bind(catalyst_rate)
        .fetch_one(&self.pool)
        .await?;

        Ok(invoice)
    }

    pub async fn set_repeat_buyer(&self, id: Uuid, is_repeat: bool) -> AppResult<Invoice> {
        let invoice = sqlx::query_as::<_, Invoice>(
            "UPDATE invoices SET is_repeat_buyer = $2, updated_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(is_repeat)
        .fetch_one(&self.pool)
        .await?;

        Ok(invoice)
    }

    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM invoices WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn count_by_buyer_name(&self, exporter_id: Uuid, buyer_name: &str) -> AppResult<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM invoices WHERE exporter_id = $1 AND LOWER(buyer_name) = LOWER($2) AND status IN ('repaid', 'funded')"
        )
        .bind(exporter_id)
        .bind(buyer_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    // Document methods
    pub async fn create_document(
        &self,
        invoice_id: Uuid,
        document_type: &str,
        file_name: &str,
        file_url: &str,
        file_hash: &str,
        file_size: i32,
    ) -> AppResult<InvoiceDocument> {
        let doc = sqlx::query_as::<_, InvoiceDocument>(
            r#"
            INSERT INTO invoice_documents (invoice_id, document_type, file_name, file_url, file_hash, file_size)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(invoice_id)
        .bind(document_type)
        .bind(file_name)
        .bind(file_url)
        .bind(file_hash)
        .bind(file_size)
        .fetch_one(&self.pool)
        .await?;

        Ok(doc)
    }

    pub async fn find_documents_by_invoice(
        &self,
        invoice_id: Uuid,
    ) -> AppResult<Vec<InvoiceDocument>> {
        let docs = sqlx::query_as::<_, InvoiceDocument>(
            "SELECT * FROM invoice_documents WHERE invoice_id = $1 ORDER BY uploaded_at DESC",
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(docs)
    }

    // NFT methods
    pub async fn create_nft(
        &self,
        invoice_id: Uuid,
        token_id: i64,
        contract_address: &str,
        chain_id: i32,
        owner_address: &str,
        mint_tx_hash: &str,
        metadata_uri: &str,
    ) -> AppResult<InvoiceNft> {
        let nft = sqlx::query_as::<_, InvoiceNft>(
            r#"
            INSERT INTO invoice_nfts (invoice_id, token_id, contract_address, chain_id, owner_address, mint_tx_hash, metadata_uri, minted_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING *
            "#,
        )
        .bind(invoice_id)
        .bind(token_id)
        .bind(contract_address)
        .bind(chain_id)
        .bind(owner_address)
        .bind(mint_tx_hash)
        .bind(metadata_uri)
        .fetch_one(&self.pool)
        .await?;

        Ok(nft)
    }

    pub async fn find_nft_by_invoice(&self, invoice_id: Uuid) -> AppResult<Option<InvoiceNft>> {
        let nft =
            sqlx::query_as::<_, InvoiceNft>("SELECT * FROM invoice_nfts WHERE invoice_id = $1")
                .bind(invoice_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(nft)
    }
}
