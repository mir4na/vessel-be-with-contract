use serde::Serialize;

/// Unified API Response struct
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationMeta>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub page: i32,
    pub per_page: i32,
    pub total: i64,
    pub total_pages: i32,
}

impl<T: Serialize> ApiResponse<T> {
    /// Success response with data
    pub fn success(data: T, message: &str) -> Self {
        Self {
            success: true,
            message: Some(message.to_string()),
            data: Some(data),
            error: None,
            pagination: None,
        }
    }

    /// Success response with data only (no message)
    pub fn data(data: T) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
            error: None,
            pagination: None,
        }
    }

    /// Paginated response with data and pagination metadata
    pub fn paginated(data: T, total: i64, page: i32, per_page: i32) -> Self {
        let total_pages = if per_page > 0 {
            ((total as f64) / (per_page as f64)).ceil() as i32
        } else {
            0
        };

        Self {
            success: true,
            message: None,
            data: Some(data),
            error: None,
            pagination: Some(PaginationMeta {
                page,
                per_page,
                total,
                total_pages,
            }),
        }
    }
}

impl ApiResponse<()> {
    /// Success response with message only (no data)
    pub fn success_message(message: &str) -> Self {
        Self {
            success: true,
            message: Some(message.to_string()),
            data: None,
            error: None,
            pagination: None,
        }
    }

    /// Error response
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: None,
            data: None,
            error: Some(ApiError {
                code: "ERROR".to_string(),
                message: message.to_string(),
            }),
            pagination: None,
        }
    }

    /// Error response with custom code
    pub fn error_with_code(code: &str, message: &str) -> Self {
        Self {
            success: false,
            message: None,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
            }),
            pagination: None,
        }
    }
}
