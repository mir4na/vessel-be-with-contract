use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct SupportedCurrency {
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub flag_emoji: String,
}

pub fn get_supported_currencies() -> Vec<SupportedCurrency> {
    vec![
        SupportedCurrency {
            code: "USD".to_string(),
            name: "US Dollar".to_string(),
            symbol: "$".to_string(),
            flag_emoji: "🇺🇸".to_string(),
        },
        SupportedCurrency {
            code: "EUR".to_string(),
            name: "Euro".to_string(),
            symbol: "€".to_string(),
            flag_emoji: "🇪🇺".to_string(),
        },
        SupportedCurrency {
            code: "GBP".to_string(),
            name: "British Pound".to_string(),
            symbol: "£".to_string(),
            flag_emoji: "🇬🇧".to_string(),
        },
        SupportedCurrency {
            code: "JPY".to_string(),
            name: "Japanese Yen".to_string(),
            symbol: "¥".to_string(),
            flag_emoji: "🇯🇵".to_string(),
        },
        SupportedCurrency {
            code: "SGD".to_string(),
            name: "Singapore Dollar".to_string(),
            symbol: "S$".to_string(),
            flag_emoji: "🇸🇬".to_string(),
        },
        SupportedCurrency {
            code: "AUD".to_string(),
            name: "Australian Dollar".to_string(),
            symbol: "A$".to_string(),
            flag_emoji: "🇦🇺".to_string(),
        },
        SupportedCurrency {
            code: "CNY".to_string(),
            name: "Chinese Yuan".to_string(),
            symbol: "¥".to_string(),
            flag_emoji: "🇨🇳".to_string(),
        },
    ]
}

#[derive(Debug, Deserialize, Validate)]
pub struct ConvertCurrencyRequest {
    pub from_currency: String,
    #[validate(range(min = 0.01, message = "Amount must be positive"))]
    pub amount: f64,
}

#[derive(Debug, Serialize)]
pub struct ConvertCurrencyResponse {
    pub from_currency: String,
    pub to_currency: String,
    pub original_amount: f64,
    pub exchange_rate: f64,
    pub buffer_rate: f64,
    pub effective_rate: f64,
    pub converted_amount: f64,
    pub locked_until: String,
    pub rate_lock_token: String,
}

#[derive(Debug, Deserialize)]
pub struct DisbursementEstimateRequest {
    pub idr_amount: f64,
}

#[derive(Debug, Serialize)]
pub struct DisbursementEstimateResponse {
    pub gross_amount: f64,
    pub platform_fee_percentage: f64,
    pub platform_fee_amount: f64,
    pub net_disbursement: f64,
    pub currency: String,
}
