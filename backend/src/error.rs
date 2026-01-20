use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    // Authentication errors
    Unauthorized(String),
    Forbidden(String),
    InvalidCredentials,
    TokenExpired,
    InvalidToken,

    // Validation errors
    ValidationError(String),
    BadRequest(String),

    // Resource errors
    NotFound(String),
    Conflict(String),

    // Database errors
    DatabaseError(String),

    // External service errors
    BlockchainError(String),
    EmailError(String),
    IpfsError(String),

    // Internal errors
    InternalError(String),

    // Business logic errors
    InsufficientBalance,
    InvoiceNotFundable,
    PoolNotOpen,
    CatalystNotUnlocked,
    InvalidTrancheSelection,
    ProfileNotComplete,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::InvalidCredentials => write!(f, "Invalid credentials"),
            AppError::TokenExpired => write!(f, "Token has expired"),
            AppError::InvalidToken => write!(f, "Invalid token"),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::BlockchainError(msg) => write!(f, "Blockchain error: {}", msg),
            AppError::EmailError(msg) => write!(f, "Email error: {}", msg),
            AppError::IpfsError(msg) => write!(f, "IPFS error: {}", msg),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            AppError::InsufficientBalance => write!(f, "Insufficient balance"),
            AppError::InvoiceNotFundable => write!(f, "Invoice is not fundable"),
            AppError::PoolNotOpen => write!(f, "Pool is not open for investment"),
            AppError::CatalystNotUnlocked => write!(f, "Catalyst tranche not unlocked"),
            AppError::InvalidTrancheSelection => write!(f, "Invalid tranche selection"),
            AppError::ProfileNotComplete => write!(f, "Profile is not complete"),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status, code, message) = match self {
            AppError::Unauthorized(msg) => {
                (actix_web::http::StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone())
            }
            AppError::Forbidden(msg) => {
                (actix_web::http::StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone())
            }
            AppError::InvalidCredentials => (
                actix_web::http::StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                "Invalid email/username or password".to_string(),
            ),
            AppError::TokenExpired => (
                actix_web::http::StatusCode::UNAUTHORIZED,
                "TOKEN_EXPIRED",
                "Token has expired".to_string(),
            ),
            AppError::InvalidToken => (
                actix_web::http::StatusCode::UNAUTHORIZED,
                "INVALID_TOKEN",
                "Invalid token".to_string(),
            ),
            AppError::ValidationError(msg) => {
                (actix_web::http::StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone())
            }
            AppError::BadRequest(msg) => {
                (actix_web::http::StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone())
            }
            AppError::NotFound(msg) => {
                (actix_web::http::StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone())
            }
            AppError::Conflict(msg) => {
                (actix_web::http::StatusCode::CONFLICT, "CONFLICT", msg.clone())
            }
            AppError::DatabaseError(msg) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                msg.clone(),
            ),
            AppError::BlockchainError(msg) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "BLOCKCHAIN_ERROR",
                msg.clone(),
            ),
            AppError::EmailError(msg) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "EMAIL_ERROR",
                msg.clone(),
            ),
            AppError::IpfsError(msg) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "IPFS_ERROR",
                msg.clone(),
            ),
            AppError::InternalError(msg) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
            AppError::InsufficientBalance => (
                actix_web::http::StatusCode::BAD_REQUEST,
                "INSUFFICIENT_BALANCE",
                "Insufficient balance for this operation".to_string(),
            ),
            AppError::InvoiceNotFundable => (
                actix_web::http::StatusCode::BAD_REQUEST,
                "INVOICE_NOT_FUNDABLE",
                "Invoice is not in a fundable state".to_string(),
            ),
            AppError::PoolNotOpen => (
                actix_web::http::StatusCode::BAD_REQUEST,
                "POOL_NOT_OPEN",
                "Pool is not open for investment".to_string(),
            ),
            AppError::CatalystNotUnlocked => (
                actix_web::http::StatusCode::FORBIDDEN,
                "CATALYST_NOT_UNLOCKED",
                "Complete the risk questionnaire to unlock Catalyst tranche".to_string(),
            ),
            AppError::InvalidTrancheSelection => (
                actix_web::http::StatusCode::BAD_REQUEST,
                "INVALID_TRANCHE",
                "Invalid tranche selection".to_string(),
            ),
            AppError::ProfileNotComplete => (
                actix_web::http::StatusCode::FORBIDDEN,
                "PROFILE_NOT_COMPLETE",
                "Please complete your profile first".to_string(),
            ),
        };

        HttpResponse::build(status).json(json!({
            "success": false,
            "error": {
                "code": code,
                "message": message
            }
        }))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {:?}", err);
        AppError::DatabaseError(err.to_string())
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        tracing::error!("Bcrypt error: {:?}", err);
        AppError::InternalError("Password hashing failed".to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
            _ => AppError::InvalidToken,
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        tracing::error!("HTTP request error: {:?}", err);
        AppError::InternalError(err.to_string())
    }
}

impl From<lettre::transport::smtp::Error> for AppError {
    fn from(err: lettre::transport::smtp::Error) -> Self {
        tracing::error!("SMTP error: {:?}", err);
        AppError::EmailError(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Anyhow error: {:?}", err);
        AppError::InternalError(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
