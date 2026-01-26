
pub mod auth;
pub mod blockchain;
pub mod currency;
pub mod funding;
pub mod importer;
pub mod invoice;
pub mod mitra;
pub mod payment;
pub mod risk_questionnaire;
pub mod user;

use actix_web::HttpResponse;
use sqlx::PgPool;
use std::sync::Arc;

use crate::config::Config;
use crate::repository::*;
use crate::services::*;
use crate::utils::{ApiResponse, JwtManager};

/// Application state shared across all handlers
#[allow(dead_code)] // Fields accessed by various handlers via web::Data
pub struct AppState {
    pub config: Arc<Config>,
    pub db_pool: PgPool,
    pub redis_pool: Option<deadpool_redis::Pool>,
    pub jwt_manager: Arc<JwtManager>,

    // Repositories
    pub user_repo: Arc<UserRepository>,
    pub invoice_repo: Arc<InvoiceRepository>,
    pub funding_repo: Arc<FundingRepository>,
    pub tx_repo: Arc<TransactionRepository>,
    pub otp_repo: Arc<OtpRepository>,
    pub mitra_repo: Arc<MitraRepository>,
    pub importer_payment_repo: Arc<ImporterPaymentRepository>,
    pub rq_repo: Arc<RiskQuestionnaireRepository>,

    // Services
    pub auth_service: Arc<AuthService>,
    pub otp_service: Arc<OtpService>,
    pub mitra_service: Arc<MitraService>,
    pub invoice_service: Arc<InvoiceService>,
    pub funding_service: Arc<FundingService>,
    pub payment_service: Arc<PaymentService>,
    pub rq_service: Arc<RiskQuestionnaireService>,
    pub currency_service: Arc<CurrencyService>,
    pub blockchain_service: Arc<BlockchainService>,
    pub pinata_service: Arc<PinataService>,
    pub email_service: Arc<EmailService>,
    pub escrow_service: Arc<EscrowService>,
}

/// Health check endpoint
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "status": "healthy",
            "service": "VESSEL Backend (Rust/Actix)",
            "version": env!("CARGO_PKG_VERSION"),
            "network": "Base"
        }),
        "Service is healthy",
    ))
}
