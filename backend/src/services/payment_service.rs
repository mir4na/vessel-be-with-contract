use rust_decimal::prelude::ToPrimitive;
use std::sync::Arc;

use crate::error::AppResult;
use crate::repository::{
    FundingRepository, InvoiceRepository, TransactionRepository, UserRepository,
};

use super::BlockchainService;

pub struct PaymentService {
    user_repo: Arc<UserRepository>,
    tx_repo: Arc<TransactionRepository>,
    funding_repo: Arc<FundingRepository>,
    invoice_repo: Arc<InvoiceRepository>,
    blockchain_service: Arc<BlockchainService>,
}

impl PaymentService {
    pub fn new(
        user_repo: Arc<UserRepository>,
        tx_repo: Arc<TransactionRepository>,
        funding_repo: Arc<FundingRepository>,
        invoice_repo: Arc<InvoiceRepository>,
        blockchain_service: Arc<BlockchainService>,
    ) -> Self {
        Self {
            user_repo,
            tx_repo,
            funding_repo,
            invoice_repo,
            blockchain_service,
        }
    }

    pub async fn get_platform_revenue(&self) -> AppResult<f64> {
        let revenue = self.tx_repo.get_platform_revenue().await?;
        Ok(revenue.to_f64().unwrap_or(0.0))
    }
}
