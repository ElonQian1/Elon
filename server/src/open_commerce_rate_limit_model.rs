use std::{error::Error, fmt};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub(crate) const RATE_LIMIT_STATUS_ACTIVE: &str = "active";
pub(crate) const RATE_LIMIT_STATUS_DISABLED: &str = "disabled";
pub(crate) const RATE_LIMIT_WILDCARD_APP: &str = "*";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceRateLimitPolicy {
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub capability_id: String,
    pub capability_key: String,
    pub requester_app_id: Option<String>,
    pub window_seconds: i64,
    pub max_requests: i64,
    pub status: String,
    pub created_by_user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceRateLimitUsage {
    pub policy_id: String,
    pub window_started_at_unix: i64,
    pub accepted_requests: i64,
    pub active_subjects: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceRateLimitDecision {
    pub policy_id: String,
    pub window_seconds: i64,
    pub max_requests: i64,
    pub used_requests: i64,
    pub remaining_requests: i64,
    pub reset_at_unix: i64,
    pub allowed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpsertOpenCommerceRateLimitRequest {
    pub merchant_id: String,
    pub capability_key: String,
    #[serde(default)]
    pub requester_app_id: Option<String>,
    pub window_seconds: i64,
    pub max_requests: i64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SetOpenCommerceRateLimitEnabledRequest {
    pub enabled: bool,
}

#[derive(Debug)]
pub(crate) struct OpenCommerceRateLimitExceeded {
    pub retry_after_seconds: i64,
    pub max_requests: i64,
    pub window_seconds: i64,
}

impl fmt::Display for OpenCommerceRateLimitExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "当前商业能力调用过于频繁，请在 {} 秒后重试（每 {} 秒最多 {} 次）",
            self.retry_after_seconds, self.window_seconds, self.max_requests
        )
    }
}

impl Error for OpenCommerceRateLimitExceeded {}

pub(crate) fn validate_rate_limit_bounds(window_seconds: i64, max_requests: i64) -> Result<()> {
    if !(1..=86_400).contains(&window_seconds) {
        bail!("限流时间窗必须在 1 到 86400 秒之间");
    }
    if !(1..=1_000_000).contains(&max_requests) {
        bail!("时间窗调用上限必须在 1 到 1000000 次之间");
    }
    Ok(())
}

fn default_enabled() -> bool {
    true
}
