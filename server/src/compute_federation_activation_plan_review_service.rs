use anyhow::{bail, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::store::{ComputeActivationPlanReviewReceipt, ReviewComputeActivationPlan, Store};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewComputeActivationPlanBody {
    pub idempotency_key: String,
    pub expected_plan_digest: String,
    pub review_note: Option<String>,
    pub confirm_review: bool,
}

pub(crate) fn review_for_admin(
    store: &Store,
    reviewer_user_id: &str,
    request_id: &str,
    body: ReviewComputeActivationPlanBody,
) -> Result<ComputeActivationPlanReviewReceipt> {
    if !body.confirm_review {
        bail!("复核激活计划前必须显式确认");
    }
    store.review_compute_activation_plan(ReviewComputeActivationPlan {
        request_id: request_id.to_string(),
        expected_plan_digest: body.expected_plan_digest,
        review_note: body.review_note,
        idempotency_scope: idempotency_scope(reviewer_user_id, request_id)?,
        idempotency_key: body.idempotency_key,
        reviewed_by_user_id: reviewer_user_id.to_string(),
    })
}

pub(crate) fn get_for_admin(
    store: &Store,
    request_id: &str,
) -> Result<Option<ComputeActivationPlanReviewReceipt>> {
    store.compute_activation_evidence_request(request_id)?;
    store.compute_activation_plan_review_for_request(request_id)
}

fn idempotency_scope(reviewer_user_id: &str, request_id: &str) -> Result<String> {
    validate_exact("激活计划复核人", reviewer_user_id, 160)?;
    validate_exact("激活证据申请 ID", request_id, 160)?;
    let value = serde_json::json!({
        "purpose":"compute_activation_plan_review",
        "request_id":request_id,
        "reviewed_by_user_id":reviewer_user_id,
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&value)?)))
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
