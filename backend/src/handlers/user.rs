use actix_multipart::Multipart;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use uuid::Uuid;

use super::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{
    ChangePasswordRequest, CompleteProfileRequest, ConnectWalletRequest, UpdateProfileRequest,
};
use crate::utils::{hash_password, verify_password, ApiResponse, Claims};

fn get_user_id(req: &HttpRequest) -> AppResult<Uuid> {
    req.extensions()
        .get::<Claims>()
        .map(|c| c.user_id())
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))
}

/// GET /api/v1/user/profile
pub async fn get_profile(state: web::Data<AppState>, req: HttpRequest) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let user = state
        .user_repo
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(user, "Profile retrieved successfully")))
}

/// PUT /api/v1/user/profile
pub async fn update_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<UpdateProfileRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let data = body.into_inner();
    let profile = state
        .user_repo
        .update_profile(
            user_id,
            data.full_name.as_deref(),
            data.phone.as_deref(),
            data.country.as_deref(),
            data.company_name.as_deref(),
            data.company_type.as_deref(),
            data.business_sector.as_deref(),
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        profile,
        "Profile updated successfully",
    )))
}

/// POST /api/v1/user/complete-profile
pub async fn complete_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CompleteProfileRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let data = body.into_inner();

    // Create or update profile
    let profile = state
        .user_repo
        .create_profile(user_id, &data.full_name)
        .await?;

    // Mark profile as completed
    state.user_repo.set_profile_completed(user_id, true).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        profile,
        "Profile completed successfully",
    )))
}

/// POST /api/v1/user/documents
pub async fn upload_document(
    state: web::Data<AppState>,
    req: HttpRequest,
    mut payload: Multipart,
) -> AppResult<HttpResponse> {
    let _user_id = get_user_id(&req)?;

    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut _document_type: Option<String> = None;

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
                _document_type = Some(String::from_utf8_lossy(&data).to_string());
            }
            _ => {}
        }
    }

    let file_data =
        file_data.ok_or_else(|| AppError::ValidationError("File is required".to_string()))?;
    let file_name =
        file_name.ok_or_else(|| AppError::ValidationError("Filename is required".to_string()))?;

    // Upload to IPFS
    let url = state
        .pinata_service
        .upload_file(file_data, &file_name)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(
        serde_json::json!({ "url": url }),
        "Document uploaded successfully",
    )))
}

/// GET /api/v1/user/profile/data
pub async fn get_personal_data(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let user = state
        .user_repo
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let profile = state.user_repo.find_profile_by_user_id(user_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "user": user,
            "profile": profile
        }),
        "Personal data retrieved",
    )))
}

/// PUT /api/v1/user/profile/password
pub async fn change_password(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ChangePasswordRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let data = body.into_inner();

    // Verify confirm password matches
    if data.new_password != data.confirm_password {
        return Err(AppError::ValidationError(
            "Passwords do not match".to_string(),
        ));
    }

    // Get user
    let user = state
        .user_repo
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Verify current password
    if !verify_password(&data.current_password, &user.password_hash)? {
        return Err(AppError::Unauthorized(
            "Current password is incorrect".to_string(),
        ));
    }

    // Hash new password and update
    let new_hash = hash_password(&data.new_password)?;
    state.user_repo.update_password(user_id, &new_hash).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_message(
        "Password changed successfully",
    )))
}

/// PUT /api/v1/user/wallet
/// Connect wallet with signature verification (supports Base Smart Wallet / passkey via ERC-1271)
/// Flow: 1) POST /auth/wallet/nonce → 2) Sign message with wallet → 3) PUT /user/wallet
pub async fn connect_wallet(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ConnectWalletRequest>,
) -> AppResult<HttpResponse> {
    let user_id = get_user_id(&req)?;
    let user = state
        .auth_service
        .connect_wallet(user_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(user, "Wallet connected successfully")))
}

/// GET /api/v1/admin/users
pub async fn list_users(
    state: web::Data<AppState>,
    query: web::Query<UserListQuery>,
) -> AppResult<HttpResponse> {
    let (users, total) = state
        .user_repo
        .list_users(
            query.role.as_deref(),
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(10),
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::paginated(
        users,
        total,
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(10),
    )))
}

#[derive(serde::Deserialize)]
pub struct UserListQuery {
    pub role: Option<String>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}
