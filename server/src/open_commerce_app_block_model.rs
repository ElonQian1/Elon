use std::{error::Error, fmt};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub(crate) const APP_BLOCK_STATUS_ACTIVE: &str = "active";
pub(crate) const APP_BLOCK_STATUS_UNBLOCKED: &str = "unblocked";

const APP_BLOCK_REASON_CODES: [&str; 5] = [
    "abusive_traffic",
    "policy_violation",
    "security_incident",
    "merchant_request",
    "other",
];

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAppBlock {
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub requester_app_id: String,
    pub reason_code: String,
    pub reason_note: String,
    pub status: String,
    pub blocked_by_user_id: String,
    pub unblocked_by_user_id: Option<String>,
    pub blocked_at: String,
    pub unblocked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAppBlockOutcome {
    pub block: OpenCommerceAppBlock,
    pub revoked_grants: usize,
    pub canceled_authorization_requests: usize,
    pub grants_restored: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BlockOpenCommerceAppRequest {
    pub merchant_id: String,
    pub requester_app_id: String,
    pub reason_code: String,
    #[serde(default)]
    pub reason_note: String,
}

#[derive(Debug)]
pub(crate) struct OpenCommerceAppBlocked {
    pub requester_app_id: String,
}

impl fmt::Display for OpenCommerceAppBlocked {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "开发者应用 {} 已被该商户封禁；解除封禁后仍需重新申请授权",
            self.requester_app_id
        )
    }
}

impl Error for OpenCommerceAppBlocked {}

pub(crate) fn normalize_app_block_reason(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if !APP_BLOCK_REASON_CODES.contains(&value.as_str()) {
        bail!("封禁原因必须是 abusive_traffic、policy_violation、security_incident、merchant_request 或 other");
    }
    Ok(value)
}

pub(crate) fn normalize_app_block_note(value: &str) -> Result<String> {
    let value = value.trim();
    if value.chars().count() > 500 {
        bail!("封禁说明不能超过 500 个字符");
    }
    Ok(value.to_string())
}
