use anyhow::{bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolStatus, provider::PROVIDER_STATUS_REGISTERING,
    },
    compute_federation_activation_model::{
        ComputeActivationEvidenceRequest, ComputeActivationEvidenceRequestReceipt,
        ACTIVATION_REQUEST_STATUS_APPROVED,
    },
    store::{
        stable_compute_capacity_pool_audit_digest, ComputeCapacityPoolAuditReport,
        ReviewComputeActivationEvidenceRequest, Store, SubmitComputeActivationEvidenceRequest,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitMyComputeActivationEvidenceRequest {
    pub idempotency_key: String,
    pub node_binding_ref: String,
    pub ready_capability_digest: String,
    pub route_proof_digest: String,
    pub hardware_observation_digest: String,
    pub confirm_evidence_submission: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CancelMyComputeActivationEvidenceRequest {
    pub expected_request_digest: String,
    pub confirm_cancel: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewComputeActivationEvidenceRequestBody {
    pub expected_request_digest: String,
    pub decision: String,
    #[serde(default)]
    pub review_note: Option<String>,
    pub confirm_review: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationEvidenceReviewReceipt {
    pub request: ComputeActivationEvidenceRequest,
    pub activation_effect: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationPreflightReport {
    pub schema: &'static str,
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub request_status: String,
    pub checked_at: String,
    pub provider_ownership_matches: bool,
    pub provider_version_matches: bool,
    pub provider_status_registering: bool,
    pub provider_has_routing: bool,
    pub provider_has_verified_hardware: bool,
    pub provider_has_verified_at: bool,
    pub provider_trust_tier: String,
    pub provider_regions_present: bool,
    pub pool_provider_matches: bool,
    pub pool_version_matches: bool,
    pub pool_status_registering: bool,
    pub ledger_audit_healthy: bool,
    pub ledger_audit_digest_matches: bool,
    pub ready_for_activation: bool,
    pub blockers: Vec<String>,
    pub activation_effect: &'static str,
}

pub(crate) fn submit_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    request: SubmitMyComputeActivationEvidenceRequest,
) -> Result<ComputeActivationEvidenceRequestReceipt> {
    if !request.confirm_evidence_submission {
        bail!("提交激活证据申请前必须显式确认");
    }
    let idempotency_scope = idempotency_scope(user_id, provider_id, pool_id)?;
    if let Some(existing) = store.compute_activation_evidence_request_by_idempotency(
        &idempotency_scope,
        &request.idempotency_key,
    )? {
        ensure_replay_matches(&existing, user_id, provider_id, pool_id, &request)?;
        return Ok(ComputeActivationEvidenceRequestReceipt {
            request: existing,
            replayed: true,
            activation_effect: "none",
        });
    }

    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    if provider.provider.status != PROVIDER_STATUS_REGISTERING {
        bail!("只有 registering Provider 可以提交激活证据申请");
    }
    let pool = crate::compute_federation_capacity_pool_service::owned_pool_for_user(
        store,
        user_id,
        provider_id,
        pool_id,
    )?;
    if pool.status != ComputeCapacityPoolStatus::Registering {
        bail!("只有 registering CapacityPool 可以提交激活证据申请");
    }
    let audit = healthy_current_audit(store, pool_id, pool.binding.capacity_epoch)?;
    let ledger_audit_digest = stable_compute_capacity_pool_audit_digest(&audit)?;

    store.submit_compute_activation_evidence_request(SubmitComputeActivationEvidenceRequest {
        provider_id: provider_id.to_string(),
        pool_id: pool_id.to_string(),
        owner_user_id: user_id.to_string(),
        expected_provider_policy_revision: provider.provider.policy_revision,
        expected_provider_digest: provider.provider_digest,
        expected_capacity_epoch: pool.binding.capacity_epoch,
        expected_pool_revision: pool.binding.pool_revision,
        expected_pool_digest: pool.binding.pool_digest,
        node_binding_ref: request.node_binding_ref,
        ready_capability_digest: request.ready_capability_digest,
        route_proof_digest: request.route_proof_digest,
        hardware_observation_digest: request.hardware_observation_digest,
        ledger_audit_digest,
        idempotency_scope,
        idempotency_key: request.idempotency_key,
    })
}

pub(crate) fn get_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    request_id: &str,
) -> Result<ComputeActivationEvidenceRequest> {
    let request = store.compute_activation_evidence_request(request_id)?;
    ensure_owned_request(&request, user_id, provider_id, pool_id)?;
    Ok(request)
}

pub(crate) fn list_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    limit: usize,
) -> Result<Vec<ComputeActivationEvidenceRequest>> {
    crate::compute_federation_capacity_pool_service::owned_pool_for_user(
        store,
        user_id,
        provider_id,
        pool_id,
    )?;
    store.list_compute_activation_evidence_requests_for_owner(user_id, provider_id, pool_id, limit)
}

pub(crate) fn cancel_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    request_id: &str,
    request: CancelMyComputeActivationEvidenceRequest,
) -> Result<ComputeActivationEvidenceRequest> {
    if !request.confirm_cancel {
        bail!("取消激活证据申请前必须显式确认");
    }
    get_for_user(store, user_id, provider_id, pool_id, request_id)?;
    store.cancel_compute_activation_evidence_request(
        user_id,
        request_id,
        &request.expected_request_digest,
    )
}

pub(crate) fn list_for_review(
    store: &Store,
    status: &str,
    limit: usize,
) -> Result<Vec<ComputeActivationEvidenceRequest>> {
    store.list_reviewable_compute_activation_evidence_requests(status, limit)
}

pub(crate) fn preflight_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    request_id: &str,
) -> Result<ComputeActivationPreflightReport> {
    let request = get_for_user(store, user_id, provider_id, pool_id, request_id)?;
    activation_preflight(store, &request)
}

pub(crate) fn preflight_for_review(
    store: &Store,
    request_id: &str,
) -> Result<ComputeActivationPreflightReport> {
    let request = store.compute_activation_evidence_request(request_id)?;
    activation_preflight(store, &request)
}

pub(crate) fn review(
    store: &Store,
    reviewer_user_id: &str,
    request_id: &str,
    body: ReviewComputeActivationEvidenceRequestBody,
) -> Result<ComputeActivationEvidenceReviewReceipt> {
    if !body.confirm_review {
        bail!("审核激活证据申请前必须显式确认");
    }
    let current = store.compute_activation_evidence_request(request_id)?;
    if current.request_digest != body.expected_request_digest {
        bail!("激活证据申请内容已变化，请刷新后重试");
    }
    if body.decision == ACTIVATION_REQUEST_STATUS_APPROVED {
        validate_approval_dependencies(store, &current)?;
    }
    let request = store.review_compute_activation_evidence_request(
        ReviewComputeActivationEvidenceRequest {
            request_id: request_id.to_string(),
            expected_request_digest: body.expected_request_digest,
            reviewer_user_id: reviewer_user_id.to_string(),
            decision: body.decision,
            review_note: body.review_note,
        },
    )?;
    Ok(ComputeActivationEvidenceReviewReceipt {
        request,
        activation_effect: "none",
    })
}

pub(crate) fn validate_approval_dependencies(
    store: &Store,
    request: &ComputeActivationEvidenceRequest,
) -> Result<()> {
    let provider = store.compute_provider(&request.provider_id)?;
    if provider.provider.owner_account_id != request.owner_user_id
        || provider.provider.status != PROVIDER_STATUS_REGISTERING
        || provider.provider.policy_revision != request.expected_provider_policy_revision
        || provider.provider_digest != request.expected_provider_digest
    {
        bail!("Provider 所有权、状态或版本已变化，不能批准当前证据申请");
    }
    let pool = store.compute_capacity_pool(&request.pool_id)?;
    if pool.provider_id != request.provider_id
        || pool.status != ComputeCapacityPoolStatus::Registering
        || pool.binding.capacity_epoch != request.expected_capacity_epoch
        || pool.binding.pool_revision != request.expected_pool_revision
        || pool.binding.pool_digest != request.expected_pool_digest
    {
        bail!("CapacityPool 归属、状态或版本已变化，不能批准当前证据申请");
    }
    let audit = healthy_current_audit(store, &request.pool_id, request.expected_capacity_epoch)?;
    if stable_compute_capacity_pool_audit_digest(&audit)? != request.ledger_audit_digest {
        bail!("CapacityPool 账本审计结果已变化，不能批准当前证据申请");
    }
    Ok(())
}

fn activation_preflight(
    store: &Store,
    request: &ComputeActivationEvidenceRequest,
) -> Result<ComputeActivationPreflightReport> {
    let provider = store.compute_provider(&request.provider_id)?;
    let pool = store.compute_capacity_pool(&request.pool_id)?;
    let audit = store
        .audit_compute_capacity_pool_epoch(&request.pool_id, request.expected_capacity_epoch)?;

    let request_approved = request.status == ACTIVATION_REQUEST_STATUS_APPROVED;
    let provider_ownership_matches = provider.provider.owner_account_id == request.owner_user_id;
    let provider_version_matches = provider.provider.policy_revision
        == request.expected_provider_policy_revision
        && provider.provider_digest == request.expected_provider_digest;
    let provider_status_registering = provider.provider.status == PROVIDER_STATUS_REGISTERING;
    let provider_has_routing =
        provider.provider.endpoint.is_some() || provider.provider.adapter.is_some();
    let provider_has_verified_hardware = provider
        .provider
        .evidence_profile
        .verified_hardware_digest
        .is_some();
    let provider_has_verified_at = provider
        .provider
        .evidence_profile
        .last_verified_at
        .is_some();
    let provider_trust_tier_ready = provider.provider.trust_tier != "self_declared";
    let provider_regions_present = !provider.provider.capabilities.regions.is_empty();
    let pool_provider_matches = pool.provider_id == request.provider_id;
    let pool_version_matches = pool.binding.capacity_epoch == request.expected_capacity_epoch
        && pool.binding.pool_revision == request.expected_pool_revision
        && pool.binding.pool_digest == request.expected_pool_digest;
    let pool_status_registering = pool.status == ComputeCapacityPoolStatus::Registering;
    let ledger_audit_healthy =
        audit.healthy && audit.current_capacity_epoch == request.expected_capacity_epoch;
    let ledger_audit_digest_matches =
        stable_compute_capacity_pool_audit_digest(&audit)? == request.ledger_audit_digest;

    let mut blockers = Vec::new();
    block_unless(&mut blockers, request_approved, "request_not_approved");
    block_unless(
        &mut blockers,
        provider_ownership_matches,
        "provider_ownership_changed",
    );
    block_unless(
        &mut blockers,
        provider_version_matches,
        "provider_version_changed",
    );
    block_unless(
        &mut blockers,
        provider_status_registering,
        "provider_not_registering",
    );
    block_unless(
        &mut blockers,
        provider_has_routing,
        "provider_routing_missing",
    );
    block_unless(
        &mut blockers,
        provider_has_verified_hardware,
        "verified_hardware_missing",
    );
    block_unless(
        &mut blockers,
        provider_has_verified_at,
        "verified_timestamp_missing",
    );
    block_unless(
        &mut blockers,
        provider_trust_tier_ready,
        "provider_trust_tier_self_declared",
    );
    block_unless(
        &mut blockers,
        provider_regions_present,
        "provider_regions_missing",
    );
    block_unless(
        &mut blockers,
        pool_provider_matches,
        "pool_provider_changed",
    );
    block_unless(&mut blockers, pool_version_matches, "pool_version_changed");
    block_unless(
        &mut blockers,
        pool_status_registering,
        "pool_not_registering",
    );
    block_unless(
        &mut blockers,
        ledger_audit_healthy,
        "ledger_audit_unhealthy",
    );
    block_unless(
        &mut blockers,
        ledger_audit_digest_matches,
        "ledger_audit_changed",
    );

    Ok(ComputeActivationPreflightReport {
        schema: "compute_federation.activation_preflight.v1",
        request_id: request.request_id.clone(),
        provider_id: request.provider_id.clone(),
        pool_id: request.pool_id.clone(),
        request_status: request.status.clone(),
        checked_at: Utc::now().to_rfc3339(),
        provider_ownership_matches,
        provider_version_matches,
        provider_status_registering,
        provider_has_routing,
        provider_has_verified_hardware,
        provider_has_verified_at,
        provider_trust_tier: provider.provider.trust_tier,
        provider_regions_present,
        pool_provider_matches,
        pool_version_matches,
        pool_status_registering,
        ledger_audit_healthy,
        ledger_audit_digest_matches,
        ready_for_activation: blockers.is_empty(),
        blockers,
        activation_effect: "none",
    })
}

fn block_unless(blockers: &mut Vec<String>, condition: bool, code: &str) {
    if !condition {
        blockers.push(code.to_string());
    }
}

fn healthy_current_audit(
    store: &Store,
    pool_id: &str,
    expected_epoch: i64,
) -> Result<ComputeCapacityPoolAuditReport> {
    let audit = store.audit_compute_capacity_pool_epoch(pool_id, expected_epoch)?;
    if !audit.healthy || audit.current_capacity_epoch != expected_epoch {
        bail!("CapacityPool 当前 epoch 的账本审计未通过");
    }
    Ok(audit)
}

fn idempotency_scope(user_id: &str, provider_id: &str, pool_id: &str) -> Result<String> {
    for (label, value) in [
        ("用户 ID", user_id),
        ("Provider ID", provider_id),
        ("CapacityPool ID", pool_id),
    ] {
        if value.trim().is_empty() || value != value.trim() {
            bail!("{label}不能为空或包含首尾空白");
        }
    }
    digest_value(&serde_json::json!({
        "purpose":"compute_activation_evidence_submission",
        "owner_user_id":user_id,
        "provider_id":provider_id,
        "pool_id":pool_id,
    }))
}

fn ensure_replay_matches(
    existing: &ComputeActivationEvidenceRequest,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    request: &SubmitMyComputeActivationEvidenceRequest,
) -> Result<()> {
    if existing.owner_user_id != user_id
        || existing.provider_id != provider_id
        || existing.pool_id != pool_id
        || existing.node_binding_ref != request.node_binding_ref
        || existing.ready_capability_digest != request.ready_capability_digest
        || existing.route_proof_digest != request.route_proof_digest
        || existing.hardware_observation_digest != request.hardware_observation_digest
    {
        bail!("相同激活证据申请幂等键不能用于不同请求");
    }
    Ok(())
}

fn ensure_owned_request(
    request: &ComputeActivationEvidenceRequest,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
) -> Result<()> {
    if request.owner_user_id != user_id
        || request.provider_id != provider_id
        || request.pool_id != pool_id
    {
        bail!("激活证据申请不属于当前用户指定的 Provider/CapacityPool");
    }
    Ok(())
}

fn digest_value(value: &Value) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}
