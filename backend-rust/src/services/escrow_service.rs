use std::sync::Arc;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::error::AppResult;

use super::BlockchainService;

/// On-chain escrow record
#[derive(Debug, Clone, serde::Serialize)]
pub struct EscrowRecord {
    pub id: String,
    pub pool_id: Uuid,
    pub investor_id: Uuid,
    pub amount: Decimal,
    pub tx_hash: String,
    pub status: EscrowStatus,
    pub explorer_url: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum EscrowStatus {
    Pending,
    Verified,
    Released,
    Refunded,
}

/// Escrow service for managing funds on-chain via IDRX token
/// All funds are held in the platform wallet and transfers are verified on-chain
pub struct EscrowService {
    blockchain_service: Option<Arc<BlockchainService>>,
}

impl EscrowService {
    pub fn new() -> Self {
        Self {
            blockchain_service: None,
        }
    }

    /// Set the blockchain service (called after initialization)
    pub fn set_blockchain_service(&mut self, service: Arc<BlockchainService>) {
        self.blockchain_service = Some(service);
    }

    fn get_blockchain_service(&self) -> AppResult<&Arc<BlockchainService>> {
        self.blockchain_service.as_ref().ok_or_else(|| {
            crate::error::AppError::BlockchainError("Blockchain service not initialized".to_string())
        })
    }

    /// Verify and record investor's IDRX transfer to escrow (platform wallet)
    /// This verifies the on-chain transaction and records it
    pub async fn verify_investment_deposit(
        &self,
        pool_id: Uuid,
        investor_id: Uuid,
        amount: Decimal,
        tx_hash: &str,
    ) -> AppResult<EscrowRecord> {
        let blockchain = self.get_blockchain_service()?;

        // Verify the on-chain transfer
        let verified = blockchain.verify_investment_transfer(tx_hash, amount).await?;

        tracing::info!(
            "Verified investment deposit: {} IDRX from {} to platform wallet for pool {}",
            verified.amount, verified.from, pool_id
        );

        Ok(EscrowRecord {
            id: format!("escrow_{}_{}", pool_id, investor_id),
            pool_id,
            investor_id,
            amount: verified.amount,
            tx_hash: verified.tx_hash,
            status: EscrowStatus::Verified,
            explorer_url: verified.explorer_url,
        })
    }

    /// Hold funds in escrow - in this implementation, funds are already in platform wallet
    /// after investor transfers IDRX. This method just records the escrow.
    pub async fn hold_funds(
        &self,
        pool_id: Uuid,
        amount: Decimal,
        investor_id: Uuid,
    ) -> AppResult<String> {
        tracing::info!(
            "Recording escrow: {} IDRX for pool {} from investor {}",
            amount, pool_id, investor_id
        );

        // In the on-chain model, funds are already transferred to platform wallet
        // This just returns an escrow reference ID
        Ok(format!("escrow_{}_{}", pool_id, investor_id))
    }

    /// Release funds from escrow to exporter
    /// This triggers an actual IDRX transfer on-chain
    pub async fn release_to_exporter(
        &self,
        pool_id: Uuid,
        exporter_id: Uuid,
        exporter_wallet: &str,
        amount: Decimal,
    ) -> AppResult<String> {
        let blockchain = self.get_blockchain_service()?;

        tracing::info!(
            "Releasing {} IDRX from pool {} to exporter {} (wallet: {})",
            amount, pool_id, exporter_id, exporter_wallet
        );

        // Execute on-chain transfer
        let tx_hash = blockchain.disburse_to_exporter(exporter_wallet, amount, pool_id).await?;

        tracing::info!(
            "Disbursement completed: {} - {} IDRX to exporter {}",
            tx_hash, amount, exporter_id
        );

        Ok(tx_hash)
    }

    /// Release investor returns from escrow
    /// This triggers an actual IDRX transfer on-chain
    pub async fn release_to_investor(
        &self,
        pool_id: Uuid,
        investor_id: Uuid,
        investor_wallet: &str,
        amount: Decimal,
    ) -> AppResult<String> {
        let blockchain = self.get_blockchain_service()?;

        tracing::info!(
            "Returning {} IDRX from pool {} to investor {} (wallet: {})",
            amount, pool_id, investor_id, investor_wallet
        );

        // Execute on-chain transfer
        let tx_hash = blockchain.return_to_investor(investor_wallet, amount, pool_id).await?;

        tracing::info!(
            "Investor return completed: {} - {} IDRX to investor {}",
            tx_hash, amount, investor_id
        );

        Ok(tx_hash)
    }

    /// Refund escrowed funds to investor (in case of pool cancellation)
    /// This triggers an actual IDRX transfer on-chain
    pub async fn refund_to_investor(
        &self,
        pool_id: Uuid,
        investor_id: Uuid,
        investor_wallet: &str,
        amount: Decimal,
    ) -> AppResult<String> {
        let blockchain = self.get_blockchain_service()?;

        tracing::info!(
            "Refunding {} IDRX from pool {} to investor {} (wallet: {})",
            amount, pool_id, investor_id, investor_wallet
        );

        // Execute on-chain transfer (same as return, different context)
        let tx_hash = blockchain.return_to_investor(investor_wallet, amount, pool_id).await?;

        tracing::info!(
            "Refund completed: {} - {} IDRX to investor {}",
            tx_hash, amount, investor_id
        );

        Ok(tx_hash)
    }

    /// Get escrow balance for platform wallet (total IDRX held)
    pub async fn get_platform_balance(&self) -> AppResult<Decimal> {
        let blockchain = self.get_blockchain_service()?;
        blockchain.get_platform_idrx_balance().await
    }

    /// Get IDRX balance for a specific address
    pub async fn get_address_balance(&self, address: &str) -> AppResult<Decimal> {
        let blockchain = self.get_blockchain_service()?;
        blockchain.get_idrx_balance(address).await
    }

    /// Get transaction history for transparency/audit
    pub async fn get_transaction_history(
        &self,
        address: &str,
        from_block: Option<u64>,
    ) -> AppResult<Vec<serde_json::Value>> {
        let blockchain = self.get_blockchain_service()?;
        blockchain.get_transfer_history(address, from_block).await
    }
}

impl Default for EscrowService {
    fn default() -> Self {
        Self::new()
    }
}
