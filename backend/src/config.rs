use std::env;
use anyhow::{Context, Result};

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are loaded from env and may be used in future features
pub struct Config {
    // Server
    pub port: u16,
    pub rust_log: String,

    // Database
    pub database_url: String,
    pub postgres_host: String,
    pub postgres_port: u16,
    pub postgres_user: String,
    pub postgres_password: String,
    pub postgres_db: String,

    // Redis
    pub redis_host: String,
    pub redis_port: u16,
    pub redis_password: String,
    pub redis_db: i64,

    // JWT
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub jwt_refresh_expiry_hours: i64,

    // Blockchain (Base Network)
    pub private_key: String,
    pub blockchain_rpc_url: String,
    pub chain_id: u64,
    pub block_explorer_url: String,
    pub invoice_nft_contract_addr: String,
    pub invoice_pool_contract_addr: String,
    pub idrx_token_contract_addr: String,
    pub platform_wallet_address: String,

    // Pinata (IPFS)
    pub pinata_api_key: String,
    pub pinata_secret_key: String,
    pub pinata_jwt: String,
    pub pinata_gateway_url: String,

    // File Upload
    pub max_file_size_mb: usize,
    pub allowed_file_types: String,

    // Platform Settings
    pub platform_fee_percentage: f64,
    pub default_advance_percentage: f64,
    pub min_invoice_amount: f64,
    pub max_invoice_amount: f64,

    // CORS
    pub cors_allowed_origins: String,

    // Frontend URL
    pub frontend_url: String,

    // SMTP Settings
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,

    // OTP Settings
    pub otp_expiry_minutes: i64,
    pub otp_max_attempts: i32,

    // Currency Conversion
    pub default_buffer_rate: f64,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        let database_url = get_env("DATABASE_URL").unwrap_or_else(|_| {
            format!(
                "postgresql://{}:{}@{}:{}/{}?sslmode=require",
                get_env_or_default("POSTGRES_USER", ""),
                get_env_or_default("POSTGRES_PASSWORD", ""),
                get_env_or_default("POSTGRES_HOST", "localhost"),
                get_env_or_default("POSTGRES_PORT", "5432"),
                get_env_or_default("POSTGRES_DB", "vessel")
            )
        });

        Ok(Self {
            // Server
            port: get_env_or_default("PORT", "8080").parse().unwrap_or(8080),
            rust_log: get_env_or_default("RUST_LOG", "info"),

            // Database
            database_url,
            postgres_host: get_env_or_default("POSTGRES_HOST", "localhost"),
            postgres_port: get_env_or_default("POSTGRES_PORT", "5432").parse().unwrap_or(5432),
            postgres_user: get_env_or_default("POSTGRES_USER", ""),
            postgres_password: get_env_or_default("POSTGRES_PASSWORD", ""),
            postgres_db: get_env_or_default("POSTGRES_DB", "vessel"),

            // Redis
            redis_host: get_env_or_default("REDIS_HOST", "localhost"),
            redis_port: get_env_or_default("REDIS_PORT", "6379").parse().unwrap_or(6379),
            redis_password: get_env_or_default("REDIS_PASSWORD", ""),
            redis_db: get_env_or_default("REDIS_DB", "0").parse().unwrap_or(0),

            // JWT
            jwt_secret: get_env("JWT_SECRET").context("JWT_SECRET is required")?,
            jwt_expiry_hours: get_env_or_default("JWT_EXPIRY_HOURS", "24").parse().unwrap_or(24),
            jwt_refresh_expiry_hours: get_env_or_default("JWT_REFRESH_EXPIRY_HOURS", "168")
                .parse()
                .unwrap_or(168),

            // Blockchain (Base Network - replacing Lisk Sepolia)
            private_key: get_env_or_default("PRIVATE_KEY", ""),
            blockchain_rpc_url: get_env_or_default("BLOCKCHAIN_RPC_URL", "https://mainnet.base.org"),
            chain_id: get_env_or_default("CHAIN_ID", "8453").parse().unwrap_or(8453), // Base Mainnet: 8453, Base Sepolia: 84532
            block_explorer_url: get_env_or_default("BLOCK_EXPLORER_URL", "https://basescan.org"),
            invoice_nft_contract_addr: get_env_or_default("INVOICE_NFT_CONTRACT_ADDRESS", ""),
            invoice_pool_contract_addr: get_env_or_default("INVOICE_POOL_CONTRACT_ADDRESS", ""),
            idrx_token_contract_addr: get_env_or_default("IDRX_TOKEN_CONTRACT_ADDRESS", ""),
            platform_wallet_address: get_env_or_default("PLATFORM_WALLET_ADDRESS", ""),

            // Pinata (IPFS)
            pinata_api_key: get_env_or_default("PINATA_API_KEY", ""),
            pinata_secret_key: get_env_or_default("PINATA_SECRET_KEY", ""),
            pinata_jwt: get_env_or_default("PINATA_JWT", ""),
            pinata_gateway_url: get_env_or_default("PINATA_GATEWAY_URL", ""),

            // File Upload
            max_file_size_mb: get_env_or_default("MAX_FILE_SIZE_MB", "10").parse().unwrap_or(10),
            allowed_file_types: get_env_or_default("ALLOWED_FILE_TYPES", "pdf,png,jpg,jpeg"),

            // Platform Settings
            platform_fee_percentage: get_env_or_default("PLATFORM_FEE_PERCENTAGE", "2.0")
                .parse()
                .unwrap_or(2.0),
            default_advance_percentage: get_env_or_default("DEFAULT_ADVANCE_PERCENTAGE", "80.0")
                .parse()
                .unwrap_or(80.0),
            min_invoice_amount: get_env_or_default("MIN_INVOICE_AMOUNT", "1000")
                .parse()
                .unwrap_or(1000.0),
            max_invoice_amount: get_env_or_default("MAX_INVOICE_AMOUNT", "1000000")
                .parse()
                .unwrap_or(1000000.0),

            // CORS
            cors_allowed_origins: get_env_or_default(
                "CORS_ALLOWED_ORIGINS",
                "http://localhost:3000,http://localhost:8080",
            ),

            // Frontend URL
            frontend_url: get_env_or_default("FRONTEND_URL", "http://localhost:3000"),

            // SMTP Settings
            smtp_host: get_env_or_default("SMTP_HOST", "smtp.gmail.com"),
            smtp_port: get_env_or_default("SMTP_PORT", "587").parse().unwrap_or(587),
            smtp_username: get_env_or_default("SMTP_USERNAME", ""),
            smtp_password: get_env_or_default("SMTP_PASSWORD", "").replace(" ", ""),
            smtp_from: get_env_or_default("SMTP_FROM", ""),

            // OTP Settings
            otp_expiry_minutes: get_env_or_default("OTP_EXPIRY_MINUTES", "5").parse().unwrap_or(5),
            otp_max_attempts: get_env_or_default("OTP_MAX_ATTEMPTS", "5").parse().unwrap_or(5),

            // Currency Conversion
            default_buffer_rate: get_env_or_default("DEFAULT_BUFFER_RATE", "0.015")
                .parse()
                .unwrap_or(0.015),
        })
    }
}

fn get_env(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("Missing environment variable: {}", key))
}

fn get_env_or_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}
