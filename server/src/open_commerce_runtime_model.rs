//! Contracts for a verified merchant-owned runtime.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub(crate) const RUNTIME_STATUS_CONFIGURED: &str = "configured";
pub(crate) const RUNTIME_STATUS_ACTIVE: &str = "active";
pub(crate) const RUNTIME_STATUS_DEGRADED: &str = "degraded";
pub(crate) const RUNTIME_STATUS_DISABLED: &str = "disabled";
pub(crate) const RUNTIME_SECRET_PREFIX: &str = "OPEN_COMMERCE_RUNTIME_SECRET_";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceRuntimeBinding {
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub endpoint_base_url: String,
    pub credential_ref: String,
    pub manifest_sha256: Option<String>,
    pub timeout_ms: i64,
    pub status: String,
    pub last_verified_at: Option<String>,
    pub last_error_code: Option<String>,
    pub created_by_user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpsertRuntimeBindingRequest {
    pub endpoint_base_url: String,
    pub credential_ref: String,
    #[serde(default)]
    pub manifest_sha256: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MerchantRuntimeEnvelope {
    pub schema: &'static str,
    pub invocation_id: String,
    pub merchant_id: String,
    pub capability_key: String,
    pub requester_user_id: String,
    pub requester_app_id: String,
    pub credential_environment: String,
    pub credential_id: Option<String>,
    pub grant_id: Option<String>,
    pub idempotency_key: String,
    pub issued_at_unix: i64,
    pub input: serde_json::Value,
}

pub(crate) fn normalize_credential_ref(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_uppercase();
    if !value.starts_with(RUNTIME_SECRET_PREFIX) || value.len() > 128 {
        bail!("运行密钥引用必须使用 OPEN_COMMERCE_RUNTIME_SECRET_ 前缀");
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        bail!("运行密钥引用只能包含大写字母、数字和下划线");
    }
    Ok(value)
}

pub(crate) fn normalize_manifest_sha256(value: Option<&str>) -> Result<Option<String>> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let value = value.to_ascii_lowercase();
            if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
                bail!("manifest_sha256 必须是 64 位十六进制摘要");
            }
            Ok(value)
        })
        .transpose()
}

pub(crate) fn normalize_timeout_ms(value: i64) -> Result<i64> {
    if !(500..=15_000).contains(&value) {
        bail!("商户运行超时必须在 500 到 15000 毫秒之间");
    }
    Ok(value)
}

fn default_timeout_ms() -> i64 {
    5_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_refs_never_accept_plain_secrets() {
        assert!(normalize_credential_ref("shared-secret").is_err());
        assert_eq!(
            normalize_credential_ref("open_commerce_runtime_secret_coffice").unwrap(),
            "OPEN_COMMERCE_RUNTIME_SECRET_COFFICE"
        );
    }
}
