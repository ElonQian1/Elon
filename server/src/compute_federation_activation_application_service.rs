use anyhow::{bail, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation_activation_application_model::ComputeActivationApplicationReceipt,
    store::{ApplyComputeActivationPlan, Store},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplyComputeActivationPlanBody {
    pub idempotency_key: String,
    pub expected_plan_digest: String,
    pub confirm_apply: bool,
}

pub(crate) fn apply_for_review(
    store: &Store,
    actor_user_id: &str,
    request_id: &str,
    body: ApplyComputeActivationPlanBody,
) -> Result<ComputeActivationApplicationReceipt> {
    if !body.confirm_apply {
        bail!("应用激活计划前必须显式确认");
    }
    validate_exact("激活计划应用人", actor_user_id, 160)?;
    validate_exact("激活证据申请 ID", request_id, 160)?;
    validate_exact("激活计划应用幂等键", &body.idempotency_key, 160)?;
    validate_digest("激活计划摘要", &body.expected_plan_digest)?;

    store.apply_compute_activation_plan(ApplyComputeActivationPlan {
        request_id: request_id.to_string(),
        expected_plan_digest: body.expected_plan_digest,
        idempotency_scope: idempotency_scope(request_id)?,
        idempotency_key: body.idempotency_key,
        applied_by_user_id: actor_user_id.to_string(),
    })
}

pub(crate) fn get_for_review(
    store: &Store,
    request_id: &str,
) -> Result<Option<ComputeActivationApplicationReceipt>> {
    store.compute_activation_evidence_request(request_id)?;
    store.compute_activation_application_for_request(request_id)
}

fn idempotency_scope(request_id: &str) -> Result<String> {
    validate_exact("激活证据申请 ID", request_id, 160)?;
    let value = serde_json::json!({
        "purpose":"compute_activation_plan_apply",
        "request_id":request_id,
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&value)?)))
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{label}必须是 64 位小写十六进制 SHA-256");
    }
    Ok(())
}

fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max_len
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符");
    }
    Ok(())
}
