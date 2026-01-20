use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::KycVerification;

#[derive(Clone)]
pub struct KycRepository {
    pool: PgPool,
}

impl KycRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        verification_type: &str,
        id_type: Option<&str>,
        id_number: Option<&str>,
        id_document_url: Option<&str>,
        selfie_url: Option<&str>,
    ) -> AppResult<KycVerification> {
        let kyc = sqlx::query_as::<_, KycVerification>(
            r#"
            INSERT INTO kyc_verifications (user_id, verification_type, id_type, id_number, id_document_url, selfie_url, status)
            VALUES ($1, $2, $3, $4, $5, $6, 'pending')
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(verification_type)
        .bind(id_type)
        .bind(id_number)
        .bind(id_document_url)
        .bind(selfie_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(kyc)
    }

    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<KycVerification>> {
        let kyc = sqlx::query_as::<_, KycVerification>(
            "SELECT * FROM kyc_verifications WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(kyc)
    }

    pub async fn find_by_user(&self, user_id: Uuid) -> AppResult<Option<KycVerification>> {
        let kyc = sqlx::query_as::<_, KycVerification>(
            "SELECT * FROM kyc_verifications WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(kyc)
    }

    pub async fn find_pending(&self, page: i32, per_page: i32) -> AppResult<(Vec<KycVerification>, i64)> {
        let offset = (page - 1) * per_page;

        let kycs = sqlx::query_as::<_, KycVerification>(
            "SELECT * FROM kyc_verifications WHERE status = 'pending' ORDER BY created_at ASC LIMIT $1 OFFSET $2"
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM kyc_verifications WHERE status = 'pending'")
            .fetch_one(&self.pool)
            .await?;

        Ok((kycs, total.0))
    }

    pub async fn approve(&self, id: Uuid, verified_by: Uuid) -> AppResult<KycVerification> {
        let kyc = sqlx::query_as::<_, KycVerification>(
            r#"
            UPDATE kyc_verifications
            SET status = 'approved', verified_by = $2, verified_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(verified_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(kyc)
    }

    pub async fn reject(&self, id: Uuid, verified_by: Uuid, reason: &str) -> AppResult<KycVerification> {
        let kyc = sqlx::query_as::<_, KycVerification>(
            r#"
            UPDATE kyc_verifications
            SET status = 'rejected', verified_by = $2, verified_at = NOW(), rejection_reason = $3, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(verified_by)
        .bind(reason)
        .fetch_one(&self.pool)
        .await?;

        Ok(kyc)
    }

    pub async fn update_documents(
        &self,
        user_id: Uuid,
        id_document_url: Option<&str>,
        selfie_url: Option<&str>,
    ) -> AppResult<KycVerification> {
        let kyc = sqlx::query_as::<_, KycVerification>(
            r#"
            UPDATE kyc_verifications
            SET id_document_url = COALESCE($2, id_document_url),
                selfie_url = COALESCE($3, selfie_url),
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(id_document_url)
        .bind(selfie_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(kyc)
    }
}
