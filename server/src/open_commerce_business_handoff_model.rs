use anyhow::{bail, Context, Result};
use chrono::DateTime;
use serde::{Deserialize, Serialize};

pub(crate) const BUSINESS_HANDOFF_RECEIPT_SCHEMA: &str =
    "open_commerce.business_handoff_receipt.v1";
pub(crate) const BUSINESS_HANDOFF_LIST_SCHEMA: &str =
    "open_commerce.business_handoff_receipt_list.v1";
pub(crate) const BUSINESS_HANDOFF_AUTHORITY: &str = "project_editor_asserted";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceBusinessHandoffReceipt {
    pub schema: &'static str,
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub invocation_id: String,
    pub integration_id: String,
    pub receipt_key: String,
    pub status: String,
    pub target_domain: String,
    pub evidence_result_sha256: String,
    pub target_reference_sha256: Option<String>,
    pub error_code: Option<String>,
    pub confirmed_by_user: bool,
    pub assertion_authority: String,
    pub recorded_by_user_id: String,
    pub recorded_by_app_id: String,
    pub completed_at: String,
    pub created_at: String,
    pub funds_moved: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceBusinessHandoffReceiptList {
    pub schema: &'static str,
    pub project_id: String,
    pub merchant_id: String,
    pub receipts: Vec<OpenCommerceBusinessHandoffReceipt>,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordBusinessHandoffReceiptRequest {
    pub merchant_id: String,
    pub invocation_id: String,
    pub integration_id: String,
    pub receipt_key: String,
    pub status: String,
    pub target_domain: String,
    pub evidence_result_sha256: String,
    #[serde(default)]
    pub target_reference: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    pub confirmed_by_user: bool,
    pub completed_at: String,
}

pub(crate) fn normalize_handoff_status(value: &str) -> Result<String> {
    match value.trim() {
        "applied" => Ok("applied".to_string()),
        "ignored" => Ok("ignored".to_string()),
        "rejected" => Ok("rejected".to_string()),
        _ => bail!("衔接状态必须是 applied、ignored 或 rejected"),
    }
}

pub(crate) fn normalize_target_domain(value: &str) -> Result<String> {
    match value.trim() {
        "erp" => Ok("erp".to_string()),
        "crm" => Ok("crm".to_string()),
        _ => bail!("衔接目标必须是 erp 或 crm"),
    }
}

pub(crate) fn normalize_handoff_receipt_key(value: &str) -> Result<String> {
    normalize_identifier(value, "衔接回执键", 3, 128)
}

pub(crate) fn normalize_sha256(value: &str, label: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        bail!("{label}必须是 64 位 SHA-256 摘要");
    }
    Ok(value)
}

pub(crate) fn normalize_target_reference(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > 160 || value.chars().any(char::is_control) {
        bail!("目标记录号格式无效");
    }
    Ok(Some(value.to_string()))
}

pub(crate) fn normalize_handoff_error_code(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    Ok(Some(normalize_identifier(value, "衔接结果代码", 2, 96)?))
}

pub(crate) fn normalize_handoff_completed_at(value: &str) -> Result<String> {
    Ok(DateTime::parse_from_rfc3339(value.trim())
        .context("衔接完成时间必须是 RFC3339 时间")?
        .to_rfc3339())
}

fn normalize_identifier(value: &str, label: &str, min: usize, max: usize) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() < min
        || value.len() > max
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | ':')
        })
    {
        bail!("{label}格式无效");
    }
    Ok(value)
}
