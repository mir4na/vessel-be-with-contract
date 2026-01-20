use actix_multipart::Multipart;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{MitraApplyRequest, CreateVaPaymentRequest};
use crate::utils::{ApiResponse, Claims};
use super::AppState;

fn get_user_id(req: &HttpRequest) -> AppResult<Uuid> {
    req.extensions()
        .get::<Claims>()
        .map(|c| c.user_id())
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))
}

/// POST /api/v1/user/mitra/apply
pub async fn apply(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<MitraApplyRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let application = state.mitra_service.apply(user_id, body.into_inner()).await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(application, "Mitra application submitted successfully")))
}

/// GET /api/v1/user/mitra/status
pub async fn get_status(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let status = state.mitra_service.get_status(user_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(status, "Mitra status retrieved successfully")))
}

/// POST /api/v1/user/mitra/documents
pub async fn upload_document(
    state: web::Data<AppState>,
    req: HttpRequest,
    mut payload: Multipart,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;

    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut document_type: Option<String> = None;

    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| AppError::BadRequest(e.to_string()))?;
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap_or("");

        match field_name {
            "file" => {
                file_name = content_disposition.get_filename().map(|s| s.to_string());
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| AppError::BadRequest(e.to_string()))?;
                    data.extend_from_slice(&chunk);
                }
                file_data = Some(data);
            }
            "document_type" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| AppError::BadRequest(e.to_string()))?;
                    data.extend_from_slice(&chunk);
                }
                document_type = Some(String::from_utf8_lossy(&data).to_string());
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| AppError::ValidationError("File is required".to_string()))?;
    let file_name = file_name.ok_or_else(|| AppError::ValidationError("Filename is required".to_string()))?;
    let document_type = document_type.ok_or_else(|| AppError::ValidationError("Document type is required".to_string()))?;

    // Validate document type
    let valid_types = ["nib", "akta_pendirian", "ktp_direktur"];
    if !valid_types.contains(&document_type.as_str()) {
        return Err(AppError::ValidationError(format!(
            "Invalid document type. Must be one of: {}",
            valid_types.join(", ")
        )));
    }

    let application = state.mitra_service.upload_document(
        user_id,
        &document_type,
        file_data,
        &file_name,
    ).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(application, "Document uploaded successfully")))
}

/// GET /api/v1/mitra/invoices/active
pub async fn get_active_invoices(
    _state: web::Data<AppState>,
    req: HttpRequest,
    _query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let _user_id = get_user_id(&req)?;
    // Stub - return empty list
    let empty: Vec<serde_json::Value> = vec![];
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(empty, 0, 1, 10)))
}

/// GET /api/v1/mitra/pools/{id}/breakdown
pub async fn get_repayment_breakdown(
    _state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let pool_id = path.into_inner();
    // Stub - return basic breakdown
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "pool_id": pool_id,
            "invoice_number": "INV-001",
            "principal_amount": 0.0,
            "total_interest": 0.0,
            "platform_fee": 0.0,
            "total_repayment": 0.0
        }),
        "Repayment breakdown retrieved"
    )))
}

/// GET /api/v1/mitra/payment-methods
pub async fn get_va_payment_methods() -> AppResult<HttpResponse> {
    let methods = crate::models::get_va_payment_methods();
    Ok(HttpResponse::Ok().json(ApiResponse::success(methods, "Payment methods retrieved")))
}

/// POST /api/v1/mitra/repayment/va
pub async fn create_va_payment(
    _state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateVaPaymentRequest>,
) -> AppResult<HttpResponse> {
    let _user_id = get_user_id(&req)?;
    // Stub
    Ok(HttpResponse::Created().json(ApiResponse::success(
        serde_json::json!({
            "pool_id": body.pool_id,
            "bank_code": body.bank_code,
            "va_number": "8888123456789",
            "status": "pending"
        }),
        "VA payment created"
    )))
}

/// GET /api/v1/mitra/repayment/va/{id}
pub async fn get_va_payment_status(
    _state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let payment_id = path.into_inner();
    // Stub
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "payment_id": payment_id,
            "status": "pending"
        }),
        "VA payment status retrieved"
    )))
}

/// POST /api/v1/mitra/repayment/va/{id}/simulate-pay
pub async fn simulate_va_payment(
    _state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let payment_id = path.into_inner();
    // Stub
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "payment_id": payment_id,
            "status": "paid",
            "message": "Payment simulated"
        }),
        "VA payment simulated"
    )))
}

// ============ Admin Mitra Endpoints ============

/// GET /api/v1/admin/mitra/pending
pub async fn get_pending_applications(
    state: web::Data<AppState>,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let (applications, total) = state.mitra_service.get_pending_applications(
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    ).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(applications, total, query.page.unwrap_or(1), query.per_page.unwrap_or(10))))
}

/// GET /api/v1/admin/mitra/{id}
pub async fn get_application(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let application_id = path.into_inner();
    let application = state.mitra_service.get_application(application_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(application, "Application retrieved")))
}

/// POST /api/v1/admin/mitra/{id}/approve
pub async fn approve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let admin_id = get_user_id(&req)?;
    let application_id = path.into_inner();
    let application = state.mitra_service.approve(application_id, admin_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(application, "Mitra application approved")))
}

/// POST /api/v1/admin/mitra/{id}/reject
pub async fn reject(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<RejectRequest>,
) -> AppResult<HttpResponse> {
    let admin_id = get_user_id(&req)?;
    let application_id = path.into_inner();
    let application = state.mitra_service.reject(application_id, admin_id, &body.reason).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(application, "Mitra application rejected")))
}

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(serde::Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}
