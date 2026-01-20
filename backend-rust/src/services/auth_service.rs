use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;
use ethers::types::{Address, Signature};
use ethers::utils::hash_message;

use crate::error::{AppError, AppResult};
use crate::models::{
    User, RegisterRequest, LoginRequest, LoginResponse,
    WalletLoginRequest, WalletNonceResponse, InvestorWalletRegisterRequest,
};
use crate::repository::UserRepository;
use crate::utils::{JwtManager, hash_password, verify_password, generate_random_token};

use super::OtpService;

pub struct AuthService {
    user_repo: Arc<UserRepository>,
    jwt_manager: Arc<JwtManager>,
    otp_service: Arc<OtpService>,
    // Store nonces for wallet authentication (in production, use Redis)
    wallet_nonces: Arc<RwLock<HashMap<String, String>>>,
}

impl AuthService {
    pub fn new(
        user_repo: Arc<UserRepository>,
        jwt_manager: Arc<JwtManager>,
        otp_service: Arc<OtpService>,
    ) -> Self {
        Self {
            user_repo,
            jwt_manager,
            otp_service,
            wallet_nonces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate nonce for wallet authentication
    pub async fn get_wallet_nonce(&self, wallet_address: &str) -> AppResult<WalletNonceResponse> {
        let wallet = wallet_address.to_lowercase();
        let nonce = generate_random_token();
        let message = format!(
            "Welcome to VESSEL!\n\nPlease sign this message to verify your wallet ownership.\n\nWallet: {}\nNonce: {}",
            wallet, nonce
        );

        // Store nonce
        {
            let mut nonces = self.wallet_nonces.write().await;
            nonces.insert(wallet, nonce.clone());
        }

        Ok(WalletNonceResponse { nonce, message })
    }

    /// Verify wallet signature
    fn verify_wallet_signature(&self, wallet_address: &str, signature: &str, message: &str) -> AppResult<bool> {
        // Parse wallet address
        let address: Address = wallet_address.parse()
            .map_err(|_| AppError::ValidationError("Invalid wallet address".to_string()))?;

        // Parse signature
        let sig: Signature = signature.parse()
            .map_err(|_| AppError::ValidationError("Invalid signature format".to_string()))?;

        // Hash the message (EIP-191 personal sign)
        let message_hash = hash_message(message);

        // Recover the signer address
        let recovered = sig.recover(message_hash)
            .map_err(|_| AppError::ValidationError("Failed to recover signer".to_string()))?;

        Ok(recovered == address)
    }

    /// Wallet login for investors
    pub async fn wallet_login(&self, req: WalletLoginRequest) -> AppResult<LoginResponse> {
        let wallet = req.wallet_address.to_lowercase();

        // Verify nonce
        {
            let nonces = self.wallet_nonces.read().await;
            let stored_nonce = nonces.get(&wallet)
                .ok_or_else(|| AppError::ValidationError("Invalid or expired nonce".to_string()))?;

            if stored_nonce != &req.nonce {
                return Err(AppError::ValidationError("Nonce mismatch".to_string()));
            }
        }

        // Verify signature
        if !self.verify_wallet_signature(&wallet, &req.signature, &req.message)? {
            return Err(AppError::InvalidCredentials);
        }

        // Clear used nonce
        {
            let mut nonces = self.wallet_nonces.write().await;
            nonces.remove(&wallet);
        }

        // Find or create user by wallet
        let user = match self.user_repo.find_by_wallet(&wallet).await? {
            Some(user) => {
                // Existing user - verify they are an investor
                if user.role != "investor" {
                    return Err(AppError::Forbidden(
                        "Wallet login is only available for investors. Please use email/password login.".to_string()
                    ));
                }
                user
            }
            None => {
                // Auto-create investor account with wallet
                self.user_repo.create_investor_with_wallet(&wallet).await?
            }
        };

        if !user.is_active {
            return Err(AppError::Forbidden("Account is deactivated".to_string()));
        }

        // Generate tokens
        let access_token = self.jwt_manager.generate_access_token(user.id, &user.email, &user.role)?;
        let refresh_token = self.jwt_manager.generate_refresh_token(user.id, &user.email, &user.role)?;

        Ok(LoginResponse {
            user,
            access_token,
            refresh_token,
            expires_in: self.jwt_manager.get_expiry_hours() * 3600,
        })
    }

    /// Register investor with wallet only
    pub async fn register_investor_wallet(&self, req: InvestorWalletRegisterRequest) -> AppResult<LoginResponse> {
        let wallet = req.wallet_address.to_lowercase();

        // Verify cooperative agreement
        if !req.cooperative_agreement {
            return Err(AppError::ValidationError("Must accept cooperative agreement".to_string()));
        }

        // Verify nonce
        {
            let nonces = self.wallet_nonces.read().await;
            let stored_nonce = nonces.get(&wallet)
                .ok_or_else(|| AppError::ValidationError("Invalid or expired nonce".to_string()))?;

            if stored_nonce != &req.nonce {
                return Err(AppError::ValidationError("Nonce mismatch".to_string()));
            }
        }

        // Verify signature
        if !self.verify_wallet_signature(&wallet, &req.signature, &req.message)? {
            return Err(AppError::InvalidCredentials);
        }

        // Clear used nonce
        {
            let mut nonces = self.wallet_nonces.write().await;
            nonces.remove(&wallet);
        }

        // Check if wallet already registered
        if self.user_repo.find_by_wallet(&wallet).await?.is_some() {
            return Err(AppError::Conflict("Wallet already registered".to_string()));
        }

        // Create investor with wallet
        let user = self.user_repo.create_investor_with_wallet(&wallet).await?;

        // Generate tokens
        let access_token = self.jwt_manager.generate_access_token(user.id, &user.email, &user.role)?;
        let refresh_token = self.jwt_manager.generate_refresh_token(user.id, &user.email, &user.role)?;

        Ok(LoginResponse {
            user,
            access_token,
            refresh_token,
            expires_in: self.jwt_manager.get_expiry_hours() * 3600,
        })
    }

    /// Traditional registration (for mitra/admin only)
    pub async fn register(&self, req: RegisterRequest) -> AppResult<LoginResponse> {
        // Validate role - only mitra can register with email/password
        if req.role == "investor" {
            return Err(AppError::ValidationError(
                "Investors must use wallet login. Please connect your wallet instead.".to_string()
            ));
        }

        // Verify OTP token
        let email = self.otp_service.verify_otp_token(&req.otp_token, "registration")?;

        // Check if email matches
        if email.to_lowercase() != req.email.to_lowercase() {
            return Err(AppError::ValidationError("OTP token does not match email".to_string()));
        }

        // Check if email already exists
        if self.user_repo.find_by_email(&req.email).await?.is_some() {
            return Err(AppError::Conflict("Email already registered".to_string()));
        }

        // Check if username already exists
        if self.user_repo.find_by_username(&req.username).await?.is_some() {
            return Err(AppError::Conflict("Username already taken".to_string()));
        }

        // Validate password confirmation
        if req.password != req.confirm_password {
            return Err(AppError::ValidationError("Passwords do not match".to_string()));
        }

        // Validate cooperative agreement
        if !req.cooperative_agreement {
            return Err(AppError::ValidationError("Must accept cooperative agreement".to_string()));
        }

        // Hash password
        let password_hash = hash_password(&req.password)?;

        // Create user
        let user = self.user_repo
            .create(&req.email, &req.username, &password_hash, &req.role)
            .await?;

        // Generate tokens
        let access_token = self.jwt_manager.generate_access_token(user.id, &user.email, &user.role)?;
        let refresh_token = self.jwt_manager.generate_refresh_token(user.id, &user.email, &user.role)?;

        Ok(LoginResponse {
            user,
            access_token,
            refresh_token,
            expires_in: self.jwt_manager.get_expiry_hours() * 3600,
        })
    }

    /// Traditional login (for mitra/admin only)
    pub async fn login(&self, req: LoginRequest) -> AppResult<LoginResponse> {
        // Find user by email or username
        let user = self.user_repo
            .find_by_email_or_username(&req.email_or_username)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        // Investors must use wallet login
        if user.role == "investor" {
            return Err(AppError::Forbidden(
                "Investors must use wallet login. Please connect your wallet instead.".to_string()
            ));
        }

        // Check if user is active
        if !user.is_active {
            return Err(AppError::Forbidden("Account is deactivated".to_string()));
        }

        // Verify password
        if !verify_password(&req.password, &user.password_hash)? {
            return Err(AppError::InvalidCredentials);
        }

        // Generate tokens
        let access_token = self.jwt_manager.generate_access_token(user.id, &user.email, &user.role)?;
        let refresh_token = self.jwt_manager.generate_refresh_token(user.id, &user.email, &user.role)?;

        Ok(LoginResponse {
            user,
            access_token,
            refresh_token,
            expires_in: self.jwt_manager.get_expiry_hours() * 3600,
        })
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> AppResult<(String, String)> {
        // Verify refresh token
        let claims = self.jwt_manager.verify_refresh_token(refresh_token)?;

        // Parse user ID
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::InvalidToken)?;

        // Get user
        let user = self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::InvalidToken)?;

        // Generate new tokens
        let new_access_token = self.jwt_manager.generate_access_token(user.id, &user.email, &user.role)?;
        let new_refresh_token = self.jwt_manager.generate_refresh_token(user.id, &user.email, &user.role)?;

        Ok((new_access_token, new_refresh_token))
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> AppResult<Option<User>> {
        self.user_repo.find_by_id(user_id).await
    }

    /// Verify OTP and return token (used for email verification)
    pub async fn verify_otp(&self, req: crate::models::VerifyOtpRequest) -> AppResult<crate::models::VerifyOtpResponse> {
        self.otp_service.verify_otp(&req.email, &req.code, &req.purpose).await
    }
}
