use actix_multipart::Multipart;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use uuid::Uuid;

use super::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{
    AdminReviewInvoiceRequest, CreateInvoiceFundingRequest, RepeatBuyerCheckRequest,
};
use crate::utils::{ApiResponse, Claims};

fn get_user_id(req: &HttpRequest) -> AppResult<Uuid> {
    req.extensions()
        .get::<Claims>()
        .map(|c| c.user_id())
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))
}

/// POST /api/v1/invoices - Create a simple invoice (uses funding request flow)
pub async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateInvoiceFundingRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let invoice = state
        .invoice_service
        .create_funding_request(user_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(
        invoice,
        "Invoice created successfully",
    )))
}

/// POST /api/v1/invoices/funding-request
pub async fn create_funding_request(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateInvoiceFundingRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let invoice = state
        .invoice_service
        .create_funding_request(user_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(
        invoice,
        "Funding request created successfully",
    )))
}

/// POST /api/v1/invoices/check-repeat-buyer
pub async fn check_repeat_buyer(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<RepeatBuyerCheckRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let result = state
        .invoice_service
        .check_repeat_buyer(user_id, &body.buyer_company_name)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result, "Repeat buyer check completed")))
}

/// GET /api/v1/invoices
pub async fn list(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<InvoiceListQuery>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let (invoices, total) = state
        .invoice_service
        .list_by_exporter(
            user_id,
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(10),
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(
        invoices,
        total,
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    )))
}

/// GET /api/v1/invoices/fundable
pub async fn list_fundable(
    state: web::Data<AppState>,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let (invoices, total) = state
        .invoice_service
        .list_fundable(query.page.unwrap_or(1), query.per_page.unwrap_or(10))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(
        invoices,
        total,
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    )))
}

/// GET /api/v1/invoices/{id}
pub async fn get(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let invoice_id = path.into_inner();
    let invoice = state.invoice_service.get_invoice(invoice_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        invoice,
        "Invoice retrieved successfully",
    )))
}

/// PUT /api/v1/invoices/{id} - Not implemented (invoices are immutable after creation)
pub async fn update(
    _state: web::Data<AppState>,
    _req: HttpRequest,
    _path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    Err(AppError::BadRequest(
        "Invoice updates are not supported. Create a new invoice instead.".to_string(),
    ))
}

/// DELETE /api/v1/invoices/{id} - Not implemented (invoices are immutable)
pub async fn delete(
    _state: web::Data<AppState>,
    _req: HttpRequest,
    _path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    Err(AppError::BadRequest(
        "Invoice deletion is not supported.".to_string(),
    ))
}

/// POST /api/v1/invoices/{id}/submit - Submit for review (not implemented)
pub async fn submit(
    _state: web::Data<AppState>,
    _req: HttpRequest,
    _path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_message(
        "Invoice already submitted during creation",
    )))
}

/// POST /api/v1/invoices/{id}/documents
pub async fn upload_document(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    mut payload: Multipart,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let invoice_id = path.into_inner();

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

    let file_data =
        file_data.ok_or_else(|| AppError::ValidationError("File is required".to_string()))?;
    let file_name =
        file_name.ok_or_else(|| AppError::ValidationError("Filename is required".to_string()))?;
    let document_type = document_type
        .ok_or_else(|| AppError::ValidationError("Document type is required".to_string()))?;

    let document = state
        .invoice_service
        .upload_document(invoice_id, &document_type, &file_name, file_data)
        .await?;
    let _ = user_id; // Verify user is authenticated

    Ok(HttpResponse::Created().json(ApiResponse::success(
        document,
        "Document uploaded successfully",
    )))
}

/// GET /api/v1/invoices/{id}/documents
pub async fn get_documents(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let invoice_id = path.into_inner();
    let documents = state.invoice_service.get_documents(invoice_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        documents,
        "Documents retrieved successfully",
    )))
}

/// POST /api/v1/invoices/{id}/tokenize - Not implemented yet
pub async fn tokenize(
    _state: web::Data<AppState>,
    _req: HttpRequest,
    _path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    Err(AppError::BadRequest(
        "Tokenization is handled automatically after approval".to_string(),
    ))
}

// ============ Admin Invoice Endpoints ============

/// GET /api/v1/admin/invoices/pending
pub async fn get_pending_invoices(
    state: web::Data<AppState>,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let (invoices, total) = state
        .invoice_service
        .list_pending(query.page.unwrap_or(1), query.per_page.unwrap_or(10))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(
        invoices,
        total,
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    )))
}

/// GET /api/v1/admin/invoices/approved
pub async fn get_approved_invoices(
    state: web::Data<AppState>,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let (invoices, total) = state
        .invoice_service
        .list_approved(query.page.unwrap_or(1), query.per_page.unwrap_or(10))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(
        invoices,
        total,
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    )))
}

/// GET /api/v1/admin/invoices/{id}/grade-suggestion
pub async fn get_grade_suggestion(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let invoice_id = path.into_inner();
    let suggestion = state
        .invoice_service
        .get_grade_suggestion(invoice_id)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        suggestion,
        "Grade suggestion retrieved",
    )))
}

/// GET /api/v1/admin/invoices/{id}/review
pub async fn get_invoice_review_data(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let invoice_id = path.into_inner();
    // Get invoice and grade suggestion
    let invoice = state.invoice_service.get_invoice(invoice_id).await?;
    let suggestion = state
        .invoice_service
        .get_grade_suggestion(invoice_id)
        .await?;
    let documents = state.invoice_service.get_documents(invoice_id).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "invoice": invoice,
            "grade_suggestion": suggestion,
            "documents": documents
        }),
        "Review data retrieved",
    )))
}

/// POST /api/v1/admin/invoices/{id}/approve
pub async fn approve(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<AdminReviewInvoiceRequest>,
) -> AppResult<HttpResponse> {
    let invoice_id = path.into_inner();
    let data = body.into_inner();

    let invoice = state
        .invoice_service
        .approve(
            invoice_id,
            data.grade.as_deref().unwrap_or("B"),
            data.priority_interest_rate,
            data.catalyst_interest_rate,
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(invoice, "Invoice approved")))
}

/// POST /api/v1/admin/invoices/{id}/reject
pub async fn reject(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<RejectRequest>,
) -> AppResult<HttpResponse> {
    let invoice_id = path.into_inner();
    let invoice = state
        .invoice_service
        .reject(invoice_id, &body.reason)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(invoice, "Invoice rejected")))
}

#[derive(serde::Deserialize)]
#[allow(dead_code)] // Fields used for query deserialization
pub struct InvoiceListQuery {
    pub status: Option<String>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
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
