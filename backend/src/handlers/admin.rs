use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{AdminReviewInvoiceRequest, AdminKycReviewRequest, AdminGrantBalanceRequest};
use crate::services::{InvoiceService, MitraService, UserService, PaymentService, FundingService};
use crate::utils::{ApiResponse, Claims};

pub struct AdminHandler {
    invoice_service: Arc<InvoiceService>,
    mitra_service: Arc<MitraService>,
    user_service: Arc<UserService>,
    payment_service: Arc<PaymentService>,
    funding_service: Arc<FundingService>,
}

impl AdminHandler {
    pub fn new(
        invoice_service: Arc<InvoiceService>,
        mitra_service: Arc<MitraService>,
        user_service: Arc<UserService>,
        payment_service: Arc<PaymentService>,
        funding_service: Arc<FundingService>,
    ) -> Self {
        Self {
            invoice_service,
            mitra_service,
            user_service,
            payment_service,
            funding_service,
        }
    }
}

fn get_admin_id_from_request(req: &HttpRequest) -> AppResult<Uuid> {
    let claims = req.extensions()
        .get::<Claims>()
        .cloned()
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    if claims.role != "admin" {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    Ok(claims.user_id)
}

// ============ Invoice Management ============

/// GET /api/v1/admin/invoices/pending
pub async fn get_pending_invoices(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let (invoices, total) = handler.invoice_service.get_pending_invoices(
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    ).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(invoices, total, query.page.unwrap_or(1), query.per_page.unwrap_or(10))))
}

/// GET /api/v1/admin/invoices/{id}
pub async fn get_invoice(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let invoice_id = path.into_inner();
    let invoice = handler.invoice_service.get_by_id(invoice_id, None).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(invoice, "Invoice retrieved successfully")))
}

/// POST /api/v1/admin/invoices/{id}/review
pub async fn review_invoice(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<AdminReviewInvoiceRequest>,
) -> AppResult<HttpResponse> {
    let admin_id = get_admin_id_from_request(&req)?;
    let invoice_id = path.into_inner();
    let invoice = handler.invoice_service.admin_review(
        admin_id,
        invoice_id,
        body.into_inner(),
    ).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(invoice, "Invoice reviewed successfully")))
}

// ============ Mitra Management ============

/// GET /api/v1/admin/mitra/applications
pub async fn get_mitra_applications(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let (applications, total) = handler.mitra_service.get_pending_applications(
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    ).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(applications, total, query.page.unwrap_or(1), query.per_page.unwrap_or(10))))
}

/// GET /api/v1/admin/mitra/applications/{id}
pub async fn get_mitra_application(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let application_id = path.into_inner();
    let application = handler.mitra_service.get_application(application_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(application, "Application retrieved successfully")))
}

/// POST /api/v1/admin/mitra/applications/{id}/approve
pub async fn approve_mitra(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let admin_id = get_admin_id_from_request(&req)?;
    let application_id = path.into_inner();
    let application = handler.mitra_service.approve(application_id, admin_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(application, "Mitra application approved")))
}

/// POST /api/v1/admin/mitra/applications/{id}/reject
pub async fn reject_mitra(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<RejectRequest>,
) -> AppResult<HttpResponse> {
    let admin_id = get_admin_id_from_request(&req)?;
    let application_id = path.into_inner();
    let application = handler.mitra_service.reject(application_id, admin_id, &body.reason).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(application, "Mitra application rejected")))
}

// ============ KYC Management ============

/// GET /api/v1/admin/kyc/pending
pub async fn get_pending_kyc(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let (kyc_list, total) = handler.user_service.get_pending_kyc(
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    ).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(kyc_list, total, query.page.unwrap_or(1), query.per_page.unwrap_or(10))))
}

/// POST /api/v1/admin/kyc/{id}/review
pub async fn review_kyc(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<AdminKycReviewRequest>,
) -> AppResult<HttpResponse> {
    let admin_id = get_admin_id_from_request(&req)?;
    let kyc_id = path.into_inner();
    let kyc = handler.user_service.admin_review_kyc(admin_id, kyc_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(kyc, "KYC reviewed successfully")))
}

// ============ User Management ============

/// GET /api/v1/admin/users
pub async fn list_users(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    query: web::Query<UserListQuery>,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let (users, total) = handler.user_service.list_users(
        query.role.as_deref(),
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    ).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(users, total, query.page.unwrap_or(1), query.per_page.unwrap_or(10))))
}

/// GET /api/v1/admin/users/{id}
pub async fn get_user(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let user_id = path.into_inner();
    let user = handler.user_service.get_user_details(user_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(user, "User retrieved successfully")))
}

/// POST /api/v1/admin/users/{id}/grant-balance
pub async fn grant_balance(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<AdminGrantBalanceRequest>,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let user_id = path.into_inner();
    let balance = handler.payment_service.admin_grant_balance(user_id, body.amount, &body.reason).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(balance, "Balance granted successfully")))
}

// ============ Platform Stats ============

/// GET /api/v1/admin/stats
pub async fn get_platform_stats(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let stats = handler.funding_service.get_platform_stats().await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(stats, "Platform stats retrieved successfully")))
}

/// GET /api/v1/admin/revenue
pub async fn get_platform_revenue(
    handler: web::Data<AdminHandler>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let _admin_id = get_admin_id_from_request(&req)?;
    let revenue = handler.payment_service.get_platform_revenue().await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({ "revenue": revenue }), "Revenue retrieved successfully")))
}

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(serde::Deserialize)]
pub struct UserListQuery {
    pub role: Option<String>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(serde::Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            // Invoice management
            .route("/invoices/pending", web::get().to(get_pending_invoices))
            .route("/invoices/{id}", web::get().to(get_invoice))
            .route("/invoices/{id}/review", web::post().to(review_invoice))
            // Mitra management
            .route("/mitra/applications", web::get().to(get_mitra_applications))
            .route("/mitra/applications/{id}", web::get().to(get_mitra_application))
            .route("/mitra/applications/{id}/approve", web::post().to(approve_mitra))
            .route("/mitra/applications/{id}/reject", web::post().to(reject_mitra))
            // KYC management
            .route("/kyc/pending", web::get().to(get_pending_kyc))
            .route("/kyc/{id}/review", web::post().to(review_kyc))
            // User management
            .route("/users", web::get().to(list_users))
            .route("/users/{id}", web::get().to(get_user))
            .route("/users/{id}/grant-balance", web::post().to(grant_balance))
            // Platform stats
            .route("/stats", web::get().to(get_platform_stats))
            .route("/revenue", web::get().to(get_platform_revenue))
    );
}
