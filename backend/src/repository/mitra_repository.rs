use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::MitraApplication;

#[derive(Clone)]
pub struct MitraRepository {
    pool: PgPool,
}

impl MitraRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        company_name: &str,
        company_type: &str,
        npwp: &str,
        annual_revenue: &str,
        address: Option<&str>,
        business_description: Option<&str>,
        website_url: Option<&str>,
        year_founded: Option<i32>,
        key_products: Option<&str>,
        export_markets: Option<&str>,
    ) -> AppResult<MitraApplication> {
        let app = sqlx::query_as::<_, MitraApplication>(
            r#"
            INSERT INTO mitra_applications (
                user_id, company_name, company_type, npwp, annual_revenue,
                address, business_description, website_url, year_founded,
                key_products, export_markets, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'pending')
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(company_name)
        .bind(company_type)
        .bind(npwp)
        .bind(annual_revenue)
        .bind(address)
        .bind(business_description)
        .bind(website_url)
        .bind(year_founded)
        .bind(key_products)
        .bind(export_markets)
        .fetch_one(&self.pool)
        .await?;

        Ok(app)
    }

    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<MitraApplication>> {
        let app =
            sqlx::query_as::<_, MitraApplication>("SELECT * FROM mitra_applications WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(app)
    }

    pub async fn find_by_user(&self, user_id: Uuid) -> AppResult<Option<MitraApplication>> {
        let app = sqlx::query_as::<_, MitraApplication>(
            "SELECT * FROM mitra_applications WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(app)
    }

    pub async fn find_pending(
        &self,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<MitraApplication>, i64)> {
        let offset = (page - 1) * per_page;

        let apps = sqlx::query_as::<_, MitraApplication>(
            "SELECT * FROM mitra_applications WHERE status = 'pending' ORDER BY created_at ASC LIMIT $1 OFFSET $2"
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM mitra_applications WHERE status = 'pending'")
                .fetch_one(&self.pool)
                .await?;

        Ok((apps, total.0))
    }

    pub async fn find_all(
        &self,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<MitraApplication>, i64)> {
        let offset = (page - 1) * per_page;

        let apps = sqlx::query_as::<_, MitraApplication>(
            "SELECT * FROM mitra_applications ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mitra_applications")
            .fetch_one(&self.pool)
            .await?;

        Ok((apps, total.0))
    }
    pub async fn update_document(
        &self,
        id: Uuid,
        document_type: &str,
        file_url: &str,
    ) -> AppResult<MitraApplication> {
        let column = match document_type {
            "nib" => "nib_document_url",
            "akta_pendirian" => "akta_pendirian_url",
            "ktp_direktur" => "ktp_direktur_url",
            _ => {
                return Err(crate::error::AppError::BadRequest(
                    "Invalid document type".to_string(),
                ))
            }
        };

        let query = format!(
            "UPDATE mitra_applications SET {} = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
            column
        );

        let app = sqlx::query_as::<_, MitraApplication>(&query)
            .bind(id)
            .bind(file_url)
            .fetch_one(&self.pool)
            .await?;

        Ok(app)
    }

    pub async fn approve(&self, id: Uuid, reviewed_by: Uuid) -> AppResult<MitraApplication> {
        let app = sqlx::query_as::<_, MitraApplication>(
            r#"
            UPDATE mitra_applications
            SET status = 'approved', reviewed_by = $2, reviewed_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(app)
    }

    pub async fn reject(
        &self,
        id: Uuid,
        reviewed_by: Uuid,
        reason: &str,
    ) -> AppResult<MitraApplication> {
        let app = sqlx::query_as::<_, MitraApplication>(
            r#"
            UPDATE mitra_applications
            SET status = 'rejected', reviewed_by = $2, reviewed_at = NOW(), rejection_reason = $3, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by)
        .bind(reason)
        .fetch_one(&self.pool)
        .await?;

        Ok(app)
    }
}
