use ethers::{
    contract::abigen,
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, H256, U256},
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::repository::{FundingRepository, InvoiceRepository};

use super::PinataService;

// Generate ERC20 contract bindings for IDRX token
abigen!(
    IERC20,
    r#"[
        function name() external view returns (string)
        function symbol() external view returns (string)
        function decimals() external view returns (uint8)
        function totalSupply() external view returns (uint256)
        function balanceOf(address account) external view returns (uint256)
        function transfer(address to, uint256 amount) external returns (bool)
        function allowance(address owner, address spender) external view returns (uint256)
        function approve(address spender, uint256 amount) external returns (bool)
        function transferFrom(address from, address to, uint256 amount) external returns (bool)
        event Transfer(address indexed from, address indexed to, uint256 value)
        event Approval(address indexed owner, address indexed spender, uint256 value)
    ]"#
);

/// Represents a verified on-chain IDRX transfer
#[derive(Debug, Clone, serde::Serialize)]
pub struct VerifiedTransfer {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    pub amount: Decimal,
    pub block_number: u64,
    pub confirmed: bool,
    pub explorer_url: String,
}

/// Transaction type for on-chain records
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OnChainTxType {
    Investment,
    Disbursement,
    Repayment,
    InvestorReturn,
    PlatformFee,
}

pub struct BlockchainService {
    config: Arc<Config>,
    provider: Provider<Http>,
    wallet: Option<LocalWallet>,
    invoice_repo: Arc<InvoiceRepository>,
    funding_repo: Arc<FundingRepository>,
    pinata_service: Arc<PinataService>,
    idrx_decimals: u8,
}

impl BlockchainService {
    pub async fn new(
        config: Arc<Config>,
        invoice_repo: Arc<InvoiceRepository>,
        funding_repo: Arc<FundingRepository>,
        pinata_service: Arc<PinataService>,
    ) -> AppResult<Self> {
        let provider = Provider::<Http>::try_from(&config.blockchain_rpc_url)
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        let wallet = if !config.private_key.is_empty() {
            let wallet: LocalWallet = config
                .private_key
                .parse()
                .map_err(|e: WalletError| AppError::BlockchainError(e.to_string()))?;
            Some(wallet.with_chain_id(config.chain_id))
        } else {
            tracing::warn!("Private key not configured, blockchain operations will be limited");
            None
        };

        // IDRX uses 2 decimals (like IDR)
        let idrx_decimals = 2u8;

        Ok(Self {
            config,
            provider,
            wallet,
            invoice_repo,
            funding_repo,
            pinata_service,
            idrx_decimals,
        })
    }

    // ==================== IDRX Token Methods ====================

    /// Get IDRX token contract instance
    fn get_idrx_contract(&self) -> AppResult<IERC20<Provider<Http>>> {
        let contract_addr: Address =
            self.config.idrx_token_contract_addr.parse().map_err(|_| {
                AppError::BlockchainError("Invalid IDRX contract address".to_string())
            })?;

        Ok(IERC20::new(contract_addr, Arc::new(self.provider.clone())))
    }

    /// Get IDRX balance for an address
    pub async fn get_idrx_balance(&self, address: &str) -> AppResult<Decimal> {
        let addr: Address = address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid address".to_string()))?;

        let contract = self.get_idrx_contract()?;
        let balance: U256 =
            contract.balance_of(addr).call().await.map_err(|e| {
                AppError::BlockchainError(format!("Failed to get IDRX balance: {}", e))
            })?;

        // Convert from token units to Decimal (IDRX has 2 decimals)
        let balance_f64 = balance.as_u128() as f64 / 10f64.powi(self.idrx_decimals as i32);
        Ok(Decimal::from_f64_retain(balance_f64).unwrap_or(Decimal::ZERO))
    }

    /// Get platform wallet IDRX balance (escrow balance)
    pub async fn get_platform_idrx_balance(&self) -> AppResult<Decimal> {
        self.get_idrx_balance(&self.config.platform_wallet_address)
            .await
    }

    /// Convert Decimal amount to token units (U256)
    fn to_token_units(&self, amount: Decimal) -> U256 {
        let multiplier = 10u128.pow(self.idrx_decimals as u32);
        let amount_u128 = (amount.to_f64().unwrap_or(0.0) * multiplier as f64) as u128;
        U256::from(amount_u128)
    }

    /// Verify an IDRX transfer transaction
    /// Returns details if the transfer is valid and matches expected parameters
    pub async fn verify_idrx_transfer(
        &self,
        tx_hash: &str,
        expected_to: &str,
        expected_amount: Decimal,
    ) -> AppResult<VerifiedTransfer> {
        let hash: H256 = tx_hash
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid transaction hash".to_string()))?;

        let expected_to_addr: Address = expected_to
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid recipient address".to_string()))?;

        // Get transaction receipt
        let receipt = self
            .provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get receipt: {}", e)))?
            .ok_or_else(|| {
                AppError::BlockchainError("Transaction not found or not confirmed".to_string())
            })?;

        // Check transaction succeeded
        let status = receipt
            .status
            .ok_or_else(|| AppError::BlockchainError("Transaction status unknown".to_string()))?;
        if status.as_u64() != 1 {
            return Err(AppError::BlockchainError("Transaction failed".to_string()));
        }

        // Parse Transfer events from logs
        let contract_addr: Address =
            self.config.idrx_token_contract_addr.parse().map_err(|_| {
                AppError::BlockchainError("Invalid IDRX contract address".to_string())
            })?;

        // Transfer event signature: Transfer(address,address,uint256)
        let transfer_topic = H256::from_slice(&ethers::utils::keccak256(
            "Transfer(address,address,uint256)",
        ));

        let mut verified_from = String::new();
        let mut verified_amount = Decimal::ZERO;
        let mut found_transfer = false;

        for log in receipt.logs.iter() {
            // Check if log is from IDRX contract and is a Transfer event
            if log.address == contract_addr
                && !log.topics.is_empty()
                && log.topics[0] == transfer_topic
            {
                // topics[1] = from, topics[2] = to (both padded to 32 bytes)
                if log.topics.len() >= 3 {
                    let to_addr = Address::from_slice(&log.topics[2].as_bytes()[12..32]);

                    if to_addr == expected_to_addr {
                        let from_addr = Address::from_slice(&log.topics[1].as_bytes()[12..32]);
                        verified_from = format!("{:?}", from_addr);

                        // Amount is in data field
                        let amount_u256 = U256::from_big_endian(&log.data);
                        let amount_f64 =
                            amount_u256.as_u128() as f64 / 10f64.powi(self.idrx_decimals as i32);
                        verified_amount =
                            Decimal::from_f64_retain(amount_f64).unwrap_or(Decimal::ZERO);
                        found_transfer = true;
                        break;
                    }
                }
            }
        }

        if !found_transfer {
            return Err(AppError::BlockchainError(
                "No matching IDRX transfer found to expected recipient".to_string(),
            ));
        }

        // Verify amount (allow small rounding difference)
        let diff = (verified_amount - expected_amount).abs();
        if diff > Decimal::from_f64_retain(0.01).unwrap() {
            return Err(AppError::BlockchainError(format!(
                "Transfer amount mismatch: expected {}, got {}",
                expected_amount, verified_amount
            )));
        }

        let block_number = receipt.block_number.map(|n| n.as_u64()).unwrap_or(0);

        Ok(VerifiedTransfer {
            tx_hash: tx_hash.to_string(),
            from: verified_from,
            to: expected_to.to_string(),
            amount: verified_amount,
            block_number,
            confirmed: true,
            explorer_url: self.get_explorer_url(tx_hash),
        })
    }

    /// Verify investment transfer - investor sends IDRX to platform wallet
    pub async fn verify_investment_transfer(
        &self,
        tx_hash: &str,
        expected_amount: Decimal,
    ) -> AppResult<VerifiedTransfer> {
        self.verify_idrx_transfer(
            tx_hash,
            &self.config.platform_wallet_address,
            expected_amount,
        )
        .await
    }

    /// Transfer IDRX from platform wallet to a recipient
    /// Used for disbursements to exporters and returns to investors
    pub async fn transfer_idrx(
        &self,
        to_address: &str,
        amount: Decimal,
        tx_type: OnChainTxType,
    ) -> AppResult<String> {
        let wallet = self.wallet.as_ref().ok_or_else(|| {
            AppError::BlockchainError("Platform wallet not configured".to_string())
        })?;

        let to_addr: Address = to_address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid recipient address".to_string()))?;

        let contract_addr: Address =
            self.config.idrx_token_contract_addr.parse().map_err(|_| {
                AppError::BlockchainError("Invalid IDRX contract address".to_string())
            })?;

        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        let contract = IERC20::new(contract_addr, Arc::new(client));

        let amount_units = self.to_token_units(amount);

        tracing::info!(
            "Transferring {} IDRX to {} for {:?}",
            amount,
            to_address,
            tx_type
        );

        let tx = contract.transfer(to_addr, amount_units);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| AppError::BlockchainError(format!("Transfer failed: {}", e)))?;

        let tx_hash = format!("{:?}", pending_tx.tx_hash());

        // Wait for confirmation
        let receipt = pending_tx
            .await
            .map_err(|e| AppError::BlockchainError(format!("Transaction failed: {}", e)))?
            .ok_or_else(|| AppError::BlockchainError("Transaction dropped".to_string()))?;

        if receipt.status.map(|s| s.as_u64()) != Some(1) {
            return Err(AppError::BlockchainError(
                "Transfer transaction failed".to_string(),
            ));
        }

        tracing::info!(
            "IDRX transfer completed: {} - {} IDRX to {} (block: {:?})",
            tx_hash,
            amount,
            to_address,
            receipt.block_number
        );

        Ok(tx_hash)
    }

    /// Disburse funds to exporter (transfer IDRX from platform to exporter)
    pub async fn disburse_to_exporter(
        &self,
        exporter_wallet: &str,
        amount: Decimal,
        pool_id: Uuid,
    ) -> AppResult<String> {
        tracing::info!(
            "Disbursing {} IDRX to exporter for pool {}",
            amount,
            pool_id
        );
        self.transfer_idrx(exporter_wallet, amount, OnChainTxType::Disbursement)
            .await
    }

    /// Return funds to investor (transfer IDRX from platform to investor)
    pub async fn return_to_investor(
        &self,
        investor_wallet: &str,
        amount: Decimal,
        pool_id: Uuid,
    ) -> AppResult<String> {
        tracing::info!("Returning {} IDRX to investor for pool {}", amount, pool_id);
        self.transfer_idrx(investor_wallet, amount, OnChainTxType::InvestorReturn)
            .await
    }

    /// Get all IDRX transfers for an address (for transparency/audit)
    pub async fn get_transfer_history(
        &self,
        address: &str,
        from_block: Option<u64>,
    ) -> AppResult<Vec<serde_json::Value>> {
        let addr: Address = address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid address".to_string()))?;

        let contract_addr: Address =
            self.config.idrx_token_contract_addr.parse().map_err(|_| {
                AppError::BlockchainError("Invalid IDRX contract address".to_string())
            })?;

        let transfer_topic = H256::from_slice(&ethers::utils::keccak256(
            "Transfer(address,address,uint256)",
        ));

        // Pad address to 32 bytes for topic filter
        let addr_topic = H256::from_slice(&{
            let mut padded = [0u8; 32];
            padded[12..32].copy_from_slice(addr.as_bytes());
            padded
        });

        let from = from_block.map(U64::from).unwrap_or(U64::from(0));

        // Get incoming transfers (to = address)
        let incoming_filter = Filter::new()
            .address(contract_addr)
            .topic0(transfer_topic)
            .topic2(addr_topic)
            .from_block(from);

        // Get outgoing transfers (from = address)
        let outgoing_filter = Filter::new()
            .address(contract_addr)
            .topic0(transfer_topic)
            .topic1(addr_topic)
            .from_block(from);

        let incoming_logs = self
            .provider
            .get_logs(&incoming_filter)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        let outgoing_logs = self
            .provider
            .get_logs(&outgoing_filter)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        let mut transfers = Vec::new();

        for log in incoming_logs.iter().chain(outgoing_logs.iter()) {
            if log.topics.len() >= 3 {
                let from_addr = Address::from_slice(&log.topics[1].as_bytes()[12..32]);
                let to_addr = Address::from_slice(&log.topics[2].as_bytes()[12..32]);
                let amount_u256 = U256::from_big_endian(&log.data);
                let amount = amount_u256.as_u128() as f64 / 10f64.powi(self.idrx_decimals as i32);

                transfers.push(serde_json::json!({
                    "tx_hash": format!("{:?}", log.transaction_hash.unwrap_or_default()),
                    "block_number": log.block_number.map(|n| n.as_u64()),
                    "from": format!("{:?}", from_addr),
                    "to": format!("{:?}", to_addr),
                    "amount": amount,
                    "direction": if to_addr == addr { "incoming" } else { "outgoing" },
                    "explorer_url": self.get_explorer_url(&format!("{:?}", log.transaction_hash.unwrap_or_default())),
                }));
            }
        }

        Ok(transfers)
    }

    pub async fn get_chain_id(&self) -> AppResult<u64> {
        let chain_id = self
            .provider
            .get_chainid()
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;
        Ok(chain_id.as_u64())
    }

    pub async fn get_block_number(&self) -> AppResult<u64> {
        let block_number = self
            .provider
            .get_block_number()
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;
        Ok(block_number.as_u64())
    }

    pub async fn get_balance(&self, address: &str) -> AppResult<U256> {
        let addr: Address = address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid address".to_string()))?;

        let balance = self
            .provider
            .get_balance(addr, None)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        Ok(balance)
    }

    pub async fn verify_transaction(&self, tx_hash: &str) -> AppResult<bool> {
        let hash: TxHash = tx_hash
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid transaction hash".to_string()))?;

        let receipt = self
            .provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        match receipt {
            Some(r) => Ok(r.status.map(|s| s.as_u64() == 1).unwrap_or(false)),
            None => Ok(false),
        }
    }

    pub async fn get_transaction_block(&self, tx_hash: &str) -> AppResult<Option<u64>> {
        let hash: TxHash = tx_hash
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid transaction hash".to_string()))?;

        let receipt = self
            .provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        Ok(receipt.and_then(|r| r.block_number).map(|n| n.as_u64()))
    }

    pub fn get_explorer_url(&self, tx_hash: &str) -> String {
        format!("{}/tx/{}", self.config.block_explorer_url, tx_hash)
    }

    pub fn get_contract_address(&self) -> &str {
        &self.config.invoice_nft_contract_addr
    }

    pub fn get_platform_wallet(&self) -> &str {
        &self.config.platform_wallet_address
    }

    // Note: Full NFT minting would require ABI bindings
    // This is a simplified version - in production, use ethers-rs contract bindings
    pub async fn mint_invoice_nft(
        &self,
        invoice_id: Uuid,
        owner_address: &str,
        metadata_uri: &str,
    ) -> AppResult<(i64, String)> {
        // In a full implementation, this would:
        // 1. Load the NFT contract ABI
        // 2. Call the mint function
        // 3. Wait for the transaction to be confirmed
        // 4. Return the token ID and tx hash

        // For now, return a placeholder
        tracing::info!(
            "Would mint NFT for invoice {} to {} with metadata {}",
            invoice_id,
            owner_address,
            metadata_uri
        );

        // Simulate token ID (in production, get from contract event)
        let token_id = chrono::Utc::now().timestamp();
        let tx_hash = format!("0x{}", ethers::utils::hex::encode(&[0u8; 32])); // Placeholder

        Ok((token_id, tx_hash))
    }

    pub async fn create_nft_metadata(&self, invoice_id: Uuid) -> AppResult<String> {
        let invoice = self
            .invoice_repo
            .find_by_id(invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        let metadata = serde_json::json!({
            "name": format!("VESSEL Invoice #{}", invoice.invoice_number),
            "description": format!("Tokenized invoice from VESSEL platform"),
            "image": "https://vessel.io/nft-image.png",
            "external_url": format!("https://vessel.io/invoices/{}", invoice_id),
            "attributes": [
                {
                    "trait_type": "Invoice Number",
                    "value": invoice.invoice_number
                },
                {
                    "trait_type": "Amount",
                    "value": invoice.amount.to_string()
                },
                {
                    "trait_type": "Currency",
                    "value": invoice.currency
                },
                {
                    "trait_type": "Buyer Country",
                    "value": invoice.buyer_country
                },
                {
                    "trait_type": "Grade",
                    "value": invoice.grade.unwrap_or_default()
                },
                {
                    "trait_type": "Due Date",
                    "value": invoice.due_date.to_string()
                }
            ]
        });

        let metadata_uri = self
            .pinata_service
            .upload_json(metadata, &format!("vessel-invoice-{}", invoice_id))
            .await?;

        Ok(metadata_uri)
    }
}
