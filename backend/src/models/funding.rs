use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use super::{Invoice, User};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum PoolStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "filled")]
    Filled,
    #[serde(rename = "disbursed")]
    Disbursed,
    #[serde(rename = "closed")]
    Closed,
}

impl Default for PoolStatus {
    fn default() -> Self {
        PoolStatus::Open
    }
}

impl std::fmt::Display for PoolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolStatus::Open => write!(f, "open"),
            PoolStatus::Filled => write!(f, "filled"),
            PoolStatus::Disbursed => write!(f, "disbursed"),
            PoolStatus::Closed => write!(f, "closed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrancheType {
    #[serde(rename = "priority")]
    Priority,
    #[serde(rename = "catalyst")]
    Catalyst,
}

impl Default for TrancheType {
    fn default() -> Self {
        TrancheType::Priority
    }
}

impl std::fmt::Display for TrancheType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrancheType::Priority => write!(f, "priority"),
            TrancheType::Catalyst => write!(f, "catalyst"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FundingPool {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub target_amount: Decimal,
    pub funded_amount: Decimal,
    pub investor_count: i32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened_at: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_at: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disbursed_at: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    // Tranche fields
    pub priority_target: Decimal,
    pub priority_funded: Decimal,
    pub catalyst_target: Decimal,
    pub catalyst_funded: Decimal,
    pub priority_interest_rate: Decimal,
    pub catalyst_interest_rate: Decimal,
    pub pool_currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_pool_tx_hash: Option<String>,

    // Relations
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<Invoice>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub investments: Option<Vec<Investment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum InvestmentStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "repaid")]
    Repaid,
    #[serde(rename = "defaulted")]
    Defaulted,
}

impl Default for InvestmentStatus {
    fn default() -> Self {
        InvestmentStatus::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Investment {
    pub id: Uuid,
    pub pool_id: Uuid,
    pub investor_id: Uuid,
    pub amount: Decimal,
    pub expected_return: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_return: Option<Decimal>,
    pub status: String,
    pub tranche: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_tx_hash: Option<String>,
    pub invested_at: NaiveDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repaid_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    // Relations
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool: Option<FundingPool>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub investor: Option<User>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct InvestRequest {
    pub pool_id: Uuid,
    #[validate(range(min = 0.01, message = "Amount must be positive"))]
    pub amount: f64,
    pub tranche: String,
    pub tx_hash: String,
    pub tnc_accepted: bool,
    pub catalyst_consents: Option<CatalystConsents>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalystConsents {
    pub first_loss_consent: bool,
    pub risk_loss_consent: bool,
    pub not_bank_consent: bool,
}

impl CatalystConsents {
    pub fn all_accepted(&self) -> bool {
        self.first_loss_consent && self.risk_loss_consent && self.not_bank_consent
    }
}

#[derive(Debug, Serialize)]
pub struct FundingPoolResponse {
    pub pool: FundingPool,
    pub remaining_amount: f64,
    pub percentage_funded: f64,
    pub priority_remaining: f64,
    pub catalyst_remaining: f64,
    pub priority_percentage_funded: f64,
    pub catalyst_percentage_funded: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<Invoice>,
}

#[derive(Debug, Serialize)]
pub struct PoolListResponse {
    pub pools: Vec<FundingPoolResponse>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

#[derive(Debug, Serialize)]
pub struct InvestmentListResponse {
    pub investments: Vec<Investment>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

#[derive(Debug, Clone, Default)]
pub struct PoolFilter {
    pub status: Option<String>,
    pub grade: Option<String>,
    pub page: i32,
    pub per_page: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MarketplaceFilter {
    pub grade: Option<String>,
    pub is_insured: Option<bool>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub sort_by: Option<String>,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_per_page")]
    pub per_page: i32,
}

fn default_page() -> i32 {
    1
}
fn default_per_page() -> i32 {
    10
}

#[derive(Debug, Serialize)]
pub struct MarketplacePoolResponse {
    #[serde(flatten)]
    pub funding_pool_response: FundingPoolResponse,
    pub project_title: String,
    pub grade: String,
    pub grade_score: i32,
    pub is_insured: bool,
    pub buyer_country: String,
    pub buyer_country_flag: String,
    pub buyer_company_name: String,
    pub buyer_country_risk: String,
    pub yield_range: String,
    pub min_yield: f64,
    pub max_yield: f64,
    pub tenor_days: i32,
    pub tenor_display: String,
    pub funding_progress: f64,
    pub remaining_amount: f64,
    pub remaining_time: String,
    pub remaining_hours: i32,
    pub is_fully_funded: bool,
    pub priority_progress: f64,
    pub catalyst_progress: f64,
}

#[derive(Debug, Serialize)]
pub struct MarketplaceListResponse {
    pub pools: Vec<MarketplacePoolResponse>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

#[derive(Debug, Serialize)]
pub struct InvestorPortfolio {
    pub total_funding: f64,
    pub total_expected_gain: f64,
    pub total_realized_gain: f64,
    pub priority_allocation: f64,
    pub catalyst_allocation: f64,
    pub active_investments: i32,
    pub completed_deals: i32,
    pub available_balance: f64,
}

#[derive(Debug, Serialize)]
pub struct InvestorActiveInvestment {
    pub investment_id: Uuid,
    pub project_name: String,
    pub invoice_number: String,
    pub buyer_name: String,
    pub buyer_country: String,
    pub buyer_flag: String,
    pub tranche: String,
    pub tranche_display: String,
    pub principal: f64,
    pub interest_rate: f64,
    pub estimated_return: f64,
    pub total_expected: f64,
    pub due_date: NaiveDateTime,
    pub days_remaining: i32,
    pub status: String,
    pub status_display: String,
    pub status_color: String,
    pub invested_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct InvestorActiveInvestmentList {
    pub investments: Vec<InvestorActiveInvestment>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
    pub summary: InvestorPortfolio,
}

#[derive(Debug, Serialize)]
pub struct MitraDashboard {
    pub total_active_financing: f64,
    pub total_owed_to_investors: f64,
    pub average_remaining_tenor: i32,
    pub active_invoices: Vec<InvoiceDashboard>,
    pub timeline_status: TimelineStatus,
}

#[derive(Debug, Serialize)]
pub struct TimelineStatus {
    pub fundraising_complete: bool,
    pub disbursement_complete: bool,
    pub repayment_complete: bool,
    pub current_step: String,
}

#[derive(Debug, Serialize)]
pub struct InvoiceDashboard {
    pub invoice_id: Uuid,
    pub invoice_number: String,
    pub buyer_name: String,
    pub buyer_country: String,
    pub due_date: NaiveDateTime,
    pub amount: f64,
    pub status: String,
    pub status_color: String,
    pub days_remaining: i32,
    pub funded_amount: f64,
    pub total_owed: f64,
}

#[derive(Debug, Serialize)]
pub struct ActiveInvestmentListResponse {
    pub investments: Vec<InvestorActiveInvestment>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

#[derive(Debug, Serialize)]
pub struct MitraInvoiceListResponse {
    pub invoices: Vec<InvoiceDashboard>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmInvestmentRequest {
    pub investment_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CalculateInvestmentRequest {
    pub pool_id: Uuid,
    pub amount: f64,
    pub tranche: String,
}

#[derive(Debug, Serialize)]
pub struct CalculateInvestmentResponse {
    pub principal: f64,
    pub interest_rate: f64,
    pub expected_return: f64,
    pub total_return: f64,
    pub tranche: String,
    pub tenor_days: i32,
}

// Additional request types

#[derive(Debug, Deserialize)]
pub struct CreatePoolRequest {
    pub funding_deadline_hours: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct InvestWithWalletRequest {
    #[validate(range(min = 0.01, message = "Amount must be positive"))]
    pub amount: f64,
    pub tranche: String,
    pub tx_hash: String,
    pub tnc_accepted: bool,
    pub catalyst_consents: Option<CatalystConsents>,
}

#[derive(Debug, Deserialize)]
pub struct ExporterDisbursementRequest {
    pub pool_id: Uuid,
    pub bank_account_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ProcessRepaymentRequest {
    pub amount: f64,
    pub tx_hash: Option<String>,
    pub notes: Option<String>,
}

// Note: CreateVaPaymentRequest is defined in mitra.rs - use that instead

#[derive(Debug, Serialize)]
pub struct RepaymentBreakdown {
    pub pool_id: Uuid,
    pub total_principal: f64,
    pub total_interest: f64,
    pub total_repayment: f64,
    pub priority_repayment: f64,
    pub catalyst_repayment: f64,
    pub platform_fee: f64,
    pub due_date: NaiveDateTime,
    pub investors: Vec<InvestorRepayment>,
}

#[derive(Debug, Serialize)]
pub struct InvestorRepayment {
    pub investor_id: Uuid,
    pub tranche: String,
    pub principal: f64,
    pub interest: f64,
    pub total: f64,
}

#[derive(Debug, Serialize)]
pub struct PlatformStats {
    pub total_funded: f64,
    pub total_repaid: f64,
    pub active_pools: i32,
    pub total_investors: i32,
    pub total_exporters: i32,
    pub average_yield: f64,
    pub default_rate: f64,
}
