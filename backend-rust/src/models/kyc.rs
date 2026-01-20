use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KycVerificationStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "rejected")]
    Rejected,
}

impl Default for KycVerificationStatus {
    fn default() -> Self {
        KycVerificationStatus::Pending
    }
}

impl std::fmt::Display for KycVerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KycVerificationStatus::Pending => write!(f, "pending"),
            KycVerificationStatus::Approved => write!(f, "approved"),
            KycVerificationStatus::Rejected => write!(f, "rejected"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KycVerificationType {
    #[serde(rename = "kyc")]
    Kyc,
    #[serde(rename = "kyb")]
    Kyb,
}

impl std::fmt::Display for KycVerificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KycVerificationType::Kyc => write!(f, "kyc"),
            KycVerificationType::Kyb => write!(f, "kyb"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KycVerification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub verification_type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_document_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selfie_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SubmitKycRequest {
    pub verification_type: Option<String>, // "kyc" or "kyb"
    pub id_type: Option<String>,
    pub id_number: Option<String>,
    pub id_document_url: Option<String>,
    pub selfie_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KycStatusResponse {
    pub status: String,
    pub verification_type: Option<String>,
    pub rejection_reason: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub submitted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct AdminKycApproveRequest {
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminKycRejectRequest {
    pub reason: String,
}
