use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::Config;
use crate::error::{AppError, AppResult};

pub struct PinataService {
    config: Arc<Config>,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PinataResponse {
    #[serde(rename = "IpfsHash")]
    pub ipfs_hash: String,
    #[serde(rename = "PinSize")]
    pub pin_size: i64,
    #[serde(rename = "Timestamp")]
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct PinataMetadata {
    pub name: String,
    #[serde(rename = "keyvalues")]
    pub key_values: serde_json::Value,
}

impl PinataService {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn upload_file(&self, file_data: Vec<u8>, file_name: &str) -> AppResult<String> {
        if self.config.pinata_jwt.is_empty() {
            return Err(AppError::IpfsError("Pinata JWT not configured".to_string()));
        }

        let part = multipart::Part::bytes(file_data)
            .file_name(file_name.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| AppError::IpfsError(e.to_string()))?;

        let form = multipart::Form::new().part("file", part);

        let response = self
            .client
            .post("https://api.pinata.cloud/pinning/pinFileToIPFS")
            .header(
                "Authorization",
                format!("Bearer {}", self.config.pinata_jwt),
            )
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::IpfsError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::IpfsError(format!(
                "Pinata upload failed: {}",
                error_text
            )));
        }

        let result: PinataResponse = response
            .json()
            .await
            .map_err(|e| AppError::IpfsError(e.to_string()))?;

        let gateway_url = if self.config.pinata_gateway_url.is_empty() {
            format!("https://gateway.pinata.cloud/ipfs/{}", result.ipfs_hash)
        } else {
            format!(
                "https://{}/ipfs/{}",
                self.config.pinata_gateway_url, result.ipfs_hash
            )
        };

        Ok(gateway_url)
    }

    pub async fn upload_json(&self, json_data: serde_json::Value, name: &str) -> AppResult<String> {
        if self.config.pinata_jwt.is_empty() {
            return Err(AppError::IpfsError("Pinata JWT not configured".to_string()));
        }

        let body = serde_json::json!({
            "pinataContent": json_data,
            "pinataMetadata": {
                "name": name
            }
        });

        let response = self
            .client
            .post("https://api.pinata.cloud/pinning/pinJSONToIPFS")
            .header(
                "Authorization",
                format!("Bearer {}", self.config.pinata_jwt),
            )
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::IpfsError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::IpfsError(format!(
                "Pinata upload failed: {}",
                error_text
            )));
        }

        let result: PinataResponse = response
            .json()
            .await
            .map_err(|e| AppError::IpfsError(e.to_string()))?;

        let gateway_url = if self.config.pinata_gateway_url.is_empty() {
            format!("https://gateway.pinata.cloud/ipfs/{}", result.ipfs_hash)
        } else {
            format!(
                "https://{}/ipfs/{}",
                self.config.pinata_gateway_url, result.ipfs_hash
            )
        };

        Ok(gateway_url)
    }

    pub fn get_ipfs_hash_from_url(&self, url: &str) -> Option<String> {
        url.split("/ipfs/").last().map(|s| s.to_string())
    }
}
