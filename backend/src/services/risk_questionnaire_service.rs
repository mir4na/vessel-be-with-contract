use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{
    RiskQuestionnaire, RiskQuestion, SubmitRiskQuestionnaireRequest,
    RiskQuestionnaireStatusResponse, RiskQuestionnaireAnswers, get_risk_questions,
};
use crate::repository::RiskQuestionnaireRepository;

pub struct RiskQuestionnaireService {
    rq_repo: Arc<RiskQuestionnaireRepository>,
}

impl RiskQuestionnaireService {
    pub fn new(rq_repo: Arc<RiskQuestionnaireRepository>) -> Self {
        Self { rq_repo }
    }

    pub fn get_questions(&self) -> Vec<RiskQuestion> {
        get_risk_questions()
    }

    pub async fn submit(&self, user_id: Uuid, req: SubmitRiskQuestionnaireRequest) -> AppResult<RiskQuestionnaire> {
        // Validate answers
        if !(1..=3).contains(&req.q1_answer) {
            return Err(AppError::ValidationError("Invalid answer for Q1".to_string()));
        }
        if !(1..=2).contains(&req.q2_answer) {
            return Err(AppError::ValidationError("Invalid answer for Q2".to_string()));
        }
        if !(1..=2).contains(&req.q3_answer) {
            return Err(AppError::ValidationError("Invalid answer for Q3".to_string()));
        }

        // Check if catalyst should be unlocked
        let catalyst_unlocked = self.check_catalyst_unlocked(req.q1_answer, req.q2_answer, req.q3_answer);

        // Check if already exists
        if let Some(_existing) = self.rq_repo.find_by_user(user_id).await? {
            // Update existing
            self.rq_repo.update(
                user_id,
                req.q1_answer,
                req.q2_answer,
                req.q3_answer,
                catalyst_unlocked,
            ).await
        } else {
            // Create new
            self.rq_repo.create(
                user_id,
                req.q1_answer,
                req.q2_answer,
                req.q3_answer,
                catalyst_unlocked,
            ).await
        }
    }

    pub async fn get_status(&self, user_id: Uuid) -> AppResult<RiskQuestionnaireStatusResponse> {
        let rq = self.rq_repo.find_by_user(user_id).await?;

        match rq {
            Some(r) => Ok(RiskQuestionnaireStatusResponse {
                completed: true,
                catalyst_unlocked: r.catalyst_unlocked,
                completed_at: Some(r.completed_at),
                answers: Some(RiskQuestionnaireAnswers {
                    q1_answer: r.q1_answer,
                    q2_answer: r.q2_answer,
                    q3_answer: r.q3_answer,
                }),
            }),
            None => Ok(RiskQuestionnaireStatusResponse {
                completed: false,
                catalyst_unlocked: false,
                completed_at: None,
                answers: None,
            }),
        }
    }

    fn check_catalyst_unlocked(&self, q1: i32, q2: i32, q3: i32) -> bool {
        // Q1: Must be >= 2 (1+ years experience)
        // Q2: Must be 1 (understands risk)
        // Q3: Must be 1 (willing to be first loss)
        q1 >= 2 && q2 == 1 && q3 == 1
    }
}
