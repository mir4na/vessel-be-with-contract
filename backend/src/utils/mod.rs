#![allow(dead_code)] // Utility functions available for future features

mod jwt;
mod hash;
pub mod response;
mod validator;

pub use jwt::*;
pub use hash::*;
pub use response::*;

/// Verify JWT token helper function used by middleware
pub fn verify_token(token: &str, secret: &str) -> crate::error::AppResult<Claims> {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    if token_data.claims.token_type != "access" {
        return Err(crate::error::AppError::InvalidToken);
    }
    Ok(token_data.claims)
}
