use chrono::{Duration, Utc};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::{
    FundingPool, FundingPoolResponse, InvestRequest, Investment, InvestorPortfolio,
    InvoiceDashboard, MitraDashboard, TimelineStatus,
};
use crate::repository::{
    FundingRepository, InvoiceRepository, RiskQuestionnaireRepository, TransactionRepository,
    UserRepository,
};

use super::{BlockchainService, EmailService, EscrowService};

pub struct FundingService {
    funding_repo: Arc<FundingRepository>,
    invoice_repo: Arc<InvoiceRepository>,
    tx_repo: Arc<TransactionRepository>,
    user_repo: Arc<UserRepository>,
    rq_repo: Arc<RiskQuestionnaireRepository>,
    email_service: Arc<EmailService>,
    escrow_service: Arc<EscrowService>,
    blockchain_service: Arc<BlockchainService>,
    config: Arc<Config>,
}

impl FundingService {
    pub fn new(
        funding_repo: Arc<FundingRepository>,
        invoice_repo: Arc<InvoiceRepository>,
        tx_repo: Arc<TransactionRepository>,
        user_repo: Arc<UserRepository>,
        rq_repo: Arc<RiskQuestionnaireRepository>,
        email_service: Arc<EmailService>,
        escrow_service: Arc<EscrowService>,
        blockchain_service: Arc<BlockchainService>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            funding_repo,
            invoice_repo,
            tx_repo,
            user_repo,
            rq_repo,
            email_service,
            escrow_service,
            blockchain_service,
            config,
        }
    }

    pub async fn create_pool(&self, invoice_id: Uuid) -> AppResult<FundingPool> {
        let invoice = self
            .invoice_repo
            .find_by_id(invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        // Check if invoice is approved or tokenized
        if invoice.status != "approved" && invoice.status != "tokenized" {
            return Err(AppError::BadRequest(
                "Invoice must be approved first".to_string(),
            ));
        }

        // Check if pool already exists
        if self
            .funding_repo
            .find_by_invoice(invoice_id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(
                "Pool already exists for this invoice".to_string(),
            ));
        }

        // Calculate tranche amounts
        let target_amount = invoice.amount;
        let priority_ratio = invoice.priority_ratio.to_f64().unwrap_or(80.0) / 100.0;
        let priority_target = target_amount * Decimal::from_f64(priority_ratio).unwrap();
        let catalyst_target = target_amount - priority_target;

        // Get interest rates
        let priority_rate = invoice.priority_interest_rate.unwrap_or(Decimal::from(10));
        let catalyst_rate = invoice.catalyst_interest_rate.unwrap_or(Decimal::from(15));

        // Calculate deadline
        let deadline = Utc::now() + Duration::days(invoice.funding_duration_days as i64);

        // Create pool
        let pool = self
            .funding_repo
            .create_pool(
                invoice_id,
                target_amount,
                priority_target,
                catalyst_target,
                priority_rate,
                catalyst_rate,
                deadline,
            )
            .await?;

        // Update invoice status to funding
        self.invoice_repo
            .update_status(invoice_id, "funding")
            .await?;

        Ok(pool)
    }

    pub async fn get_pool(&self, id: Uuid) -> AppResult<FundingPoolResponse> {
        let pool = self
            .funding_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pool not found".to_string()))?;

        let invoice = self.invoice_repo.find_by_id(pool.invoice_id).await?;

        self.build_pool_response(pool, invoice)
    }

    pub async fn list_pools(
        &self,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<FundingPoolResponse>, i64)> {
        let (pools, total) = self.funding_repo.find_all(page, per_page).await?;

        let mut responses = Vec::new();
        for pool in pools {
            let invoice = self.invoice_repo.find_by_id(pool.invoice_id).await?;
            responses.push(self.build_pool_response(pool, invoice)?);
        }

        Ok((responses, total))
    }

    /// Investment flow (ON-CHAIN):
    /// 1. Investor transfers IDRX to platform wallet (done before calling this)
    /// 2. This endpoint verifies the on-chain transaction
    /// 3. Records the investment with verified tx_hash
    /// 4. Updates pool funded amounts
    ///    All transactions are transparent and verifiable on Base mainnet
    pub async fn invest(&self, investor_id: Uuid, req: InvestRequest) -> AppResult<Investment> {
        // Validate TnC acceptance
        if !req.tnc_accepted {
            return Err(AppError::ValidationError(
                "Must accept terms and conditions".to_string(),
            ));
        }

        // Validate tx_hash is provided
        if req.tx_hash.is_empty() {
            return Err(AppError::ValidationError(
                "Transaction hash is required. Please transfer IDRX to platform wallet first."
                    .to_string(),
            ));
        }

        // Get pool
        let pool = self
            .funding_repo
            .find_by_id(req.pool_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pool not found".to_string()))?;

        // Check pool status
        if pool.status != "open" {
            return Err(AppError::PoolNotOpen);
        }

        // Check if investor already invested in this pool
        if self
            .funding_repo
            .find_investment_by_pool_and_investor(req.pool_id, investor_id)
            .await?
            .is_some()
        {
            return Err(AppError::Forbidden(
                "You have already invested in this pool. Only one investment per pool is allowed."
                    .to_string(),
            ));
        }

        // Check tranche
        let is_catalyst = req.tranche == "catalyst";

        if is_catalyst {
            // Check catalyst consents
            if let Some(consents) = &req.catalyst_consents {
                if !consents.all_accepted() {
                    return Err(AppError::ValidationError(
                        "All catalyst consents must be accepted".to_string(),
                    ));
                }
            } else {
                return Err(AppError::ValidationError(
                    "Catalyst consents required for catalyst tranche".to_string(),
                ));
            }

            // Check if catalyst is unlocked
            if !self.rq_repo.is_catalyst_unlocked(investor_id).await? {
                return Err(AppError::CatalystNotUnlocked);
            }
        }

        // Get investor
        let investor = self
            .user_repo
            .find_by_id(investor_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Investor not found".to_string()))?;

        // Investor must have a wallet address
        let _investor_wallet = investor.wallet_address.as_ref().ok_or_else(|| {
            AppError::ValidationError("Investor wallet address not set".to_string())
        })?;

        let amount = Decimal::from_f64(req.amount)
            .ok_or_else(|| AppError::ValidationError("Invalid amount".to_string()))?;

        // ============ ON-CHAIN VERIFICATION ============
        // Verify the IDRX transfer transaction on Base mainnet
        // This ensures the investor actually sent IDRX to the platform wallet
        let verified_transfer = self.blockchain_service
            .verify_investment_transfer(&req.tx_hash, amount)
            .await
            .map_err(|e| AppError::BlockchainError(format!(
                "Failed to verify on-chain transfer: {}. Please ensure you have transferred {} IDRX to the platform wallet.",
                e, amount
            )))?;

        tracing::info!(
            "Verified on-chain investment: {} IDRX from {} (tx: {}, block: {})",
            verified_transfer.amount,
            verified_transfer.from,
            verified_transfer.tx_hash,
            verified_transfer.block_number
        );

        // Check tranche availability
        let (available, interest_rate) = if is_catalyst {
            let available = pool.catalyst_target - pool.catalyst_funded;
            (available, pool.catalyst_interest_rate)
        } else {
            let available = pool.priority_target - pool.priority_funded;
            (available, pool.priority_interest_rate)
        };

        // Check 10-90% Limits
        let min_limit = pool.target_amount * Decimal::from_f64(0.1).unwrap();
        let max_limit = pool.target_amount * Decimal::from_f64(0.9).unwrap();
        let pool_remaining = pool.target_amount - pool.funded_amount;

        if pool_remaining >= min_limit {
            if amount < min_limit {
                return Err(AppError::ValidationError(format!(
                    "Minimum investment is 10% of target ({})",
                    min_limit
                )));
            }
            if amount > max_limit {
                return Err(AppError::ValidationError(format!(
                    "Maximum investment is 90% of target ({})",
                    max_limit
                )));
            }
        } else {
            // If remaining is small (last chunk), allowing exact fill or remaining
            if amount > pool_remaining {
                return Err(AppError::ValidationError(format!(
                    "Amount exceeds remaining pool capacity ({})",
                    pool_remaining
                )));
            }
        }

        if amount > available {
            return Err(AppError::BadRequest(format!(
                "Only {} available in {} tranche",
                available, req.tranche
            )));
        }

        // Forward funds to InvoicePool Contract (Platform -> Contract)
        // Since we verified the user sent to Platform, we now move it to Contract
        // Note: verify_investment_transfer confirmed user sent to Platform Wallet

        let contract_addr = &self.config.invoice_pool_contract_addr;
        let _forward_tx = self
            .blockchain_service
            .transfer_idrx(
                contract_addr,
                amount,
                crate::services::blockchain_service::OnChainTxType::Investment,
            )
            .await
            .map_err(|e| {
                AppError::BlockchainError(format!("Failed to forward funds to contract: {}", e))
            })?;

        // Record on Smart Contract
        // Get Token ID
        let nft = self
            .invoice_repo
            .find_nft_by_invoice(pool.invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("NFT record not found for invoice".to_string()))?;

        let token_id = nft.token_id.ok_or_else(|| {
            AppError::InternalError("Token ID missing from NFT record".to_string())
        })?;

        // Call contract
        let _contract_tx = self
            .blockchain_service
            .record_investment_on_chain(token_id, &verified_transfer.from, amount)
            .await?;

        // Calculate expected return
        let invoice = self
            .invoice_repo
            .find_by_id(pool.invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        let days_until_due = (invoice.due_date - chrono::Utc::now().date_naive()).num_days();
        let annual_rate = interest_rate.to_f64().unwrap_or(10.0) / 100.0;
        let daily_rate = annual_rate / 365.0;
        let interest = amount.to_f64().unwrap_or(0.0) * daily_rate * days_until_due as f64;
        let expected_return = Decimal::from_f64(amount.to_f64().unwrap_or(0.0) + interest).unwrap();

        // Record the on-chain transaction in database for audit trail
        self.tx_repo
            .create_blockchain_transaction(
                investor_id,
                "investment",
                amount,
                &req.tx_hash,
                verified_transfer.block_number as i64,
                Some(req.pool_id),
                Some("pool"),
                Some(&format!("On-chain IDRX investment in pool {}", req.pool_id)),
                &verified_transfer.explorer_url,
            )
            .await?;

        // Create investment record with verified tx_hash
        let investment = self
            .funding_repo
            .create_investment(
                req.pool_id,
                investor_id,
                amount,
                expected_return,
                &req.tranche,
                &req.tx_hash,
            )
            .await?;

        // Update pool funded amounts
        let new_funded = pool.funded_amount + amount;
        let (new_priority_funded, new_catalyst_funded) = if is_catalyst {
            (pool.priority_funded, pool.catalyst_funded + amount)
        } else {
            (pool.priority_funded + amount, pool.catalyst_funded)
        };

        let investor_count = self
            .funding_repo
            .count_investors_in_pool(req.pool_id)
            .await? as i32;

        self.funding_repo
            .update_funded_amount(
                req.pool_id,
                new_funded,
                new_priority_funded,
                new_catalyst_funded,
                investor_count,
            )
            .await?;

        // Check if pool is now fully funded
        if new_funded >= pool.target_amount {
            self.funding_repo.set_filled(req.pool_id).await?;
            self.invoice_repo
                .update_status(pool.invoice_id, "funded")
                .await?;

            // Notify exporter
            if let Some(exporter) = self.user_repo.find_by_id(invoice.exporter_id).await? {
                let _ = self
                    .email_service
                    .send_pool_funded_notification(
                        &exporter.email,
                        &invoice.invoice_number,
                        pool.target_amount.to_f64().unwrap_or(0.0),
                    )
                    .await;
            }
        }

        // Send confirmation email with on-chain tx details
        let _ = self
            .email_service
            .send_investment_confirmation(
                &investor.email,
                &invoice.invoice_number,
                amount.to_f64().unwrap_or(0.0),
                &req.tranche,
                expected_return.to_f64().unwrap_or(0.0),
            )
            .await;

        tracing::info!(
            "Investment recorded: {} IDRX in pool {} by investor {} - viewable at {}",
            amount,
            req.pool_id,
            investor_id,
            verified_transfer.explorer_url
        );

        Ok(investment)
    }

    pub async fn get_investor_portfolio(&self, investor_id: Uuid) -> AppResult<InvestorPortfolio> {
        let (
            total_funding,
            total_expected,
            total_realized,
            priority_alloc,
            catalyst_alloc,
            active_count,
            completed_count,
        ) = self
            .funding_repo
            .get_investor_portfolio_stats(investor_id)
            .await?;

        let user = self
            .user_repo
            .find_by_id(investor_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(InvestorPortfolio {
            total_funding: total_funding.to_f64().unwrap_or(0.0),
            total_expected_gain: total_expected.to_f64().unwrap_or(0.0),
            total_realized_gain: total_realized.to_f64().unwrap_or(0.0),
            priority_allocation: priority_alloc.to_f64().unwrap_or(0.0),
            catalyst_allocation: catalyst_alloc.to_f64().unwrap_or(0.0),
            active_investments: active_count as i32,
            completed_deals: completed_count as i32,
            available_balance: user.balance_idrx.to_f64().unwrap_or(0.0),
        })
    }

    pub async fn get_mitra_dashboard(&self, mitra_id: Uuid) -> AppResult<MitraDashboard> {
        let (invoices, _) = self
            .invoice_repo
            .find_by_exporter(mitra_id, None, 1, 100)
            .await?;

        let mut total_financing = 0.0;
        let mut total_owed = 0.0;
        let mut total_days = 0;
        let mut active_invoices = Vec::new();

        for invoice in invoices.iter() {
            if invoice.status == "funded" || invoice.status == "funding" {
                let amount = invoice.amount.to_f64().unwrap_or(0.0);
                total_financing += amount;

                // Calculate total owed (principal + interest)
                let interest_rate = invoice
                    .priority_interest_rate
                    .unwrap_or(Decimal::from(10))
                    .to_f64()
                    .unwrap_or(10.0);
                let days_until_due =
                    (invoice.due_date - chrono::Utc::now().date_naive()).num_days();
                let interest = amount * (interest_rate / 100.0) * (days_until_due as f64 / 365.0);
                let owed = amount + interest;
                total_owed += owed;
                total_days += days_until_due as i32;

                active_invoices.push(InvoiceDashboard {
                    invoice_id: invoice.id,
                    invoice_number: invoice.invoice_number.clone(),
                    buyer_name: invoice.buyer_name.clone(),
                    buyer_country: invoice.buyer_country.clone(),
                    due_date: invoice.due_date.and_hms_opt(0, 0, 0).unwrap(),
                    amount,
                    status: invoice.status.clone(),
                    status_color: if days_until_due > 14 {
                        "green"
                    } else if days_until_due > 0 {
                        "yellow"
                    } else {
                        "red"
                    }
                    .to_string(),
                    days_remaining: days_until_due as i32,
                    funded_amount: amount, // Simplified
                    total_owed: owed,
                });
            }
        }

        let avg_tenor = if !active_invoices.is_empty() {
            total_days / active_invoices.len() as i32
        } else {
            0
        };

        Ok(MitraDashboard {
            total_active_financing: total_financing,
            total_owed_to_investors: total_owed,
            average_remaining_tenor: avg_tenor,
            active_invoices,
            timeline_status: TimelineStatus {
                fundraising_complete: false,
                disbursement_complete: false,
                repayment_complete: false,
                current_step: "Fundraising".to_string(),
            },
        })
    }

    fn build_pool_response(
        &self,
        pool: FundingPool,
        invoice: Option<crate::models::Invoice>,
    ) -> AppResult<FundingPoolResponse> {
        let remaining = (pool.target_amount - pool.funded_amount)
            .to_f64()
            .unwrap_or(0.0);
        let percentage = if pool.target_amount > Decimal::ZERO {
            (pool.funded_amount / pool.target_amount * Decimal::from(100))
                .to_f64()
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let priority_remaining = (pool.priority_target - pool.priority_funded)
            .to_f64()
            .unwrap_or(0.0);
        let catalyst_remaining = (pool.catalyst_target - pool.catalyst_funded)
            .to_f64()
            .unwrap_or(0.0);

        let priority_pct = if pool.priority_target > Decimal::ZERO {
            (pool.priority_funded / pool.priority_target * Decimal::from(100))
                .to_f64()
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let catalyst_pct = if pool.catalyst_target > Decimal::ZERO {
            (pool.catalyst_funded / pool.catalyst_target * Decimal::from(100))
                .to_f64()
                .unwrap_or(0.0)
        } else {
            0.0
        };

        Ok(FundingPoolResponse {
            pool,
            remaining_amount: remaining,
            percentage_funded: percentage,
            priority_remaining,
            catalyst_remaining,
            priority_percentage_funded: priority_pct,
            catalyst_percentage_funded: catalyst_pct,
            invoice,
        })
    }

    pub async fn close_pool(&self, pool_id: Uuid) -> AppResult<FundingPool> {
        let pool = self
            .funding_repo
            .find_by_id(pool_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pool not found".to_string()))?;

        if pool.status != "open" && pool.status != "filled" {
            return Err(AppError::BadRequest(
                "Pool can only be closed when open or filled".to_string(),
            ));
        }

        // Close on-chain
        let nft = self
            .invoice_repo
            .find_nft_by_invoice(pool.invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("NFT record not found".to_string()))?;

        let token_id = nft.token_id.ok_or_else(|| {
            AppError::InternalError("Token ID missing from NFT record".to_string())
        })?;

        let _tx_hash = self
            .blockchain_service
            .close_pool_on_chain(token_id)
            .await?;

        // Close in DB
        let closed_pool = self.funding_repo.set_closed(pool_id).await?;

        // Update invoice status back to tokenized (pool closed without full funding)
        self.invoice_repo
            .update_status(pool.invoice_id, "tokenized")
            .await?;

        Ok(closed_pool)
    }

    /// Get all funding pools for a specific mitra (exporter)
    pub async fn get_mitra_pools(
        &self,
        mitra_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<FundingPoolResponse>, i64)> {
        let (pools, total) = self
            .funding_repo
            .find_by_exporter(mitra_id, page, per_page)
            .await?;

        let mut responses = Vec::new();
        for pool in pools {
            let invoice = self.invoice_repo.find_by_id(pool.invoice_id).await?;
            responses.push(self.build_pool_response(pool, invoice)?);
        }

        Ok((responses, total))
    }

    /// Get funding pool for a specific invoice (with ownership check)
    pub async fn get_pool_by_invoice(
        &self,
        mitra_id: Uuid,
        invoice_id: Uuid,
    ) -> AppResult<FundingPoolResponse> {
        // First verify ownership
        let invoice = self
            .invoice_repo
            .find_by_id(invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        if invoice.exporter_id != mitra_id {
            return Err(AppError::Forbidden("Not the invoice owner".to_string()));
        }

        // Get the pool
        let pool = self
            .funding_repo
            .find_by_invoice(invoice_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound("No funding pool found for this invoice".to_string())
            })?;

        self.build_pool_response(pool, Some(invoice))
    }

    pub async fn repay_invoice(
        &self,
        exporter_id: Uuid,
        invoice_id: Uuid,
        req: crate::models::RepayInvoiceRequest,
    ) -> AppResult<String> {
        // 1. Get Invoice & ownership check
        let invoice = self
            .invoice_repo
            .find_by_id(invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        if invoice.exporter_id != exporter_id {
            return Err(AppError::Forbidden("Not the invoice owner".to_string()));
        }

        // 2. Get Pool
        let pool = self
            .funding_repo
            .find_by_invoice(invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pool not found".to_string()))?;

        if pool.status != "disbursed" {
            // Allow if it's already closed/repaid? Idempotency? For now strict check.
            return Err(AppError::ValidationError(
                "Pool must be disbursed to be repaid".to_string(),
            ));
        }

        let payment_amount = Decimal::from_f64(req.amount)
            .ok_or_else(|| AppError::ValidationError("Invalid amount".to_string()))?;

        // 3. Verify Mitra Transfer (Mitra -> Platform)
        // Verify user sent funds to platform wallet
        let _verified_transfer = self
            .blockchain_service
            .verify_investment_transfer(&req.tx_hash, payment_amount)
            .await
            .map_err(|e| {
                AppError::BlockchainError(format!("Failed to verify repayment transfer: {}", e))
            })?;

        // 4. Forward Funds (Platform -> Contract)
        let contract_addr = &self.config.invoice_pool_contract_addr;
        let _forward_tx = self
            .blockchain_service
            .transfer_idrx(
                contract_addr,
                payment_amount,
                crate::services::blockchain_service::OnChainTxType::Repayment,
            )
            .await?;

        // 5. Calculate Investor Returns
        let investments = self.funding_repo.find_investments_by_pool(pool.id).await?;

        let mut returns: Vec<Decimal> = Vec::new();

        // Logic: Iterate investments and determine return amount.
        // For Hackathon/MVP: we assume full repayment triggers full expected return payment.
        // We push expected_return for each investment.
        // NOTE: If payment_amount < sum(expected_returns), this will fail on contract side (insufficient balance).
        // The frontend must ensure amount covers total obligation.

        for inv in investments {
            returns.push(inv.expected_return);

            // Update investment status locally
            self.funding_repo
                .set_investment_repaid(inv.id, inv.expected_return, "pending_on_chain")
                .await?;
        }

        // 6. Record on Chain (Contract distributes funds)
        let nft = self
            .invoice_repo
            .find_nft_by_invoice(invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("NFT record missing".to_string()))?;

        let token_id = nft
            .token_id
            .ok_or_else(|| AppError::InternalError("Token ID missing".to_string()))?;

        let tx_hash = self
            .blockchain_service
            .record_repayment_on_chain(token_id, payment_amount, returns)
            .await?;

        // 7. Update Invoice/Pool status
        let _ = self
            .invoice_repo
            .update_status(invoice_id, "repaid")
            .await?;
        let _ = self.funding_repo.set_closed(pool.id).await?;

        // We should also update stored investments with the real return tx hash if available or use the block tx hash
        // Skipping detailed per-investment tx hash update for now, or use the same hash.

        Ok(tx_hash)
    }
}
