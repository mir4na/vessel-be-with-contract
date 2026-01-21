use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    #[serde(rename = "investment")]
    Investment,
    #[serde(rename = "advance_payment")]
    AdvancePayment,
    #[serde(rename = "buyer_repayment")]
    BuyerRepayment,
    #[serde(rename = "investor_return")]
    InvestorReturn,
    #[serde(rename = "platform_fee")]
    PlatformFee,
    #[serde(rename = "refund")]
    Refund,
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionType::Investment => write!(f, "investment"),
            TransactionType::AdvancePayment => write!(f, "advance_payment"),
            TransactionType::BuyerRepayment => write!(f, "buyer_repayment"),
            TransactionType::InvestorReturn => write!(f, "investor_return"),
            TransactionType::PlatformFee => write!(f, "platform_fee"),
            TransactionType::Refund => write!(f, "refund"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "confirmed")]
    Confirmed,
    #[serde(rename = "failed")]
    Failed,
}

impl Default for TransactionStatus {
    fn default() -> Self {
        TransactionStatus::Pending
    }
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "pending"),
            TransactionStatus::Confirmed => write!(f, "confirmed"),
            TransactionStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub amount: Decimal,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// URL to view transaction on block explorer (e.g., basescan.org)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explorer_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BalanceTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_before: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_after: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    pub invoice_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub tx_type: String,
    pub amount: f64,
    pub currency: Option<String>,
    pub tx_hash: Option<String>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub notes: Option<String>,
}
