use std::sync::Arc;
use chrono::{Duration, Utc};

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::{ConvertCurrencyResponse, DisbursementEstimateResponse, get_supported_currencies, SupportedCurrency};
use crate::utils::generate_random_token;

pub struct CurrencyService {
    config: Arc<Config>,
}

impl CurrencyService {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn get_supported_currencies(&self) -> Vec<SupportedCurrency> {
        get_supported_currencies()
    }

    pub async fn get_locked_exchange_rate(&self, from_currency: &str, amount: f64) -> AppResult<ConvertCurrencyResponse> {
        // Validate currency
        let supported = self.get_supported_currencies();
        if !supported.iter().any(|c| c.code == from_currency) {
            return Err(AppError::ValidationError(format!("Unsupported currency: {}", from_currency)));
        }

        // Get exchange rate (in production, this would call an external API)
        let exchange_rate = self.get_exchange_rate(from_currency).await?;

        // Apply buffer rate
        let buffer_rate = self.config.default_buffer_rate;
        let effective_rate = exchange_rate * (1.0 - buffer_rate);

        // Calculate converted amount
        let converted_amount = amount * effective_rate;

        // Generate lock token
        let lock_token = generate_random_token();

        // Lock expires in 30 minutes
        let locked_until = (Utc::now() + Duration::minutes(30)).to_rfc3339();

        Ok(ConvertCurrencyResponse {
            from_currency: from_currency.to_string(),
            to_currency: "IDR".to_string(),
            original_amount: amount,
            exchange_rate,
            buffer_rate,
            effective_rate,
            converted_amount,
            locked_until,
            rate_lock_token: lock_token,
        })
    }

    pub fn calculate_disbursement_estimate(&self, idr_amount: f64) -> DisbursementEstimateResponse {
        let platform_fee_pct = self.config.platform_fee_percentage;
        let platform_fee_amount = idr_amount * (platform_fee_pct / 100.0);
        let net_disbursement = idr_amount - platform_fee_amount;

        DisbursementEstimateResponse {
            gross_amount: idr_amount,
            platform_fee_percentage: platform_fee_pct,
            platform_fee_amount,
            net_disbursement,
            currency: "IDR".to_string(),
        }
    }

    async fn get_exchange_rate(&self, from_currency: &str) -> AppResult<f64> {
        // In production, this would call an external forex API
        // For now, return mock rates
        let rate = match from_currency {
            "USD" => 15500.0,
            "EUR" => 17000.0,
            "GBP" => 19500.0,
            "JPY" => 105.0,
            "SGD" => 11500.0,
            "AUD" => 10500.0,
            "CNY" => 2150.0,
            _ => return Err(AppError::ValidationError(format!("Unknown currency: {}", from_currency))),
        };

        Ok(rate)
    }
}
