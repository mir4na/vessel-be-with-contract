use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RiskQuestionnaire {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q1_answer: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q2_answer: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q3_answer: Option<i32>,
    pub catalyst_unlocked: bool,
    pub completed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RiskQuestion {
    pub id: i32,
    pub question: String,
    pub options: Vec<RiskOption>,
    pub required_for_catalyst: bool,
}

#[derive(Debug, Serialize)]
pub struct RiskOption {
    pub value: i32,
    pub label: String,
    pub unlocks_catalyst: bool,
}

pub fn get_risk_questions() -> Vec<RiskQuestion> {
    vec![
        RiskQuestion {
            id: 1,
            question: "Seberapa lama pengalaman Anda dalam berinvestasi?".to_string(),
            options: vec![
                RiskOption {
                    value: 1,
                    label: "Kurang dari 1 tahun".to_string(),
                    unlocks_catalyst: false,
                },
                RiskOption {
                    value: 2,
                    label: "1-3 tahun".to_string(),
                    unlocks_catalyst: true,
                },
                RiskOption {
                    value: 3,
                    label: "Lebih dari 3 tahun".to_string(),
                    unlocks_catalyst: true,
                },
            ],
            required_for_catalyst: true,
        },
        RiskQuestion {
            id: 2,
            question: "Apakah Anda memahami bahwa tranche Catalyst memiliki risiko lebih tinggi dan dapat kehilangan modal?".to_string(),
            options: vec![
                RiskOption {
                    value: 1,
                    label: "Ya, saya memahami risikonya".to_string(),
                    unlocks_catalyst: true,
                },
                RiskOption {
                    value: 2,
                    label: "Tidak, saya tidak mau mengambil risiko tersebut".to_string(),
                    unlocks_catalyst: false,
                },
            ],
            required_for_catalyst: true,
        },
        RiskQuestion {
            id: 3,
            question: "Apakah Anda bersedia dana Anda menjadi jaminan pertama jika terjadi gagal bayar?".to_string(),
            options: vec![
                RiskOption {
                    value: 1,
                    label: "Ya, saya bersedia".to_string(),
                    unlocks_catalyst: true,
                },
                RiskOption {
                    value: 2,
                    label: "Tidak, saya tidak bersedia".to_string(),
                    unlocks_catalyst: false,
                },
            ],
            required_for_catalyst: true,
        },
    ]
}

#[derive(Debug, Deserialize)]
pub struct SubmitRiskQuestionnaireRequest {
    pub q1_answer: i32,
    pub q2_answer: i32,
    pub q3_answer: i32,
}

#[derive(Debug, Serialize)]
pub struct RiskQuestionnaireStatusResponse {
    pub completed: bool,
    pub catalyst_unlocked: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub answers: Option<RiskQuestionnaireAnswers>,
}

#[derive(Debug, Serialize)]
pub struct RiskQuestionnaireAnswers {
    pub q1_answer: Option<i32>,
    pub q2_answer: Option<i32>,
    pub q3_answer: Option<i32>,
}

impl RiskQuestionnaire {
    /// Check if the answers unlock catalyst tranche
    pub fn check_catalyst_unlocked(&self) -> bool {
        // Q1: Must be >= 2 (1+ years experience)
        // Q2: Must be 1 (understands risk)
        // Q3: Must be 1 (willing to be first loss)
        let q1_ok = self.q1_answer.map(|a| a >= 2).unwrap_or(false);
        let q2_ok = self.q2_answer.map(|a| a == 1).unwrap_or(false);
        let q3_ok = self.q3_answer.map(|a| a == 1).unwrap_or(false);

        q1_ok && q2_ok && q3_ok
    }
}
