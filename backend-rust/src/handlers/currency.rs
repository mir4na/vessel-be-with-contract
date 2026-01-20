use actix_web::{web, HttpResponse};

use crate::error::AppResult;
use crate::utils::ApiResponse;
use super::AppState;

/// GET /api/v1/currency/supported
pub async fn get_supported_currencies(
    state: web::Data<AppState>,
) -> AppResult<HttpResponse> {
    let currencies = state.currency_service.get_supported_currencies();
    Ok(HttpResponse::Ok().json(ApiResponse::success(currencies, "Supported currencies retrieved")))
}

/// POST /api/v1/currency/convert
pub async fn get_locked_exchange_rate(
    state: web::Data<AppState>,
    body: web::Json<ConvertRequest>,
) -> AppResult<HttpResponse> {
    let result = state.currency_service.get_locked_exchange_rate(
        &body.from_currency,
        body.amount,
    ).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result, "Exchange rate locked")))
}

/// GET /api/v1/currency/disbursement-estimate
pub async fn calculate_estimated_disbursement(
    state: web::Data<AppState>,
    query: web::Query<DisbursementQuery>,
) -> AppResult<HttpResponse> {
    let estimate = state.currency_service.calculate_disbursement_estimate(query.amount);
    Ok(HttpResponse::Ok().json(ApiResponse::success(estimate, "Disbursement estimate calculated")))
}

#[derive(serde::Deserialize)]
pub struct ConvertRequest {
    pub from_currency: String,
    pub amount: f64,
}

#[derive(serde::Deserialize)]
pub struct DisbursementQuery {
    pub amount: f64,
}
