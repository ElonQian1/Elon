//! Shared V1 contracts for the AI-native open commerce network.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::open_commerce_app_activity_health_model::OpenCommerceAppActivityHealth;
use crate::open_commerce_directory_model::OpenCommerceDirectoryPublication;
use crate::open_commerce_integration_model::{OpenCommerceIntegration, OpenCommerceSyncReceipt};
use crate::open_commerce_rate_limit_model::{
    OpenCommerceRateLimitPolicy, OpenCommerceRateLimitUsage,
};
use crate::open_commerce_runtime_model::OpenCommerceRuntimeBinding;

pub(crate) const OPEN_COMMERCE_SCHEMA: &str = "open_commerce.v1";
pub(crate) const MERCHANT_STATUS_ACTIVE: &str = "active";
pub(crate) const MERCHANT_STATUS_DISABLED: &str = "disabled";
pub(crate) const CAPABILITY_STATUS_ACTIVE: &str = "active";
pub(crate) const CAPABILITY_STATUS_DISABLED: &str = "disabled";
pub(crate) const ACCESS_PUBLIC: &str = "public";
pub(crate) const ACCESS_AUTHORIZED: &str = "authorized";
pub(crate) const ACCESS_OWNER_ONLY: &str = "owner_only";
pub(crate) const HANDLER_MERCHANT_PROFILE: &str = "merchant_profile";
pub(crate) const HANDLER_STATIC_JSON: &str = "static_json";
pub(crate) const HANDLER_MERCHANT_RUNTIME: &str = "merchant_runtime";
pub(crate) const SETTLEMENT_RECORDED_NOT_CHARGED: &str = "recorded_not_charged";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceMerchant {
    pub id: String,
    pub project_id: String,
    pub owner_user_id: String,
    pub slug: String,
    pub display_name: String,
    pub description: String,
    pub status: String,
    pub node_mode: String,
    pub public_profile: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceCapability {
    pub id: String,
    pub merchant_id: String,
    pub capability_key: String,
    pub display_name: String,
    pub description: String,
    pub kind: String,
    pub access_level: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub handler_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler_config: Option<Value>,
    pub unit_price_micros: i64,
    pub currency: String,
    pub freshness_seconds: i64,
    pub status: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceMerchantDetail {
    pub schema: &'static str,
    pub merchant: OpenCommerceMerchant,
    pub capabilities: Vec<OpenCommerceCapability>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceGrant {
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub grantor_user_id: String,
    pub grantee_app_id: String,
    pub scopes: Vec<String>,
    pub purpose: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub max_invocations: Option<i64>,
    pub max_amount_micros: Option<i64>,
    pub budget_currency: String,
    pub used_invocations: i64,
    pub used_amount_micros: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceInvocation {
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub capability_id: String,
    pub capability_key: String,
    pub requester_user_id: String,
    pub requester_app_id: String,
    pub credential_environment: String,
    pub credential_id: Option<String>,
    pub grant_id: Option<String>,
    pub idempotency_key: String,
    pub request_hash: String,
    pub request_shape: Value,
    pub status: String,
    pub result: Option<Value>,
    pub error_code: Option<String>,
    pub units: i64,
    pub unit_price_micros: i64,
    pub amount_micros: i64,
    pub currency: String,
    pub settlement_status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAuditEvent {
    pub id: String,
    pub project_id: String,
    pub actor_user_id: String,
    pub actor_app_id: Option<String>,
    pub action: String,
    pub subject_type: String,
    pub subject_id: String,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceOverview {
    pub schema: &'static str,
    pub project_id: String,
    pub merchants: Vec<OpenCommerceMerchantDetail>,
    pub directory_publications: Vec<OpenCommerceDirectoryPublication>,
    pub grants: Vec<OpenCommerceGrant>,
    pub recent_invocations: Vec<OpenCommerceInvocation>,
    pub integrations: Vec<OpenCommerceIntegration>,
    pub runtime_bindings: Vec<OpenCommerceRuntimeBinding>,
    pub recent_sync_receipts: Vec<OpenCommerceSyncReceipt>,
    pub recent_audit_events: Vec<OpenCommerceAuditEvent>,
    pub rate_limit_policies: Vec<OpenCommerceRateLimitPolicy>,
    pub rate_limit_usage: Vec<OpenCommerceRateLimitUsage>,
    pub app_activity_health: Vec<OpenCommerceAppActivityHealth>,
    pub totals: OpenCommerceTotals,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct OpenCommerceTotals {
    pub merchants: usize,
    pub active_merchants: usize,
    pub published_merchants: usize,
    pub capabilities: usize,
    pub active_capabilities: usize,
    pub active_grants: usize,
    pub invocations: usize,
    pub integrations: usize,
    pub connected_integrations: usize,
    pub degraded_integrations: usize,
    pub active_runtime_bindings: usize,
    pub sync_receipts: usize,
    pub metered_amount_micros: i64,
    pub rate_limit_policies: usize,
    pub active_rate_limit_policies: usize,
    pub recent_rate_limited_invocations: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateMerchantRequest {
    pub display_name: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_node_mode")]
    pub node_mode: String,
    #[serde(default = "empty_object")]
    pub public_profile: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpdateMerchantRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub node_mode: Option<String>,
    #[serde(default)]
    pub public_profile: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateCapabilityRequest {
    pub capability_key: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_capability_kind")]
    pub kind: String,
    #[serde(default = "default_access_level")]
    pub access_level: String,
    #[serde(default = "empty_object")]
    pub input_schema: Value,
    #[serde(default = "empty_object")]
    pub output_schema: Value,
    pub handler_type: String,
    #[serde(default)]
    pub handler_config: Option<Value>,
    #[serde(default)]
    pub unit_price_micros: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub freshness_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpdateCapabilityRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub access_level: Option<String>,
    #[serde(default)]
    pub input_schema: Option<Value>,
    #[serde(default)]
    pub output_schema: Option<Value>,
    #[serde(default)]
    pub handler_type: Option<String>,
    #[serde(default)]
    pub handler_config: Option<Value>,
    #[serde(default)]
    pub unit_price_micros: Option<i64>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub freshness_seconds: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateGrantRequest {
    pub merchant_id: String,
    pub grantee_app_id: String,
    pub scopes: Vec<String>,
    pub purpose: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub max_invocations: Option<i64>,
    #[serde(default)]
    pub max_amount_micros: Option<i64>,
    #[serde(default = "default_currency")]
    pub budget_currency: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct InvokeCapabilityRequest {
    pub merchant_id: String,
    pub capability_key: String,
    pub requester_app_id: String,
    #[serde(default)]
    pub grant_id: Option<String>,
    pub idempotency_key: String,
    #[serde(default = "empty_object")]
    pub input: Value,
}

pub(crate) fn normalize_slug(value: &str) -> Result<String> {
    normalize_identifier(value, "商户 slug", 3, 64, &['-', '_'])
}

pub(crate) fn slug_from_display_name(value: &str) -> Result<String> {
    let slug = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.len() >= 3 {
        normalize_slug(&slug)
    } else {
        Ok(format!("merchant-{}", uuid::Uuid::new_v4().simple()))
    }
}

pub(crate) fn normalize_capability_key(value: &str) -> Result<String> {
    normalize_identifier(value, "能力键", 3, 96, &['.', '-', '_'])
}

pub(crate) fn normalize_app_id(value: &str) -> Result<String> {
    normalize_identifier(value, "调用方 App ID", 2, 96, &['.', '-', '_', ':'])
}

pub(crate) fn normalize_idempotency_key(value: &str) -> Result<String> {
    let value = value.trim();
    if value.len() < 3 || value.len() > 128 {
        bail!("幂等键长度必须在 3 到 128 字符之间");
    }
    if value.chars().any(char::is_whitespace) {
        bail!("幂等键不能包含空白字符");
    }
    Ok(value.to_string())
}

pub(crate) fn validate_display_name(value: &str, label: &str) -> Result<String> {
    let value = value.trim();
    let count = value.chars().count();
    if !(2..=80).contains(&count) {
        bail!("{label}长度必须在 2 到 80 个字符之间");
    }
    Ok(value.to_string())
}

pub(crate) fn validate_status(value: &str) -> Result<String> {
    match value.trim() {
        MERCHANT_STATUS_ACTIVE => Ok(MERCHANT_STATUS_ACTIVE.to_string()),
        MERCHANT_STATUS_DISABLED => Ok(MERCHANT_STATUS_DISABLED.to_string()),
        _ => bail!("状态必须是 active 或 disabled"),
    }
}

pub(crate) fn validate_capability_kind(value: &str) -> Result<String> {
    match value.trim() {
        "query" => Ok("query".to_string()),
        "action" => Ok("action".to_string()),
        _ => bail!("能力类型必须是 query 或 action"),
    }
}

pub(crate) fn validate_access_level(value: &str) -> Result<String> {
    match value.trim() {
        ACCESS_PUBLIC => Ok(ACCESS_PUBLIC.to_string()),
        ACCESS_AUTHORIZED => Ok(ACCESS_AUTHORIZED.to_string()),
        ACCESS_OWNER_ONLY => Ok(ACCESS_OWNER_ONLY.to_string()),
        _ => bail!("访问级别必须是 public、authorized 或 owner_only"),
    }
}

pub(crate) fn validate_handler_type(value: &str) -> Result<String> {
    match value.trim() {
        HANDLER_MERCHANT_PROFILE => Ok(HANDLER_MERCHANT_PROFILE.to_string()),
        HANDLER_STATIC_JSON => Ok(HANDLER_STATIC_JSON.to_string()),
        HANDLER_MERCHANT_RUNTIME => Ok(HANDLER_MERCHANT_RUNTIME.to_string()),
        _ => bail!("仅支持 merchant_profile、static_json 或 merchant_runtime 处理器"),
    }
}

pub(crate) fn validate_json_object(value: &Value, label: &str) -> Result<Value> {
    if !value.is_object() {
        return Err(anyhow!("{label}必须是 JSON object"));
    }
    Ok(value.clone())
}

pub(crate) fn empty_object() -> Value {
    Value::Object(Map::new())
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

fn default_node_mode() -> String {
    "platform_hosted".to_string()
}

fn default_capability_kind() -> String {
    "query".to_string()
}

fn default_access_level() -> String {
    ACCESS_PUBLIC.to_string()
}

fn default_currency() -> String {
    "CNY".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_rejects_arbitrary_http_handlers() {
        assert!(validate_handler_type("http").is_err());
        assert_eq!(
            validate_handler_type("static_json").unwrap(),
            HANDLER_STATIC_JSON
        );
    }

    #[test]
    fn identifiers_are_bounded_and_machine_readable() {
        assert_eq!(
            normalize_capability_key("Booking.Preview").unwrap(),
            "booking.preview"
        );
        assert!(normalize_app_id("bad app").is_err());
        assert!(normalize_idempotency_key("a b c").is_err());
    }
}
