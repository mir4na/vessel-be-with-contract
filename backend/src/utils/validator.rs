use validator::Validate;

use crate::error::{AppError, AppResult};

/// Validate a request struct using the validator crate
pub fn validate_request<T: Validate>(request: &T) -> AppResult<()> {
    request.validate().map_err(|e| {
        let errors: Vec<String> = e
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |err| {
                    format!(
                        "{}: {}",
                        field,
                        err.message.clone().unwrap_or_else(|| "Invalid value".into())
                    )
                })
            })
            .collect();

        AppError::ValidationError(errors.join(", "))
    })
}

/// Validate email format
pub fn is_valid_email(email: &str) -> bool {
    // Simple email validation
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let domain_parts: Vec<&str> = parts[1].split('.').collect();
    if domain_parts.len() < 2 {
        return false;
    }
    !parts[0].is_empty() && domain_parts.iter().all(|p| !p.is_empty())
}

/// Validate Indonesian NIK (16 digits)
pub fn is_valid_nik(nik: &str) -> bool {
    nik.len() == 16 && nik.chars().all(|c| c.is_numeric())
}

/// Validate Indonesian NPWP (15-16 digits)
pub fn is_valid_npwp(npwp: &str) -> bool {
    let digits_only: String = npwp.chars().filter(|c| c.is_numeric()).collect();
    digits_only.len() >= 15 && digits_only.len() <= 16
}

/// Validate Ethereum address format
pub fn is_valid_eth_address(address: &str) -> bool {
    if !address.starts_with("0x") {
        return false;
    }
    address.len() == 42 && address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate transaction hash format
pub fn is_valid_tx_hash(hash: &str) -> bool {
    if !hash.starts_with("0x") {
        return false;
    }
    hash.len() == 66 && hash[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate Indonesian phone number
pub fn is_valid_phone_id(phone: &str) -> bool {
    let digits_only: String = phone.chars().filter(|c| c.is_numeric()).collect();
    // Indonesian phone: starts with 08 or 62, 10-14 digits
    (digits_only.starts_with("08") || digits_only.starts_with("62"))
        && digits_only.len() >= 10
        && digits_only.len() <= 14
}

/// Validate password strength
pub fn is_strong_password(password: &str) -> bool {
    // At least 8 characters
    if password.len() < 8 {
        return false;
    }

    // Has at least one uppercase, one lowercase, and one digit
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());

    has_upper && has_lower && has_digit
}
