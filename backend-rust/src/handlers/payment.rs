use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::AdminGrantBalanceRequest;
use crate::utils::{ApiResponse, Claims};
use super::AppState;

fn get_user_id(req: &HttpRequest) -> AppResult<Uuid> {
    req.extensions()
        .get::<Claims>()
        .map(|c| c.user_id())
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))
}

/// GET /api/v1/payments/balance
pub async fn get_balance(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let balance = state.payment_service.get_balance(user_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(balance, "Balance retrieved successfully")))
}

/// POST /api/v1/payments/deposit
pub async fn deposit(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<DepositRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let balance = state.payment_service.deposit(user_id, body.amount, &body.tx_hash).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(balance, "Deposit successful")))
}

/// POST /api/v1/payments/withdraw
pub async fn withdraw(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<WithdrawRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let balance = state.payment_service.withdraw(user_id, body.amount, &body.to_address).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(balance, "Withdrawal initiated successfully")))
}

/// POST /api/v1/admin/balance/grant
pub async fn admin_grant_balance(
    state: web::Data<AppState>,
    body: web::Json<AdminGrantBalanceRequest>,
) -> AppResult<HttpResponse> {
    let balance = state.payment_service.admin_grant_balance(
        body.user_id,
        body.amount,
        &body.reason,
    ).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(balance, "Balance granted successfully")))
}

/// GET /api/v1/admin/platform/revenue
pub async fn get_platform_revenue(
    state: web::Data<AppState>,
) -> AppResult<HttpResponse> {
    let revenue = state.payment_service.get_platform_revenue().await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({ "revenue": revenue, "currency": "IDRX" }),
        "Platform revenue retrieved",
    )))
}

#[derive(serde::Deserialize)]
pub struct DepositRequest {
    pub amount: f64,
    pub tx_hash: String,
}

#[derive(serde::Deserialize)]
pub struct WithdrawRequest {
    pub amount: f64,
    pub to_address: String,
}
