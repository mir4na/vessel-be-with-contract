use ethers::types::{Address, H256, Signature};
use ethers::utils::hex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::{
    ConnectWalletRequest, GoogleAuthRequest, GoogleAuthResponse, InvestorWalletRegisterRequest,
    LoginRequest, LoginResponse, RegisterRequest, User, WalletLoginRequest, WalletNonceResponse,
};
use crate::repository::{MitraRepository, UserRepository};
use crate::utils::{generate_random_token, hash_password, verify_password, JwtManager};

use super::{BlockchainService, OtpService};

pub struct AuthService {
    user_repo: Arc<UserRepository>,
    mitra_repo: Arc<MitraRepository>,
    jwt_manager: Arc<JwtManager>,
    otp_service: Arc<OtpService>,
    config: Arc<Config>,
    wallet_nonces: Arc<RwLock<HashMap<String, String>>>,
    blockchain_service: Arc<BlockchainService>,
}

impl AuthService {
    pub fn new(
        user_repo: Arc<UserRepository>,
        mitra_repo: Arc<MitraRepository>,
        jwt_manager: Arc<JwtManager>,
        otp_service: Arc<OtpService>,
        config: Arc<Config>,
        blockchain_service: Arc<BlockchainService>,
    ) -> Self {
        Self {
            user_repo,
            mitra_repo,
            jwt_manager,
            otp_service,
            config,
            wallet_nonces: Arc::new(RwLock::new(HashMap::new())),
            blockchain_service,
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

    /// Verify wallet signature (supports EOA and ERC-1271 Smart Wallets)
    async fn verify_wallet_signature(
        &self,
        wallet_address: &str,
        signature_str: &str,
        message: &str,
    ) -> AppResult<bool> {
        let wallet_addr: Address = wallet_address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid wallet address".to_string()))?;

        // 1. Prepare message hash (EIP-191)
        // We use BlockchainService helper to ensure consistency
        let message_hash = self.blockchain_service.hash_message(message);

        // 2. Decode signature
        // Handle 0x prefix if present
        let sig_clean = signature_str.strip_prefix("0x").unwrap_or(signature_str);
        let signature_bytes = hex::decode(sig_clean)
            .map_err(|_| AppError::ValidationError("Invalid signature hex".to_string()))?;

        // 3. Attempt EOA Verification (Standard ECDSA)
        // Only if signature length is 65 bytes
        if signature_bytes.len() == 65 {
            if let Ok(sig) = signature_str.parse::<Signature>() {
                if let Ok(recovered) = sig.recover(H256::from(message_hash)) {
                    if recovered == wallet_addr {
                        return Ok(true);
                    }
                }
            }
        }

        // 4. Fallback: ERC-1271 Verification (Smart Contract Wallet)
        // If EOA check failed or format was different, check on-chain
        self.blockchain_service
            .verify_signature_erc1271(wallet_address, message_hash, signature_bytes)
            .await
    }

    /// Wallet login for investors and mitra (supports Base Smart Wallet / passkey via ERC-1271)
    pub async fn wallet_login(&self, req: WalletLoginRequest) -> AppResult<LoginResponse> {
        let wallet = req.wallet_address.to_lowercase();

        // Verify nonce
        {
            let nonces = self.wallet_nonces.read().await;
            let stored_nonce = nonces
                .get(&wallet)
                .ok_or_else(|| AppError::ValidationError("Invalid or expired nonce".to_string()))?;

            if stored_nonce != &req.nonce {
                return Err(AppError::ValidationError("Nonce mismatch".to_string()));
            }
        }

        // Verify signature
        if !self.verify_wallet_signature(&wallet, &req.signature, &req.message).await? {
            return Err(AppError::InvalidCredentials);
        }

        // Clear used nonce
        {
            let mut nonces = self.wallet_nonces.write().await;
            nonces.remove(&wallet);
        }

        // Find or create user by wallet
        // Supports both investor and mitra with connected wallets (Base Smart Wallet / passkey)
        let user = match self.user_repo.find_by_wallet(&wallet).await? {
            Some(user) => user,
            None => {
                // Auto-create investor account with wallet
                self.user_repo.create_investor_with_wallet(&wallet).await?
            }
        };

        if !user.is_active {
            return Err(AppError::Forbidden("Account is deactivated".to_string()));
        }

        // Generate tokens
        let access_token = self.jwt_manager.generate_access_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;
        let refresh_token = self.jwt_manager.generate_refresh_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;

        Ok(LoginResponse {
            user,
            access_token,
            refresh_token,
            expires_in: self.jwt_manager.get_expiry_hours() * 3600,
        })
    }

    /// Register investor with wallet only
    pub async fn register_investor_wallet(
        &self,
        req: InvestorWalletRegisterRequest,
    ) -> AppResult<LoginResponse> {
        let wallet = req.wallet_address.to_lowercase();

        // Verify cooperative agreement
        if !req.cooperative_agreement {
            return Err(AppError::ValidationError(
                "Must accept cooperative agreement".to_string(),
            ));
        }

        // Verify nonce
        {
            let nonces = self.wallet_nonces.read().await;
            let stored_nonce = nonces
                .get(&wallet)
                .ok_or_else(|| AppError::ValidationError("Invalid or expired nonce".to_string()))?;

            if stored_nonce != &req.nonce {
                return Err(AppError::ValidationError("Nonce mismatch".to_string()));
            }
        }

        // Verify signature
        if !self.verify_wallet_signature(&wallet, &req.signature, &req.message).await? {
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
        let access_token = self.jwt_manager.generate_access_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;
        let refresh_token = self.jwt_manager.generate_refresh_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;

        Ok(LoginResponse {
            user,
            access_token,
            refresh_token,
            expires_in: self.jwt_manager.get_expiry_hours() * 3600,
        })
    }

    /// Connect wallet to existing account with signature verification
    /// Supports Base Smart Wallet (passkey) via ERC-1271
    /// Works for both investor and mitra accounts
    pub async fn connect_wallet(
        &self,
        user_id: Uuid,
        req: ConnectWalletRequest,
    ) -> AppResult<User> {
        let wallet = req.wallet_address.to_lowercase();

        // Verify nonce
        {
            let nonces = self.wallet_nonces.read().await;
            let stored_nonce = nonces
                .get(&wallet)
                .ok_or_else(|| AppError::ValidationError("Invalid or expired nonce".to_string()))?;

            if stored_nonce != &req.nonce {
                return Err(AppError::ValidationError("Nonce mismatch".to_string()));
            }
        }

        // Verify signature (supports both EOA and ERC-1271 / Base Smart Wallet / passkey)
        if !self
            .verify_wallet_signature(&wallet, &req.signature, &req.message)
            .await?
        {
            return Err(AppError::InvalidCredentials);
        }

        // Clear used nonce
        {
            let mut nonces = self.wallet_nonces.write().await;
            nonces.remove(&wallet);
        }

        // Check if wallet is already used by another account
        if let Some(existing) = self.user_repo.find_by_wallet(&wallet).await? {
            if existing.id != user_id {
                return Err(AppError::Conflict(
                    "Wallet already connected to another account".to_string(),
                ));
            }
            // Wallet already connected to this user — return as-is
            return Ok(existing);
        }

        // Update wallet address on user record
        let user = self.user_repo.update_wallet(user_id, &wallet).await?;

        tracing::info!(
            "Wallet connected: user={}, wallet={}",
            user_id,
            wallet
        );

        Ok(user)
    }

    /// Traditional registration (for mitra only - investors use wallet login)
    pub async fn register(&self, req: RegisterRequest) -> AppResult<LoginResponse> {
        // Verify OTP token
        let email = self
            .otp_service
            .verify_otp_token(&req.otp_token, "registration")
            .map_err(|e| {
                tracing::warn!("Registration failed: Invalid OTP token: {}", e);
                e
            })?;

        // Check if email matches
        if email.to_lowercase() != req.email.to_lowercase() {
            tracing::warn!(
                "Registration failed: Email mismatch (token: {}, req: {})",
                email,
                req.email
            );
            return Err(AppError::ValidationError(
                "OTP token does not match email".to_string(),
            ));
        }

        // Check if email already exists
        if self.user_repo.find_by_email(&req.email).await?.is_some() {
            tracing::warn!(
                "Registration failed: Email already registered: {}",
                req.email
            );
            return Err(AppError::Conflict("Email already registered".to_string()));
        }

        // Check if username already exists
        if self
            .user_repo
            .find_by_username(&req.username)
            .await?
            .is_some()
        {
            tracing::warn!(
                "Registration failed: Username already taken: {}",
                req.username
            );
            return Err(AppError::Conflict("Username already taken".to_string()));
        }

        // Validate password confirmation
        if req.password != req.confirm_password {
            tracing::warn!(
                "Registration failed: Passwords do not match for {}",
                req.email
            );
            return Err(AppError::ValidationError(
                "Passwords do not match".to_string(),
            ));
        }

        // Validate cooperative agreement
        if !req.cooperative_agreement {
            tracing::warn!(
                "Registration failed: Cooperative agreement not accepted by {}",
                req.email
            );
            return Err(AppError::ValidationError(
                "Must accept cooperative agreement".to_string(),
            ));
        }

        // Hash password
        let password_hash = hash_password(&req.password)?;

        // Create user with role "mitra" and member_status "calon_anggota_mitra"
        let mut user = self
            .user_repo
            .create(&req.email, &req.username, &password_hash, "mitra")
            .await?;

        // Update member_status to calon_anggota_mitra
        self.user_repo
            .update_member_status(user.id, "calon_anggota_mitra")
            .await?;
        user.member_status = "calon_anggota_mitra".to_string();

        // Set profile_completed based on whether company details were provided
        // Check if company_name is SOME and NOT EMPTY string
        let is_profile_complete = req
            .company_name
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        self.user_repo
            .set_profile_completed(user.id, is_profile_complete)
            .await?;
        user.profile_completed = is_profile_complete;

        // Auto-create mitra application with pending status ONLY if company name provided and not empty
        if let Some(company_name) = &req.company_name {
            if !company_name.is_empty() {
                let _mitra_application = self
                    .mitra_repo
                    .create(
                        user.id,
                        company_name,
                        req.company_type.as_deref().unwrap_or("PT"),
                        req.npwp.as_deref().unwrap_or(""),
                        req.annual_revenue.as_deref().unwrap_or(""),
                        req.address.as_deref(),
                        req.business_description.as_deref(),
                        req.website_url.as_deref(),
                        req.year_founded,
                        req.key_products.as_deref(),
                        req.export_markets.as_deref(),
                    )
                    .await?;

                tracing::info!(
                    "Mitra registered: {} with pending application for company: {}",
                    req.email,
                    company_name
                );
            } else {
                tracing::info!(
                    "Mitra registered: {} (without initial company profile)",
                    req.email
                );
            }
        } else {
            tracing::info!(
                "Mitra registered: {} (without initial company profile)",
                req.email
            );
        }

        // Generate tokens
        let access_token = self.jwt_manager.generate_access_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;
        let refresh_token = self.jwt_manager.generate_refresh_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;

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
        let user = self
            .user_repo
            .find_by_email_or_username(&req.email_or_username)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        // Investors must use wallet login
        if user.role == "investor" {
            return Err(AppError::Forbidden(
                "Investors must use wallet login. Please connect your wallet instead.".to_string(),
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
        let access_token = self.jwt_manager.generate_access_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;
        let refresh_token = self.jwt_manager.generate_refresh_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;

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
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::InvalidToken)?;

        // Get user
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::InvalidToken)?;

        // Generate new tokens
        let new_access_token = self.jwt_manager.generate_access_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;
        let new_refresh_token = self.jwt_manager.generate_refresh_token(
            user.id,
            user.email.as_deref().unwrap_or(""),
            &user.role,
        )?;

        Ok((new_access_token, new_refresh_token))
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> AppResult<Option<User>> {
        self.user_repo.find_by_id(user_id).await
    }

    /// Verify OTP and return token (used for email verification)
    pub async fn verify_otp(
        &self,
        req: crate::models::VerifyOtpRequest,
    ) -> AppResult<crate::models::VerifyOtpResponse> {
        self.otp_service
            .verify_otp(&req.email, &req.code, &req.purpose)
            .await
    }

    /// Verify Google ID token and return OTP token (skips email OTP verification)
    pub async fn google_auth(&self, req: GoogleAuthRequest) -> AppResult<GoogleAuthResponse> {
        // Verify Google ID token by calling Google's tokeninfo endpoint
        let client = reqwest::Client::new();
        let google_response = client
            .get(format!(
                "https://oauth2.googleapis.com/tokeninfo?id_token={}",
                req.id_token
            ))
            .send()
            .await
            .map_err(|e| {
                AppError::ValidationError(format!("Failed to verify Google token: {}", e))
            })?;

        if !google_response.status().is_success() {
            return Err(AppError::ValidationError(
                "Invalid Google token".to_string(),
            ));
        }

        #[derive(serde::Deserialize)]
        struct GoogleTokenInfo {
            email: String,
            email_verified: String,
            aud: String,
        }

        let token_info: GoogleTokenInfo = google_response.json().await.map_err(|e| {
            AppError::ValidationError(format!("Failed to parse Google response: {}", e))
        })?;

        // Verify the token audience matches our client ID
        if !self.config.google_client_id.is_empty()
            && token_info.aud != self.config.google_client_id
        {
            return Err(AppError::ValidationError(
                "Invalid Google client ID".to_string(),
            ));
        }

        // Verify email is verified by Google
        if token_info.email_verified != "true" {
            return Err(AppError::ValidationError(
                "Google email is not verified".to_string(),
            ));
        }

        // Generate OTP token directly (skipping email OTP since Google already verified)
        let otp_token = self
            .jwt_manager
            .generate_otp_token(&token_info.email, "registration")?;

        Ok(GoogleAuthResponse {
            email: token_info.email,
            otp_token,
            expires_in_minutes: 30,
        })
    }
}
