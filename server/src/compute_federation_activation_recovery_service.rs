use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::provider::{
        ComputeProviderAdapterRef, ComputeProviderEndpointRef, PROVIDER_STATUS_ACTIVE,
    },
    compute_federation_activation_recovery_model::{
        ComputeActivationRecoveryApplicationReceipt, ComputeActivationRecoveryPlanReceipt,
        ComputeActivationRecoveryReviewReceipt,
    },
    store::{
        ApplyComputeActivationRecoveryPlan, PrepareComputeActivationRecoveryPlan,
        ReviewComputeActivationRecoveryPlan, Store,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrepareActivationRecoveryPlanBody {
    pub idempotency_key: String,
    pub expected_quarantine_digest: String,
    pub endpoint: Option<ComputeProviderEndpointRef>,
    pub adapter: Option<ComputeProviderAdapterRef>,
    pub verified_hardware_digest: String,
    pub trust_tier: String,
    pub verified_at: String,
    pub remediation_summary: String,
    pub evidence_refs: Vec<String>,
    pub confirm_prepare: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewActivationRecoveryPlanBody {
    pub idempotency_key: String,
    pub expected_plan_digest: String,
    pub review_note: Option<String>,
    pub confirm_review: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplyActivationRecoveryPlanBody {
    pub idempotency_key: String,
    pub expected_plan_digest: String,
    pub confirm_apply: bool,
}

pub(crate) fn prepare(
    store: &Store,
    actor: &str,
    request_id: &str,
    body: PrepareActivationRecoveryPlanBody,
) -> Result<ComputeActivationRecoveryPlanReceipt> {
    if !body.confirm_prepare {
        bail!("准备隔离恢复计划前必须显式确认")
    }
    validate_exact("恢复计划准备人", actor, 160)?;
    validate_exact("申请 ID", request_id, 160)?;
    validate_exact("幂等键", &body.idempotency_key, 160)?;
    validate_digest("隔离摘要", &body.expected_quarantine_digest)?;
    validate_digest("verified 硬件摘要", &body.verified_hardware_digest)?;
    validate_exact("信任层", &body.trust_tier, 80)?;
    if body.trust_tier == "self_declared" {
        bail!("恢复目标不能使用 self_declared 信任层")
    }
    let verified_at = DateTime::parse_from_rfc3339(body.verified_at.trim())
        .map_err(|_| anyhow::anyhow!("verified_at 不是 RFC3339 时间"))?;
    if verified_at.offset().local_minus_utc() != 0
        || verified_at.with_timezone(&Utc) > Utc::now() + Duration::minutes(5)
    {
        bail!("verified_at 必须为合理的 UTC 时间")
    }
    let quarantine = store
        .compute_activation_quarantine_for_request(request_id)?
        .ok_or_else(|| anyhow::anyhow!("激活隔离回执不存在"))?;
    if quarantine.quarantine_digest != body.expected_quarantine_digest {
        bail!("隔离摘要已变化")
    }
    let current = store.compute_provider(&quarantine.provider_id)?.provider;
    let current_updated_at = DateTime::parse_from_rfc3339(current.updated_at.trim())
        .map_err(|_| anyhow::anyhow!("当前 Provider updated_at 不是 RFC3339 时间"))?;
    let target_updated_at = if verified_at >= current_updated_at {
        body.verified_at.clone()
    } else {
        current.updated_at.clone()
    };
    let mut target = current.clone();
    target.policy_revision = target
        .policy_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("Provider revision 溢出"))?;
    target.status = PROVIDER_STATUS_ACTIVE.to_string();
    target.trust_tier = body.trust_tier;
    target.endpoint = body.endpoint.or(current.endpoint);
    target.adapter = body.adapter.or(current.adapter);
    target.evidence_profile.verified_hardware_digest = Some(body.verified_hardware_digest);
    target.evidence_profile.last_verified_at = Some(body.verified_at);
    target.updated_at = target_updated_at;
    store.prepare_compute_activation_recovery_plan(PrepareComputeActivationRecoveryPlan {
        request_id: request_id.to_string(),
        expected_quarantine_digest: body.expected_quarantine_digest,
        target_provider: target,
        remediation_summary: body.remediation_summary,
        evidence_refs: body.evidence_refs,
        idempotency_scope: scope("prepare", actor, request_id)?,
        idempotency_key: body.idempotency_key,
        prepared_by_user_id: actor.to_string(),
    })
}
pub(crate) fn review(
    store: &Store,
    actor: &str,
    request_id: &str,
    body: ReviewActivationRecoveryPlanBody,
) -> Result<ComputeActivationRecoveryReviewReceipt> {
    if !body.confirm_review {
        bail!("复核隔离恢复计划前必须显式确认")
    };
    store.review_compute_activation_recovery_plan(ReviewComputeActivationRecoveryPlan {
        request_id: request_id.to_string(),
        expected_plan_digest: body.expected_plan_digest,
        review_note: body.review_note,
        idempotency_scope: scope("review", actor, request_id)?,
        idempotency_key: body.idempotency_key,
        reviewed_by_user_id: actor.to_string(),
    })
}
pub(crate) fn apply(
    store: &Store,
    actor: &str,
    request_id: &str,
    body: ApplyActivationRecoveryPlanBody,
) -> Result<ComputeActivationRecoveryApplicationReceipt> {
    if !body.confirm_apply {
        bail!("应用隔离恢复计划前必须显式确认")
    };
    store.apply_compute_activation_recovery_plan(ApplyComputeActivationRecoveryPlan {
        request_id: request_id.to_string(),
        expected_plan_digest: body.expected_plan_digest,
        idempotency_scope: scope("apply", actor, request_id)?,
        idempotency_key: body.idempotency_key,
        applied_by_user_id: actor.to_string(),
    })
}
fn scope(purpose: &str, actor: &str, request_id: &str) -> Result<String> {
    validate_exact("执行人", actor, 160)?;
    validate_exact("申请 ID", request_id, 160)?;
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(
        &serde_json::json!({"purpose":format!("compute_activation_recovery_{purpose}"),"actor":actor,"request_id":request_id}),
    )?)))
}
fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|b| b.is_ascii_hexdigit())
    {
        bail!("{label}必须是 64 位小写十六进制 SHA-256")
    };
    Ok(())
}
fn validate_exact(label: &str, value: &str, max: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符")
    };
    Ok(())
}
