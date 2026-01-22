use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use super::{User, UserProfile};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum InvoiceStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "pending_review")]
    PendingReview,
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "tokenized")]
    Tokenized,
    #[serde(rename = "funding")]
    Funding,
    #[serde(rename = "funded")]
    Funded,
    #[serde(rename = "matured")]
    Matured,
    #[serde(rename = "repaid")]
    Repaid,
    #[serde(rename = "defaulted")]
    Defaulted,
}

impl Default for InvoiceStatus {
    fn default() -> Self {
        InvoiceStatus::Draft
    }
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvoiceStatus::Draft => write!(f, "draft"),
            InvoiceStatus::PendingReview => write!(f, "pending_review"),
            InvoiceStatus::Approved => write!(f, "approved"),
            InvoiceStatus::Rejected => write!(f, "rejected"),
            InvoiceStatus::Tokenized => write!(f, "tokenized"),
            InvoiceStatus::Funding => write!(f, "funding"),
            InvoiceStatus::Funded => write!(f, "funded"),
            InvoiceStatus::Matured => write!(f, "matured"),
            InvoiceStatus::Repaid => write!(f, "repaid"),
            InvoiceStatus::Defaulted => write!(f, "defaulted"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentType {
    #[serde(rename = "invoice_pdf")]
    InvoicePdf,
    #[serde(rename = "bill_of_lading")]
    BillOfLading,
    #[serde(rename = "packing_list")]
    PackingList,
    #[serde(rename = "certificate_of_origin")]
    CertificateOfOrigin,
    #[serde(rename = "insurance")]
    Insurance,
    #[serde(rename = "customs")]
    Customs,
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "purchase_order")]
    PurchaseOrder,
    #[serde(rename = "commercial_invoice")]
    CommercialInvoice,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invoice {
    pub id: Uuid,
    pub exporter_id: Uuid,
    pub buyer_name: String,
    pub buyer_country: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_email: Option<String>,
    pub invoice_number: String,
    pub currency: String,
    pub amount: Decimal,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interest_rate: Option<Decimal>,
    pub advance_percentage: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advance_amount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_hash: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    // Grading fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade_score: Option<i32>,
    pub is_repeat_buyer: bool,
    pub funding_limit_percentage: Decimal,
    pub is_insured: bool,
    pub document_complete_score: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_country_risk: Option<String>,

    // Tranche fields
    pub priority_ratio: Decimal,
    pub catalyst_ratio: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_interest_rate: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalyst_interest_rate: Option<Decimal>,

    // Currency fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_amount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idrx_amount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_rate: Option<Decimal>,
    pub buffer_rate: Decimal,

    // Additional fields
    pub funding_duration_days: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_link: Option<String>,

    // Relations (not from DB, populated separately)
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exporter: Option<User>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<InvoiceDocument>>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nft: Option<InvoiceNft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceDocument {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub document_type: String,
    pub file_name: String,
    pub file_url: String,
    pub file_hash: String,
    pub file_size: i32,
    pub uploaded_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceNft {
    pub id: Uuid,
    pub invoice_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_address: Option<String>,
    pub chain_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint_tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minted_at: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burned_at: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burn_tx_hash: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateInvoiceFundingRequest {
    // Buyer data
    pub buyer_company_name: String,
    pub buyer_country: String,
    #[validate(email(message = "Invalid buyer email"))]
    pub buyer_email: String,

    // Invoice data
    pub invoice_number: String,
    pub original_currency: String,
    #[validate(range(min = 0.01, message = "Amount must be positive"))]
    pub original_amount: f64,
    pub locked_exchange_rate: f64,
    #[validate(range(min = 0.01, message = "IDR amount must be positive"))]
    pub idr_amount: f64,
    pub due_date: String,
    #[serde(default)]
    pub funding_duration_days: Option<i32>,

    // Tranche configuration
    #[serde(default)]
    pub priority_ratio: Option<f64>,
    #[serde(default)]
    pub catalyst_ratio: Option<f64>,
    #[validate(range(
        min = 0.01,
        max = 100.0,
        message = "Priority interest rate must be 0.01-100"
    ))]
    pub priority_interest_rate: f64,
    #[validate(range(
        min = 0.01,
        max = 100.0,
        message = "Catalyst interest rate must be 0.01-100"
    ))]
    pub catalyst_interest_rate: f64,

    // Repeat buyer
    #[serde(default)]
    pub is_repeat_buyer: bool,
    pub repeat_buyer_proof: Option<String>,

    // Confirmation
    pub data_confirmation: bool,

    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    pub buyer_id: Uuid,
    pub invoice_number: String,
    pub currency: Option<String>,
    pub amount: f64,
    pub issue_date: String,
    pub due_date: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInvoiceRequest {
    pub invoice_number: Option<String>,
    pub currency: Option<String>,
    pub amount: Option<f64>,
    pub issue_date: Option<String>,
    pub due_date: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitInvoiceRequest {
    pub invoice_id: Uuid,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AdminApproveInvoiceRequest {
    pub grade: String,
    pub priority_interest_rate: Option<f64>,
    pub catalyst_interest_rate: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminGradeSuggestionResponse {
    pub invoice_id: String,
    pub suggested_grade: String,
    pub grade_score: i32,
    pub country_risk: String,
    pub country_score: i32,
    pub history_score: i32,
    pub document_score: i32,
    pub is_repeat_buyer: bool,
    pub documents_complete: bool,
    pub funding_limit: f64,
}

#[derive(Debug, Deserialize)]
pub struct ApproveInvoiceRequest {
    pub invoice_id: Uuid,
    pub interest_rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct RejectInvoiceRequest {
    pub invoice_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentValidationStatus {
    pub document_id: String,
    pub document_type: String,
    pub file_name: String,
    pub file_url: String,
    pub is_valid: bool,
    pub needs_revision: bool,
    pub revision_note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceReviewData {
    pub invoice: Invoice,
    pub exporter: Option<UserProfile>,
    pub documents: Vec<DocumentValidationStatus>,
    pub grade_suggestion: AdminGradeSuggestionResponse,
}

#[derive(Debug, Deserialize)]
pub struct ValidateDocumentRequest {
    pub document_id: String,
    pub is_valid: bool,
    pub needs_revision: bool,
    pub revision_note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<Invoice>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

#[derive(Debug, Clone, Default)]
pub struct InvoiceFilter {
    pub status: Option<String>,
    pub statuses: Option<Vec<String>>,
    pub buyer_id: Option<Uuid>,
    pub exporter_id: Option<Uuid>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub page: i32,
    pub per_page: i32,
}

#[derive(Debug, Deserialize)]
pub struct RepeatBuyerCheckRequest {
    pub buyer_company_name: String,
}

#[derive(Debug, Serialize)]
pub struct RepeatBuyerCheckResponse {
    pub is_repeat_buyer: bool,
    pub message: String,
    pub previous_transactions: Option<i32>,
    pub funding_limit: f64,
}

#[derive(Debug, Serialize)]
pub struct EstimatedDisbursement {
    pub gross_amount: f64,
    pub platform_fee: f64,
    pub net_disbursement: f64,
    pub currency: String,
}

// Type aliases for backward compatibility
pub type CreateFundingRequestInput = CreateInvoiceFundingRequest;
pub type CheckRepeatBuyerRequest = RepeatBuyerCheckRequest;

// Admin review request
#[derive(Debug, Deserialize)]
pub struct AdminReviewInvoiceRequest {
    pub action: String, // "approve" or "reject"
    pub grade: Option<String>,
    pub priority_interest_rate: Option<f64>,
    pub catalyst_interest_rate: Option<f64>,
    pub rejection_reason: Option<String>,
    pub notes: Option<String>,
}
