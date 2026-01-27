use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type, Default)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum UserRole {
    #[serde(rename = "investor")]
    #[default]
    Investor,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "mitra")]
    Mitra,
    #[serde(rename = "exporter")]
    Exporter,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Investor => write!(f, "investor"),
            UserRole::Admin => write!(f, "admin"),
            UserRole::Mitra => write!(f, "mitra"),
            UserRole::Exporter => write!(f, "exporter"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type, Default)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum MemberStatus {
    #[serde(rename = "calon_anggota_pendana")]
    #[default]
    CalonAnggotaPendana,
    #[serde(rename = "calon_anggota_mitra")]
    CalonAnggotaMitra,
    #[serde(rename = "member_mitra")]
    MemberMitra,
    #[serde(rename = "admin")]
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub is_verified: bool,
    pub is_active: bool,
    pub cooperative_agreement: bool,
    pub member_status: String,

    pub email_verified: bool,
    pub profile_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<UserProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub full_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_sector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 3, max = 50, message = "Username must be 3-50 characters"))]
    pub username: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    pub confirm_password: String,
    pub otp_token: String,
    pub cooperative_agreement: bool,
    // Mitra application fields
    pub company_name: Option<String>,
    pub company_type: Option<String>,
    pub npwp: Option<String>,
    pub annual_revenue: Option<String>,
    pub address: Option<String>,
    pub business_description: Option<String>,
    pub website_url: Option<String>,
    pub year_founded: Option<i32>,
    pub key_products: Option<String>,
    pub export_markets: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CompleteProfileRequest {
    #[validate(length(min = 3, message = "Full name must be at least 3 characters"))]
    pub full_name: String,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub company_name: Option<String>,
    pub company_type: Option<String>,
    pub business_sector: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    pub email_or_username: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWalletRequest {
    pub wallet_address: String,
}

// Wallet-based authentication for investors
#[derive(Debug, Deserialize, Validate)]
pub struct WalletLoginRequest {
    #[validate(length(min = 42, max = 42, message = "Invalid wallet address"))]
    pub wallet_address: String,
    pub signature: String,
    pub message: String,
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct WalletNonceResponse {
    pub nonce: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GetNonceRequest {
    pub wallet_address: String,
}

// Simplified investor registration (wallet-only)
#[derive(Debug, Deserialize, Validate)]
pub struct InvestorWalletRegisterRequest {
    #[validate(length(min = 42, max = 42, message = "Invalid wallet address"))]
    pub wallet_address: String,
    pub signature: String,
    pub message: String,
    pub nonce: String,
    pub cooperative_agreement: bool,
}

// Connect wallet with signature verification (for any authenticated user - investor or mitra)
// Supports Base Smart Wallet (passkey) via ERC-1271
#[derive(Debug, Deserialize, Validate)]
pub struct ConnectWalletRequest {
    #[validate(length(min = 42, max = 42, message = "Invalid wallet address"))]
    pub wallet_address: String,
    pub signature: String,
    pub message: String,
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: User,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

// Google OAuth
#[derive(Debug, Deserialize)]
pub struct GoogleAuthRequest {
    pub id_token: String,
}

#[derive(Debug, Serialize)]
pub struct GoogleAuthResponse {
    pub email: String,
    pub otp_token: String,
    pub expires_in_minutes: i64,
}
