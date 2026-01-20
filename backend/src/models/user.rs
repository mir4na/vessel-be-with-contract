use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum UserRole {
    #[serde(rename = "investor")]
    Investor,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "mitra")]
    Mitra,
    #[serde(rename = "exporter")]
    Exporter,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::Investor
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum MemberStatus {
    #[serde(rename = "calon_anggota_pendana")]
    CalonAnggotaPendana,
    #[serde(rename = "calon_anggota_mitra")]
    CalonAnggotaMitra,
    #[serde(rename = "member_mitra")]
    MemberMitra,
    #[serde(rename = "admin")]
    Admin,
}

impl Default for MemberStatus {
    fn default() -> Self {
        MemberStatus::CalonAnggotaPendana
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
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
    pub balance_idr: rust_decimal::Decimal,
    pub email_verified: bool,
    pub profile_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub role: String,
    pub cooperative_agreement: bool,
    pub otp_token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CompleteProfileRequest {
    #[validate(length(min = 3, message = "Full name must be at least 3 characters"))]
    pub full_name: String,
    pub phone: Option<String>,
    pub nik: Option<String>,
    pub ktp_photo_url: Option<String>,
    pub selfie_url: Option<String>,
    pub bank_code: Option<String>,
    pub account_number: Option<String>,
    pub account_name: Option<String>,
    pub company_name: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    pub email_or_username: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserBalanceResponse {
    pub balance_idrx: f64,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: User,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
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

// Bank Account Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedBank {
    pub code: String,
    pub name: String,
}

pub fn get_supported_banks() -> Vec<SupportedBank> {
    vec![
        SupportedBank { code: "bca".to_string(), name: "Bank Central Asia (BCA)".to_string() },
        SupportedBank { code: "mandiri".to_string(), name: "Bank Mandiri".to_string() },
        SupportedBank { code: "bni".to_string(), name: "Bank Negara Indonesia (BNI)".to_string() },
        SupportedBank { code: "bri".to_string(), name: "Bank Rakyat Indonesia (BRI)".to_string() },
        SupportedBank { code: "cimb".to_string(), name: "CIMB Niaga".to_string() },
        SupportedBank { code: "danamon".to_string(), name: "Bank Danamon".to_string() },
        SupportedBank { code: "permata".to_string(), name: "Bank Permata".to_string() },
        SupportedBank { code: "bsi".to_string(), name: "Bank Syariah Indonesia (BSI)".to_string() },
        SupportedBank { code: "btn".to_string(), name: "Bank Tabungan Negara (BTN)".to_string() },
        SupportedBank { code: "ocbc".to_string(), name: "OCBC NISP".to_string() },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BankAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bank_code: String,
    pub bank_name: String,
    pub account_number: String,
    pub account_name: String,
    pub is_verified: bool,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
}

pub const BANK_ACCOUNT_MICROCOPY: &str = "Rekening ini akan menjadi satu-satunya tujuan pencairan dana demi keamanan. Kamu bisa mengubahnya nanti di bagian profile.";

#[derive(Debug, Deserialize, Validate)]
pub struct ChangeBankAccountRequest {
    pub otp_token: String,
    pub bank_code: String,
    pub account_number: String,
    pub account_name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 8, message = "Current password must be at least 8 characters"))]
    pub current_password: String,
    #[validate(length(min = 8, message = "New password must be at least 8 characters"))]
    pub new_password: String,
    pub confirm_password: String,
}

// KYC/Identity Models
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserIdentity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub nik: String,
    pub full_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ktp_photo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selfie_url: Option<String>,
    pub is_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserIdentity {
    pub fn mask_nik(&self) -> String {
        if self.nik.len() != 16 {
            return "****".to_string();
        }
        format!("{}******{}", &self.nik[..6], &self.nik[12..])
    }
}

#[derive(Debug, Serialize)]
pub struct ProfileDataResponse {
    pub full_name: String,
    pub nik_masked: String,
    pub email: String,
    pub phone: String,
    pub username: String,
    pub member_status: String,
    pub role: String,
    pub is_verified: bool,
    pub joined_at: String,
}

#[derive(Debug, Serialize)]
pub struct BankAccountResponse {
    pub bank_code: String,
    pub bank_name: String,
    pub account_number: String,
    pub account_name: String,
    pub is_primary: bool,
    pub is_verified: bool,
    pub microcopy: String,
}

pub fn mask_account_number(number: &str) -> String {
    if number.len() <= 4 {
        return number.to_string();
    }
    format!("****{}", &number[number.len() - 4..])
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

// Admin request types
#[derive(Debug, Deserialize)]
pub struct AdminGrantBalanceRequest {
    pub user_id: uuid::Uuid,
    pub amount: f64,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminKycReviewRequest {
    pub action: String, // "approve" or "reject"
    pub rejection_reason: Option<String>,
    pub notes: Option<String>,
}
