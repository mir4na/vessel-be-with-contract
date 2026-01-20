use actix_web::{web, HttpResponse};

use crate::error::AppResult;
use crate::models::{
    RegisterRequest, LoginRequest, VerifyOtpRequest, SendOtpRequest,
    WalletLoginRequest, RefreshTokenRequest, GetNonceRequest, InvestorWalletRegisterRequest,
};
use crate::utils::ApiResponse;
use super::AppState;

/// POST /api/v1/auth/send-otp
/// For mitra/admin registration - not needed for investors
pub async fn send_otp(
    state: web::Data<AppState>,
    body: web::Json<SendOtpRequest>,
) -> AppResult<HttpResponse> {
    state.otp_service.send_otp(&body.email, "registration").await?;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_message("OTP sent successfully")))
}

/// POST /api/v1/auth/verify-otp
/// For mitra/admin registration - not needed for investors
pub async fn verify_otp(
    state: web::Data<AppState>,
    body: web::Json<VerifyOtpRequest>,
) -> AppResult<HttpResponse> {
    let result = state.auth_service.verify_otp(body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result, "Email verified successfully")))
}

/// POST /api/v1/auth/register
/// For mitra/admin registration only - investors use wallet-connect
pub async fn register(
    state: web::Data<AppState>,
    body: web::Json<RegisterRequest>,
) -> AppResult<HttpResponse> {
    let result = state.auth_service.register(body.into_inner()).await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(result, "Registration successful")))
}

/// POST /api/v1/auth/login
/// For mitra/admin login only - investors use wallet-connect
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> AppResult<HttpResponse> {
    let result = state.auth_service.login(body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result, "Login successful")))
}

/// POST /api/v1/auth/wallet/nonce
/// Get nonce for wallet signature (for investors)
pub async fn get_wallet_nonce(
    state: web::Data<AppState>,
    body: web::Json<GetNonceRequest>,
) -> AppResult<HttpResponse> {
    let result = state.auth_service.get_wallet_nonce(&body.wallet_address).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result, "Nonce generated")))
}

/// POST /api/v1/auth/wallet/login
/// Wallet-based login for investors only
pub async fn wallet_login(
    state: web::Data<AppState>,
    body: web::Json<WalletLoginRequest>,
) -> AppResult<HttpResponse> {
    let result = state.auth_service.wallet_login(body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result, "Wallet login successful")))
}

/// POST /api/v1/auth/wallet/register
/// Wallet-based registration for investors only
pub async fn wallet_register(
    state: web::Data<AppState>,
    body: web::Json<InvestorWalletRegisterRequest>,
) -> AppResult<HttpResponse> {
    let result = state.auth_service.register_investor_wallet(body.into_inner()).await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(result, "Investor registered successfully")))
}

/// POST /api/v1/auth/refresh
pub async fn refresh_token(
    state: web::Data<AppState>,
    body: web::Json<RefreshTokenRequest>,
) -> AppResult<HttpResponse> {
    let result = state.auth_service.refresh_token(&body.refresh_token).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "access_token": result.0,
            "refresh_token": result.1
        }),
        "Token refreshed successfully"
    )))
}
