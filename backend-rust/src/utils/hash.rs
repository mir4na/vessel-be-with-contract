use bcrypt::{hash, verify, DEFAULT_COST};

use crate::error::{AppError, AppResult};

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> AppResult<String> {
    hash(password, DEFAULT_COST).map_err(|e| AppError::InternalError(format!("Hash error: {}", e)))
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    verify(password, hash).map_err(|e| AppError::InternalError(format!("Verify error: {}", e)))
}

/// Generate a random OTP code (6 digits)
pub fn generate_otp() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1000000))
}

/// Generate a random token for various purposes
pub fn generate_random_token() -> String {
    use rand::Rng;
    use ethers::utils::hex;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

/// Generate a virtual account number
pub fn generate_va_number(bank_code: &str, user_id: &str) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_part: u32 = rng.gen_range(10000000..99999999);

    // Format: bank prefix + random + last 4 of user_id
    let user_suffix = if user_id.len() >= 4 {
        &user_id[user_id.len() - 4..]
    } else {
        user_id
    };

    let prefix = match bank_code {
        "bca" => "8888",
        "mandiri" => "8899",
        "bni" => "8800",
        "bri" => "8877",
        _ => "8000",
    };

    format!("{}{}{}", prefix, random_part, user_suffix.chars().filter(|c| c.is_numeric()).collect::<String>())
}
