use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;

use super::AppState;
use crate::error::{AppError, AppResult};
use crate::utils::{ApiResponse, Claims};

fn get_user_id(req: &HttpRequest) -> AppResult<Uuid> {
    req.extensions()
        .get::<Claims>()
        .map(|c| c.user_id())
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))
}

/// GET /api/v1/admin/platform/revenue
pub async fn get_platform_revenue(state: web::Data<AppState>) -> AppResult<HttpResponse> {
    let revenue = state.payment_service.get_platform_revenue().await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({ "revenue": revenue, "currency": "IDRX" }),
        "Platform revenue retrieved",
    )))
}
