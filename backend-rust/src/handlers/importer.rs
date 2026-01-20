use actix_web::{web, HttpResponse};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::ImporterPayRequest;
use crate::utils::ApiResponse;
use super::AppState;

/// GET /api/v1/public/payments/{payment_id}
/// Public endpoint for importers to view payment info
pub async fn get_payment_info(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let payment_id = path.into_inner();
    let payment = state.importer_payment_repo.find_by_id(payment_id).await?
        .ok_or_else(|| AppError::NotFound("Payment not found".to_string()))?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(payment, "Payment info retrieved")))
}

/// POST /api/v1/public/payments/{payment_id}/pay
/// Public endpoint for importers to submit payment
pub async fn pay(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<ImporterPayRequest>,
) -> AppResult<HttpResponse> {
    let payment_id = path.into_inner();

    // Get the payment
    let payment = state.importer_payment_repo.find_by_id(payment_id).await?
        .ok_or_else(|| AppError::NotFound("Payment not found".to_string()))?;

    // Validate payment status
    if payment.payment_status != "pending" {
        return Err(AppError::BadRequest("Payment is not in pending status".to_string()));
    }

    // Update payment with proof
    let amount = Decimal::from_f64(body.amount)
        .ok_or_else(|| AppError::ValidationError("Invalid amount".to_string()))?;
    let updated = state.importer_payment_repo.update_payment(
        payment_id,
        amount,
        &body.tx_hash,
    ).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(updated, "Payment submitted successfully")))
}
