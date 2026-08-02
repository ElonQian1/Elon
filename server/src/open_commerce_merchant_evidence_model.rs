use anyhow::{bail, Context, Result};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const BUSINESS_RECEIPT_FIELD: &str = "_yilong_business_receipt";
pub(crate) const BUSINESS_RECEIPT_SCHEMA: &str = "open_commerce.merchant_business_receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct MerchantBusinessReceipt {
    pub schema: String,
    pub entity_type: String,
    pub reference_id: String,
    pub state: String,
    pub occurred_at: String,
    #[serde(default)]
    pub amount_minor: Option<i64>,
    #[serde(default)]
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MerchantEvidenceErpBinding {
    pub instance_id: String,
    pub instance_key: String,
    pub configuration_revision: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MerchantBusinessEvidenceSummary {
    pub schema: &'static str,
    pub sequence: i64,
    pub invocation_id: String,
    pub merchant_id: String,
    pub erp_binding: Option<MerchantEvidenceErpBinding>,
    pub capability_key: String,
    pub capability_kind: String,
    pub requester_app_id: String,
    pub status: String,
    pub source_authority: &'static str,
    pub receipt_state: &'static str,
    pub business_receipt: Option<MerchantBusinessReceipt>,
    pub result_available: bool,
    pub result_sha256: Option<String>,
    pub error_code: Option<String>,
    pub amount_micros: i64,
    pub currency: String,
    pub settlement_status: String,
    pub funds_moved: bool,
    pub created_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MerchantBusinessEvidenceList {
    pub schema: &'static str,
    pub project_id: String,
    pub merchant_id: String,
    pub erp_binding: Option<MerchantEvidenceErpBinding>,
    pub evidence: Vec<MerchantBusinessEvidenceSummary>,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MerchantBusinessEvidenceDetail {
    pub schema: &'static str,
    pub evidence: MerchantBusinessEvidenceSummary,
    pub result: Option<Value>,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub(crate) struct MerchantTerminalInvocationRecord {
    pub sequence: i64,
    pub invocation: crate::open_commerce_model::OpenCommerceInvocation,
}

pub(crate) fn validate_optional_business_receipt(
    result: &Value,
) -> Result<Option<MerchantBusinessReceipt>> {
    let Some(raw) = result
        .as_object()
        .and_then(|object| object.get(BUSINESS_RECEIPT_FIELD))
    else {
        return Ok(None);
    };
    let receipt: MerchantBusinessReceipt =
        serde_json::from_value(raw.clone()).context("商户业务回执格式无效")?;
    if receipt.schema != BUSINESS_RECEIPT_SCHEMA {
        bail!("商户业务回执版本不受支持");
    }
    validate_token(&receipt.entity_type, "业务实体类型", 64)?;
    validate_reference(&receipt.reference_id)?;
    validate_token(&receipt.state, "业务状态", 64)?;
    DateTime::parse_from_rfc3339(&receipt.occurred_at)
        .context("商户业务回执 occurred_at 必须是 RFC3339 时间")?;
    match (receipt.amount_minor, receipt.currency.as_deref()) {
        (None, None) => {}
        (Some(amount), Some(currency)) => {
            if amount < 0 {
                bail!("商户业务回执金额不能为负数");
            }
            if !(3..=8).contains(&currency.len())
                || !currency
                    .chars()
                    .all(|character| character.is_ascii_uppercase())
            {
                bail!("商户业务回执币种格式无效");
            }
        }
        _ => bail!("商户业务回执金额与币种必须同时提供"),
    }
    Ok(Some(receipt))
}

fn validate_token(value: &str, label: &str, max_len: usize) -> Result<()> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > max_len
        || !value.chars().enumerate().all(|(index, character)| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || (index > 0 && matches!(character, '.' | '_' | '-'))
        })
    {
        bail!("{label}格式无效");
    }
    Ok(())
}

fn validate_reference(value: &str) -> Result<()> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 160
        || value.chars().any(|character| character.is_control())
    {
        bail!("商户业务回执 reference_id 格式无效");
    }
    Ok(())
}
