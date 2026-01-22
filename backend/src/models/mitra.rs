use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MitraApplicationStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "rejected")]
    Rejected,
}

impl Default for MitraApplicationStatus {
    fn default() -> Self {
        MitraApplicationStatus::Pending
    }
}

impl std::fmt::Display for MitraApplicationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MitraApplicationStatus::Pending => write!(f, "pending"),
            MitraApplicationStatus::Approved => write!(f, "approved"),
            MitraApplicationStatus::Rejected => write!(f, "rejected"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MitraApplication {
    pub id: Uuid,
    pub user_id: Uuid,
    pub company_name: String,
    pub company_type: String,
    pub npwp: String,
    pub annual_revenue: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nib_document_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub akta_pendirian_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ktp_direktur_url: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_founded: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_products: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_markets: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct MitraApplyRequest {
    #[validate(length(min = 1, message = "Company name is required"))]
    pub company_name: String,
    pub company_type: Option<String>,
    #[validate(length(min = 15, max = 16, message = "NPWP must be 15-16 characters"))]
    pub npwp: String,
    pub annual_revenue: String,
    pub address: Option<String>,
    pub business_description: Option<String>,
    pub website_url: Option<String>,
    pub year_founded: Option<i32>,
    pub key_products: Option<String>,
    pub export_markets: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MitraDocumentUploadRequest {
    pub document_type: String, // "nib", "akta_pendirian", "ktp_direktur"
    pub file_url: String,
}

#[derive(Debug, Serialize)]
pub struct MitraStatusResponse {
    pub status: String,
    pub application: Option<MitraApplication>,
    pub rejection_reason: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub documents_status: MitraDocumentsStatus,
}

#[derive(Debug, Serialize)]
pub struct MitraDocumentsStatus {
    pub nib_uploaded: bool,
    pub akta_pendirian_uploaded: bool,
    pub ktp_direktur_uploaded: bool,
    pub all_documents_complete: bool,
}

#[derive(Debug, Deserialize)]
pub struct AdminMitraApproveRequest {
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminMitraRejectRequest {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct MitraApplicationListResponse {
    pub applications: Vec<MitraApplication>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

// Virtual Account for Mitra repayment
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VirtualAccount {
    pub id: Uuid,
    pub pool_id: Uuid,
    pub user_id: Uuid,
    pub va_number: String,
    pub bank_code: String,
    pub bank_name: String,
    pub amount: rust_decimal::Decimal,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateVAPaymentRequest {
    pub pool_id: Uuid,
    pub bank_code: String,
}

// Type alias for case compatibility
pub type CreateVaPaymentRequest = CreateVAPaymentRequest;

#[derive(Debug, Serialize)]
pub struct VAPaymentResponse {
    pub va: VirtualAccount,
    pub bank_name: String,
    pub amount_display: String,
    pub expires_in_hours: i32,
    pub payment_instructions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct VAPaymentMethod {
    pub bank_code: String,
    pub bank_name: String,
    pub logo_url: String,
}

pub fn get_va_payment_methods() -> Vec<VAPaymentMethod> {
    vec![
        VAPaymentMethod {
            bank_code: "bca".to_string(),
            bank_name: "Bank Central Asia (BCA)".to_string(),
            logo_url: "/assets/banks/bca.png".to_string(),
        },
        VAPaymentMethod {
            bank_code: "mandiri".to_string(),
            bank_name: "Bank Mandiri".to_string(),
            logo_url: "/assets/banks/mandiri.png".to_string(),
        },
        VAPaymentMethod {
            bank_code: "bni".to_string(),
            bank_name: "Bank Negara Indonesia (BNI)".to_string(),
            logo_url: "/assets/banks/bni.png".to_string(),
        },
        VAPaymentMethod {
            bank_code: "bri".to_string(),
            bank_name: "Bank Rakyat Indonesia (BRI)".to_string(),
            logo_url: "/assets/banks/bri.png".to_string(),
        },
    ]
}

#[derive(Debug, Serialize)]
pub struct RepaymentBreakdown {
    pub pool_id: Uuid,
    pub invoice_number: String,
    pub principal_amount: f64,
    pub total_interest: f64,
    pub platform_fee: f64,
    pub total_repayment: f64,
    pub priority_breakdown: TrancheBreakdown,
    pub catalyst_breakdown: TrancheBreakdown,
    pub due_date: DateTime<Utc>,
    pub days_remaining: i32,
}

#[derive(Debug, Serialize)]
pub struct TrancheBreakdown {
    pub tranche: String,
    pub principal: f64,
    pub interest_rate: f64,
    pub interest_amount: f64,
    pub total: f64,
    pub investor_count: i32,
}
