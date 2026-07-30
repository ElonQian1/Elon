//! Contracts for merchant-owned data-source integrations and bounded sync evidence.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub(crate) const INTEGRATION_STATUS_CONFIGURED: &str = "configured";
pub(crate) const INTEGRATION_STATUS_CONNECTED: &str = "connected";
pub(crate) const INTEGRATION_STATUS_DEGRADED: &str = "degraded";
pub(crate) const INTEGRATION_STATUS_DISABLED: &str = "disabled";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceIntegration {
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub integration_key: String,
    pub provider_key: String,
    pub display_name: String,
    pub connection_mode: String,
    pub status: String,
    pub scopes: Vec<String>,
    pub data_domains: Vec<String>,
    pub created_by_user_id: String,
    pub last_verified_at: Option<String>,
    pub last_sync_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceSyncReceipt {
    pub id: String,
    pub project_id: String,
    pub integration_id: String,
    pub receipt_key: String,
    pub sync_kind: String,
    pub status: String,
    pub records_seen: i64,
    pub records_changed: i64,
    pub cursor_digest: Option<String>,
    pub error_code: Option<String>,
    pub recorded_by_user_id: String,
    pub recorded_by_app_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateIntegrationRequest {
    pub merchant_id: String,
    pub integration_key: String,
    pub provider_key: String,
    pub display_name: String,
    pub connection_mode: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub data_domains: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SetIntegrationEnabledRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RecordSyncReceiptRequest {
    pub integration_id: String,
    pub receipt_key: String,
    pub sync_kind: String,
    pub status: String,
    #[serde(default)]
    pub records_seen: i64,
    #[serde(default)]
    pub records_changed: i64,
    #[serde(default)]
    pub cursor_digest: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    pub started_at: String,
    pub completed_at: String,
}

pub(crate) fn normalize_integration_key(value: &str) -> Result<String> {
    normalize_identifier(value, "接入键", 3, 96, &['.', '-', '_', ':'])
}

pub(crate) fn normalize_provider_key(value: &str) -> Result<String> {
    normalize_identifier(value, "平台标识", 2, 64, &['.', '-', '_'])
}

pub(crate) fn normalize_connection_mode(value: &str) -> Result<String> {
    match value.trim() {
        "official_api" => Ok("official_api".to_string()),
        "merchant_export" => Ok("merchant_export".to_string()),
        "local_adapter" => Ok("local_adapter".to_string()),
        "manual_import" => Ok("manual_import".to_string()),
        _ => bail!("接入方式必须是 official_api、merchant_export、local_adapter 或 manual_import"),
    }
}

pub(crate) fn normalize_sync_kind(value: &str) -> Result<String> {
    match value.trim() {
        "full" => Ok("full".to_string()),
        "incremental" => Ok("incremental".to_string()),
        "health_check" => Ok("health_check".to_string()),
        _ => bail!("同步类型必须是 full、incremental 或 health_check"),
    }
}

pub(crate) fn normalize_sync_status(value: &str) -> Result<String> {
    match value.trim() {
        "succeeded" => Ok("succeeded".to_string()),
        "partial" => Ok("partial".to_string()),
        "failed" => Ok("failed".to_string()),
        _ => bail!("同步状态必须是 succeeded、partial 或 failed"),
    }
}

pub(crate) fn normalize_string_list(
    values: &[String],
    label: &str,
    max_items: usize,
) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for value in values {
        let value = normalize_identifier(value, label, 1, 64, &['.', '-', '_', ':'])?;
        if !result.contains(&value) {
            result.push(value);
        }
    }
    if result.len() > max_items {
        bail!("{label}最多允许 {max_items} 项");
    }
    Ok(result)
}

pub(crate) fn normalize_receipt_key(value: &str) -> Result<String> {
    normalize_identifier(value, "同步回执键", 3, 128, &['.', '-', '_', ':'])
}

fn normalize_identifier(
    value: &str,
    label: &str,
    min: usize,
    max: usize,
    extra: &[char],
) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() < min || value.len() > max {
        bail!("{label}长度必须在 {min} 到 {max} 字符之间");
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || extra.contains(&ch))
    {
        bail!("{label}包含不支持的字符");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_contracts_reject_secrets_and_unbounded_labels() {
        assert_eq!(
            normalize_connection_mode("official_api").unwrap(),
            "official_api"
        );
        assert!(normalize_provider_key("bad provider").is_err());
        assert!(normalize_string_list(
            &(0..33)
                .map(|index| format!("scope-{index}"))
                .collect::<Vec<_>>(),
            "授权范围",
            32
        )
        .is_err());
    }
}
