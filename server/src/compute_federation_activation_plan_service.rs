use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::provider::{ComputeProviderEndpointRef, PROVIDER_STATUS_ACTIVE},
    compute_federation_activation_model::ACTIVATION_REQUEST_STATUS_APPROVED,
    compute_federation_activation_plan_model::{
        ComputeActivationPlan, ComputeActivationPlanReceipt,
    },
    store::{validate_compute_provider_contract, PrepareComputeActivationPlan, Store},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrepareComputeActivationPlanBody {
    pub idempotency_key: String,
    pub expected_request_digest: String,
    pub endpoint: ComputeProviderEndpointRef,
    pub verified_hardware_digest: String,
    pub trust_tier: String,
    pub verified_at: String,
    pub confirm_prepare: bool,
}

pub(crate) fn prepare_for_review(
    store: &Store,
    actor_user_id: &str,
    request_id: &str,
    body: PrepareComputeActivationPlanBody,
) -> Result<ComputeActivationPlanReceipt> {
    if !body.confirm_prepare {
        bail!("准备激活计划前必须显式确认");
    }
    validate_body(&body)?;
    let request = store.compute_activation_evidence_request(request_id)?;
    if request.status != ACTIVATION_REQUEST_STATUS_APPROVED
        || request.request_digest != body.expected_request_digest
    {
        bail!("只有当前摘要匹配的 approved 激活证据申请可以准备计划");
    }
    crate::compute_federation_activation_service::validate_approval_dependencies(store, &request)?;

    let provider = store.compute_provider(&request.provider_id)?;
    let mut target_provider = provider.provider;
    target_provider.policy_revision = target_provider
        .policy_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("Provider policy revision 溢出"))?;
    target_provider.status = PROVIDER_STATUS_ACTIVE.to_string();
    target_provider.trust_tier = body.trust_tier;
    target_provider.endpoint = Some(body.endpoint);
    target_provider.adapter = None;
    target_provider.evidence_profile.observed_hardware_digest =
        Some(request.hardware_observation_digest.clone());
    target_provider.evidence_profile.last_observed_at = Some(request.requested_at.clone());
    target_provider.evidence_profile.verified_hardware_digest = Some(body.verified_hardware_digest);
    target_provider.evidence_profile.last_verified_at = Some(body.verified_at);
    target_provider.updated_at = request
        .reviewed_at
        .clone()
        .unwrap_or_else(|| request.updated_at.clone());
    validate_compute_provider_contract(&target_provider)?;

    store.prepare_compute_activation_plan(PrepareComputeActivationPlan {
        request_id: request.request_id,
        provider_id: request.provider_id,
        pool_id: request.pool_id,
        expected_request_digest: request.request_digest,
        expected_provider_policy_revision: request.expected_provider_policy_revision,
        expected_provider_digest: request.expected_provider_digest,
        expected_capacity_epoch: request.expected_capacity_epoch,
        expected_pool_revision: request.expected_pool_revision,
        expected_pool_digest: request.expected_pool_digest,
        target_provider,
        idempotency_scope: idempotency_scope(actor_user_id, request_id)?,
        idempotency_key: body.idempotency_key,
        prepared_by_user_id: actor_user_id.to_string(),
    })
}

pub(crate) fn get_for_review(
    store: &Store,
    request_id: &str,
) -> Result<Option<ComputeActivationPlan>> {
    store.compute_activation_evidence_request(request_id)?;
    store.compute_activation_plan_for_request(request_id)
}

fn validate_body(body: &PrepareComputeActivationPlanBody) -> Result<()> {
    validate_exact("激活计划幂等键", &body.idempotency_key, 160)?;
    validate_exact("目标信任层", &body.trust_tier, 80)?;
    if body.trust_tier == "self_declared" {
        bail!("激活计划目标信任层不能仍为 self_declared");
    }
    validate_digest("激活证据申请摘要", &body.expected_request_digest)?;
    validate_digest("verified 硬件摘要", &body.verified_hardware_digest)?;
    let verified_at = DateTime::parse_from_rfc3339(body.verified_at.trim())
        .map_err(|_| anyhow::anyhow!("verified_at 不是 RFC3339 时间"))?;
    if verified_at.offset().local_minus_utc() != 0 {
        bail!("verified_at 必须使用 UTC 时区");
    }
    if verified_at.with_timezone(&Utc) > Utc::now() + Duration::minutes(5) {
        bail!("verified_at 不能明显晚于服务端当前时间");
    }
    Ok(())
}

fn idempotency_scope(actor_user_id: &str, request_id: &str) -> Result<String> {
    validate_exact("激活计划准备人", actor_user_id, 160)?;
    validate_exact("激活证据申请 ID", request_id, 160)?;
    let value = serde_json::json!({
        "purpose":"compute_activation_plan_prepare",
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
