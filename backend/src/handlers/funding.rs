use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;

use super::AppState;
use crate::error::{AppError, AppResult};
use crate::models::InvestRequest;
use crate::utils::{ApiResponse, Claims};

fn get_user_id(req: &HttpRequest) -> AppResult<Uuid> {
    req.extensions()
        .get::<Claims>()
        .map(|c| c.user_id())
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))
}

/// POST /api/v1/invoices/{id}/pool
pub async fn create_pool(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let invoice_id = path.into_inner();
    let pool = state.funding_service.create_pool(invoice_id).await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(
        pool,
        "Funding pool created successfully",
    )))
}

/// GET /api/v1/pools
pub async fn list_pools(
    state: web::Data<AppState>,
    query: web::Query<PoolListQuery>,
) -> AppResult<HttpResponse> {
    let (pools, total) = state
        .funding_service
        .list_pools(query.page.unwrap_or(1), query.per_page.unwrap_or(10))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(
        pools,
        total,
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    )))
}

/// GET /api/v1/pools/{id}
pub async fn get_pool(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let pool_id = path.into_inner();
    let pool = state.funding_service.get_pool(pool_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(pool, "Pool retrieved successfully")))
}

/// GET /api/v1/marketplace - uses list_pools for now
pub async fn get_marketplace(
    state: web::Data<AppState>,
    query: web::Query<MarketplaceQuery>,
) -> AppResult<HttpResponse> {
    let (pools, total) = state
        .funding_service
        .list_pools(query.page.unwrap_or(1), query.per_page.unwrap_or(10))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(
        pools,
        total,
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    )))
}

/// GET /api/v1/marketplace/{id}/detail
pub async fn get_pool_detail(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let pool_id = path.into_inner();
    let detail = state.funding_service.get_pool(pool_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(detail, "Pool detail retrieved")))
}

/// POST /api/v1/marketplace/calculate
pub async fn calculate_investment(
    _state: web::Data<AppState>,
    body: web::Json<CalculateInvestmentRequest>,
) -> AppResult<HttpResponse> {
    // Simple calculation stub
    let data = body.into_inner();
    let interest_rate = if data.tranche == "priority" {
        0.08
    } else {
        0.12
    };
    let expected_return = data.amount * interest_rate;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "principal": data.amount,
            "interest_rate": interest_rate,
            "expected_return": expected_return,
            "total_return": data.amount + expected_return,
            "tranche": data.tranche,
            "tenor_days": 30
        }),
        "Investment calculated",
    )))
}

/// POST /api/v1/investments
pub async fn invest(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<InvestRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let investment = state
        .funding_service
        .invest(user_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(investment, "Investment initiated")))
}

/// POST /api/v1/investments/confirm
pub async fn confirm_investment(
    _state: web::Data<AppState>,
    _req: HttpRequest,
    body: web::Json<ConfirmInvestmentRequest>,
) -> AppResult<HttpResponse> {
    // Stub - investment confirmation handled in invest
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({ "investment_id": body.investment_id }),
        "Investment confirmed",
    )))
}

/// GET /api/v1/investments
pub async fn get_my_investments(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10);

    let (investments, total) = state
        .funding_service
        .get_investor_investments(user_id, page, per_page)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        crate::models::ActiveInvestmentListResponse {
            investments,
            total,
            page,
            per_page,
            total_pages: (total as f64 / per_page as f64).ceil() as i32,
        },
        "Investments retrieved",
    )))
}

/// GET /api/v1/investments/portfolio
pub async fn get_portfolio(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let portfolio = state
        .funding_service
        .get_investor_portfolio(user_id)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(portfolio, "Portfolio retrieved")))
}

/// GET /api/v1/investments/active
pub async fn get_active_investments(
    _state: web::Data<AppState>,
    _req: HttpRequest,
) -> AppResult<HttpResponse> {
    // Stub - return empty list
    let empty: Vec<serde_json::Value> = vec![];
    Ok(HttpResponse::Ok().json(ApiResponse::success(empty, "Active investments retrieved")))
}

/// POST /api/v1/exporter/disbursement
pub async fn exporter_disbursement(
    _state: web::Data<AppState>,
    _req: HttpRequest,
    _body: web::Json<ExporterDisbursementRequest>,
) -> AppResult<HttpResponse> {
    // Stub
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_message(
        "Disbursement request received",
    )))
}

/// GET /api/v1/mitra/dashboard
pub async fn get_mitra_dashboard(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let dashboard = state.funding_service.get_mitra_dashboard(user_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(dashboard, "Dashboard retrieved")))
}

/// GET /api/v1/mitra/invoices
pub async fn get_mitra_active_invoices(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10);

    // Get all invoices for this mitra (can filter by status via query param if needed)
    let (invoices, total) = state
        .invoice_repo
        .find_by_exporter(user_id, None, page, per_page)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::paginated(invoices, total, page, per_page)))
}

/// GET /api/v1/mitra/pools - List all pools owned by mitra with full details
pub async fn get_mitra_pools(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10);

    let (pools, total) = state
        .funding_service
        .get_mitra_pools(user_id, page, per_page)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::paginated(pools, total, page, per_page)))
}

/// GET /api/v1/mitra/invoices/{id}/pool - Get pool detail for a specific invoice
pub async fn get_pool_by_invoice(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let invoice_id = path.into_inner();

    let pool = state
        .funding_service
        .get_pool_by_invoice(user_id, invoice_id)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(pool, "Pool detail retrieved")))
}

// ============ Admin Funding Endpoints ============

/// POST /api/v1/admin/pools/{id}/disburse
pub async fn disburse(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let pool_id = path.into_inner();
    let _admin_id = get_user_id(&req)?; // Ensure authenticated (Role check usually in middleware)

    let pool = state.funding_service.disburse_pool(pool_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(pool, "Disbursement initiated successfully")))
}

/// POST /api/v1/admin/pools/{id}/close
pub async fn close_pool_and_notify(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let pool_id = path.into_inner();
    let pool = state.funding_service.close_pool(pool_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(pool, "Pool closed successfully")))
}

/// GET /api/v1/admin/users/{id}/pools
pub async fn get_exporter_pools(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<PaginationQuery>,
) -> AppResult<HttpResponse> {
    let user_id = path.into_inner();
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10);

    let (pools, total) = state
        .funding_service
        .get_mitra_pools(user_id, page, per_page)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::paginated(pools, total, page, per_page)))
}

/// POST /api/v1/admin/invoices/{id}/repay (Used by Mitra/Admin)
pub async fn process_repayment(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<crate::models::RepayInvoiceRequest>,
) -> AppResult<HttpResponse> {
    let invoice_id = path.into_inner();
    let user_id = get_user_id(&req)?;

    // We allow Mitra (exporter) to initiate repayment
    // Logic inside service should verify ownership
    let result = state
        .funding_service
        .repay_invoice(user_id, invoice_id, body.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        result,
        "Repayment processed successfully",
    )))
}

#[derive(serde::Deserialize)]
#[allow(dead_code)] // Fields used for query deserialization
pub struct PoolListQuery {
    pub status: Option<String>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(serde::Deserialize)]
pub struct MarketplaceQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)] // Fields used for query deserialization
pub struct PaginationQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)] // Fields used for request deserialization
pub struct CalculateInvestmentRequest {
    pub pool_id: Uuid,
    pub amount: f64,
    pub tranche: String,
}

#[derive(serde::Deserialize)]
pub struct ConfirmInvestmentRequest {
    pub investment_id: Uuid,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)] // Fields used for request deserialization
pub struct ExporterDisbursementRequest {
    pub pool_id: Uuid,
    // Bank account removed
}
