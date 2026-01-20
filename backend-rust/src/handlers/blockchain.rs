use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::utils::{ApiResponse, Claims};
use super::AppState;

fn get_user_id(req: &HttpRequest) -> AppResult<Uuid> {
    req.extensions()
        .get::<Claims>()
        .map(|c| c.user_id())
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))
}

// ============ On-Chain Transparency Endpoints ============
// These endpoints provide transparent, verifiable on-chain data

/// GET /api/v1/blockchain/balance/{address}
/// Get IDRX balance for any address (public, transparent)
pub async fn get_idrx_balance(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let address = path.into_inner();
    let balance = state.blockchain_service.get_idrx_balance(&address).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "address": address,
            "balance": balance,
            "currency": "IDRX",
            "chain": "Base Mainnet",
            "chain_id": 8453
        }),
        "IDRX balance retrieved",
    )))
}

/// GET /api/v1/blockchain/platform-balance
/// Get platform wallet IDRX balance (public, transparent)
pub async fn get_platform_balance(
    state: web::Data<AppState>,
) -> AppResult<HttpResponse> {
    let balance = state.blockchain_service.get_platform_idrx_balance().await?;
    let platform_wallet = state.blockchain_service.get_platform_wallet();

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "platform_wallet": platform_wallet,
            "balance": balance,
            "currency": "IDRX",
            "chain": "Base Mainnet",
            "chain_id": 8453,
            "explorer_url": format!("{}/address/{}", state.config.block_explorer_url, platform_wallet)
        }),
        "Platform balance retrieved",
    )))
}

/// GET /api/v1/blockchain/verify/{tx_hash}
/// Verify any transaction on Base mainnet (public, transparent)
pub async fn verify_transaction(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let tx_hash = path.into_inner();
    let verified = state.blockchain_service.verify_transaction(&tx_hash).await?;
    let block = state.blockchain_service.get_transaction_block(&tx_hash).await?;
    let explorer_url = state.blockchain_service.get_explorer_url(&tx_hash);

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "tx_hash": tx_hash,
            "verified": verified,
            "block_number": block,
            "chain": "Base Mainnet",
            "chain_id": 8453,
            "explorer_url": explorer_url
        }),
        if verified { "Transaction verified on-chain" } else { "Transaction not found or failed" },
    )))
}

/// GET /api/v1/blockchain/transfers/{address}
/// Get IDRX transfer history for an address (public, transparent)
pub async fn get_transfer_history(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<TransferHistoryQuery>,
) -> AppResult<HttpResponse> {
    let address = path.into_inner();
    let transfers = state.blockchain_service
        .get_transfer_history(&address, query.from_block)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "address": address,
            "transfers": transfers,
            "count": transfers.len(),
            "chain": "Base Mainnet",
            "chain_id": 8453
        }),
        "Transfer history retrieved",
    )))
}

/// GET /api/v1/blockchain/my-transactions
/// Get authenticated user's on-chain transactions
pub async fn get_my_transactions(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;

    let (transactions, total) = state.tx_repo
        .find_blockchain_transactions_by_user(
            user_id,
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(20),
        )
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::paginated(
        transactions,
        total,
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(20),
    )))
}

/// GET /api/v1/blockchain/my-idrx-balance
/// Get authenticated user's IDRX wallet balance
pub async fn get_my_idrx_balance(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;

    let user = state.user_repo.find_by_id(user_id).await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let wallet = user.wallet_address
        .ok_or_else(|| AppError::ValidationError("Wallet address not set".to_string()))?;

    let balance = state.blockchain_service.get_idrx_balance(&wallet).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "wallet_address": wallet,
            "balance": balance,
            "currency": "IDRX",
            "chain": "Base Mainnet",
            "chain_id": 8453,
            "explorer_url": format!("{}/address/{}", state.config.block_explorer_url, wallet)
        }),
        "Your IDRX balance retrieved",
    )))
}

/// GET /api/v1/blockchain/pools/{id}/transactions
/// Get all on-chain transactions for a funding pool (public, transparent)
pub async fn get_pool_transactions(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let pool_id = path.into_inner();

    // Get pool to verify it exists
    let pool = state.funding_repo.find_by_id(pool_id).await?
        .ok_or_else(|| AppError::NotFound("Pool not found".to_string()))?;

    // Get all blockchain transactions for this pool
    let transactions = state.tx_repo.find_blockchain_transactions_by_pool(pool_id).await?;

    // Calculate totals
    let total_invested: f64 = transactions.iter()
        .filter(|t| t.tx_type == "investment")
        .map(|t| t.amount.to_string().parse::<f64>().unwrap_or(0.0))
        .sum();

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "pool_id": pool_id,
            "invoice_id": pool.invoice_id,
            "status": pool.status,
            "transactions": transactions,
            "transaction_count": transactions.len(),
            "total_invested_on_chain": total_invested,
            "currency": "IDRX",
            "chain": "Base Mainnet",
            "chain_id": 8453
        }),
        "Pool transactions retrieved",
    )))
}

/// GET /api/v1/blockchain/chain-info
/// Get current blockchain info (public)
pub async fn get_chain_info(
    state: web::Data<AppState>,
) -> AppResult<HttpResponse> {
    let chain_id = state.blockchain_service.get_chain_id().await?;
    let block_number = state.blockchain_service.get_block_number().await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "chain_id": chain_id,
            "chain_name": "Base Mainnet",
            "current_block": block_number,
            "rpc_url": "https://mainnet.base.org",
            "explorer_url": state.config.block_explorer_url,
            "idrx_contract": state.config.idrx_token_contract_addr,
            "platform_wallet": state.blockchain_service.get_platform_wallet()
        }),
        "Chain info retrieved",
    )))
}

#[derive(serde::Deserialize)]
pub struct TransferHistoryQuery {
    pub from_block: Option<u64>,
}

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}
