#![allow(dead_code)] // Common models for future use

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Note: ApiResponse, ApiError, PaginationMeta are available from crate::utils::response directly

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_per_page")]
    pub per_page: i32,
}

fn default_page() -> i32 {
    1
}

fn default_per_page() -> i32 {
    10
}

impl PaginationParams {
    pub fn normalize(&mut self) {
        if self.page < 1 {
            self.page = 1;
        }
        if self.per_page < 1 {
            self.per_page = 10;
        }
        if self.per_page > 100 {
            self.per_page = 100;
        }
    }

    pub fn offset(&self) -> i32 {
        (self.page - 1) * self.per_page
    }
}

pub fn calculate_total_pages(total: i64, per_page: i32) -> i32 {
    if per_page == 0 {
        return 0;
    }
    let pages = total / per_page as i64;
    if total % per_page as i64 > 0 {
        (pages + 1) as i32
    } else {
        pages as i32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub title: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<HashMap<String, serde_json::Value>>,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

// Simple message response
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}
