use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::RiskQuestionnaire;

#[derive(Clone)]
pub struct RiskQuestionnaireRepository {
    pool: PgPool,
}

impl RiskQuestionnaireRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        q1_answer: i32,
        q2_answer: i32,
        q3_answer: i32,
        catalyst_unlocked: bool,
        selected_tier: String,
    ) -> AppResult<RiskQuestionnaire> {
        let rq = sqlx::query_as::<_, RiskQuestionnaire>(
            r#"
            INSERT INTO risk_questionnaires (user_id, q1_answer, q2_answer, q3_answer, catalyst_unlocked, selected_tier)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(q1_answer)
        .bind(q2_answer)
        .bind(q3_answer)
        .bind(catalyst_unlocked)
        .bind(selected_tier)
        .fetch_one(&self.pool)
        .await?;

        Ok(rq)
    }

    pub async fn find_by_user(&self, user_id: Uuid) -> AppResult<Option<RiskQuestionnaire>> {
        let rq = sqlx::query_as::<_, RiskQuestionnaire>(
            "SELECT * FROM risk_questionnaires WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rq)
    }

    pub async fn update(
        &self,
        user_id: Uuid,
        q1_answer: i32,
        q2_answer: i32,
        q3_answer: i32,
        catalyst_unlocked: bool,
        selected_tier: String,
    ) -> AppResult<RiskQuestionnaire> {
        let rq = sqlx::query_as::<_, RiskQuestionnaire>(
            r#"
            UPDATE risk_questionnaires
            SET q1_answer = $2, q2_answer = $3, q3_answer = $4, catalyst_unlocked = $5, selected_tier = $6, completed_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(q1_answer)
        .bind(q2_answer)
        .bind(q3_answer)
        .bind(catalyst_unlocked)
        .bind(selected_tier)
        .fetch_one(&self.pool)
        .await?;

        Ok(rq)
    }

    pub async fn is_catalyst_unlocked(&self, user_id: Uuid) -> AppResult<bool> {
        let rq = self.find_by_user(user_id).await?;
        Ok(rq.map(|r| r.catalyst_unlocked).unwrap_or(false))
    }
}
