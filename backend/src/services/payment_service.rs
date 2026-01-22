use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::UserBalanceResponse;
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

    pub async fn get_balance(&self, user_id: Uuid) -> AppResult<UserBalanceResponse> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(UserBalanceResponse {
            balance_idrx: user.balance_idrx.to_f64().unwrap_or(0.0),
            currency: "IDRX".to_string(),
        })
    }

    pub async fn deposit(
        &self,
        user_id: Uuid,
        amount: f64,
        tx_hash: &str,
    ) -> AppResult<UserBalanceResponse> {
        // Verify transaction on blockchain
        let is_valid = self.blockchain_service.verify_transaction(tx_hash).await?;
        if !is_valid {
            return Err(AppError::BlockchainError(
                "Transaction not confirmed".to_string(),
            ));
        }

        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let deposit_amount = Decimal::from_f64(amount)
            .ok_or_else(|| AppError::ValidationError("Invalid amount".to_string()))?;

        let new_balance = user.balance_idrx + deposit_amount;

        // Update balance
        self.user_repo.update_balance(user_id, new_balance).await?;

        // Record transaction
        self.tx_repo
            .create_balance_transaction(
                user_id,
                "deposit",
                deposit_amount,
                user.balance_idrx,
                new_balance,
                None,
                None,
                Some("Deposit from wallet"),
            )
            .await?;

        // Record blockchain transaction
        self.tx_repo
            .create(
                None,
                Some(user_id),
                "investment", // Using investment type for deposit tracking
                deposit_amount,
                "IDRX",
                Some(tx_hash),
                user.wallet_address.as_deref(),
                Some(self.blockchain_service.get_platform_wallet()),
                Some("Deposit"),
            )
            .await?;

        Ok(UserBalanceResponse {
            balance_idrx: new_balance.to_f64().unwrap_or(0.0),
            currency: "IDRX".to_string(),
        })
    }

    pub async fn withdraw(
        &self,
        user_id: Uuid,
        amount: f64,
        _to_address: &str,
    ) -> AppResult<UserBalanceResponse> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let withdraw_amount = Decimal::from_f64(amount)
            .ok_or_else(|| AppError::ValidationError("Invalid amount".to_string()))?;

        if user.balance_idrx < withdraw_amount {
            return Err(AppError::InsufficientBalance);
        }

        let new_balance = user.balance_idrx - withdraw_amount;

        // Update balance
        self.user_repo.update_balance(user_id, new_balance).await?;

        // Record transaction
        self.tx_repo
            .create_balance_transaction(
                user_id,
                "withdrawal",
                withdraw_amount,
                user.balance_idrx,
                new_balance,
                None,
                None,
                Some("Withdrawal to wallet"),
            )
            .await?;

        Ok(UserBalanceResponse {
            balance_idrx: new_balance.to_f64().unwrap_or(0.0),
            currency: "IDRX".to_string(),
        })
    }

    pub async fn admin_grant_balance(
        &self,
        user_id: Uuid,
        amount: f64,
        reason: &str,
    ) -> AppResult<UserBalanceResponse> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let grant_amount = Decimal::from_f64(amount)
            .ok_or_else(|| AppError::ValidationError("Invalid amount".to_string()))?;

        let new_balance = user.balance_idrx + grant_amount;

        // Update balance
        self.user_repo.update_balance(user_id, new_balance).await?;

        // Record transaction
        self.tx_repo
            .create_balance_transaction(
                user_id,
                "deposit",
                grant_amount,
                user.balance_idrx,
                new_balance,
                None,
                Some("admin_grant"),
                Some(reason),
            )
            .await?;

        Ok(UserBalanceResponse {
            balance_idrx: new_balance.to_f64().unwrap_or(0.0),
            currency: "IDRX".to_string(),
        })
    }

    pub async fn get_platform_revenue(&self) -> AppResult<f64> {
        let revenue = self.tx_repo.get_platform_revenue().await?;
        Ok(revenue.to_f64().unwrap_or(0.0))
    }
}
