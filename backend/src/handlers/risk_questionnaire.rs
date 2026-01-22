use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;

use super::AppState;
use crate::error::{AppError, AppResult};
use crate::models::SubmitRiskQuestionnaireRequest;
use crate::utils::{ApiResponse, Claims};

fn get_user_id(req: &HttpRequest) -> AppResult<Uuid> {
    req.extensions()
        .get::<Claims>()
        .map(|c| c.user_id())
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))
}

/// GET /api/v1/risk-questionnaire/questions
pub async fn get_questions(state: web::Data<AppState>) -> AppResult<HttpResponse> {
    let questions = state.rq_service.get_questions();
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        questions,
        "Questions retrieved successfully",
    )))
}

/// POST /api/v1/risk-questionnaire
pub async fn submit(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<SubmitRiskQuestionnaireRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let result = state.rq_service.submit(user_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        result,
        "Risk questionnaire submitted successfully",
    )))
}

/// GET /api/v1/risk-questionnaire/status
pub async fn get_status(state: web::Data<AppState>, req: HttpRequest) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let status = state.rq_service.get_status(user_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        status,
        "Risk questionnaire status retrieved successfully",
    )))
}
