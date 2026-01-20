use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "paid")]
    Paid,
    #[serde(rename = "overdue")]
    Overdue,
    #[serde(rename = "canceled")]
    Canceled,
}

impl Default for PaymentStatus {
    fn default() -> Self {
        PaymentStatus::Pending
    }
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentStatus::Pending => write!(f, "pending"),
            PaymentStatus::Paid => write!(f, "paid"),
            PaymentStatus::Overdue => write!(f, "overdue"),
            PaymentStatus::Canceled => write!(f, "canceled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImporterPayment {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub pool_id: Uuid,
    pub buyer_email: String,
    pub buyer_name: String,
    pub amount_due: Decimal,
    pub amount_paid: Decimal,
    pub currency: String,
    pub payment_status: String,
    pub due_date: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ImporterPaymentInfoResponse {
    pub payment_id: Uuid,
    pub invoice_number: String,
    pub buyer_name: String,
    pub buyer_email: String,
    pub seller_name: String,
    pub amount_due: f64,
    pub amount_paid: f64,
    pub remaining_amount: f64,
    pub currency: String,
    pub due_date: DateTime<Utc>,
    pub status: String,
    pub is_overdue: bool,
    pub days_until_due: i32,
}

#[derive(Debug, Deserialize)]
pub struct ImporterPayRequest {
    pub amount: f64,
    pub tx_hash: String,
}

#[derive(Debug, Serialize)]
pub struct ImporterPayResponse {
    pub success: bool,
    pub message: String,
    pub payment_id: Uuid,
    pub amount_paid: f64,
    pub remaining_amount: f64,
    pub status: String,
    pub tx_hash: String,
}
