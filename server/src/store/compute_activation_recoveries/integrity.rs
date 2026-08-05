use super::*;
use sha2::{Digest, Sha256};

pub(super) fn recovery_plan_digest(
    quarantine: &StoredQuarantine,
    input: &PrepareComputeActivationRecoveryPlan,
    target_digest: &str,
    routing_digest: &str,
    evidence_refs_digest: &str,
    expected_pool_revision: i64,
    expected_pool_digest: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema": RECOVERY_PLAN_SCHEMA,
        "quarantine_id": quarantine.quarantine_id,
        "application_id": quarantine.application_id,
        "request_id": quarantine.request_id,
        "provider_id": quarantine.provider_id,
        "pool_id": quarantine.pool_id,
        "expected_quarantine_digest": quarantine.quarantine_digest,
        "expected_provider_policy_revision": quarantine.quarantined_provider_policy_revision,
        "expected_provider_digest": quarantine.quarantined_provider_digest,
        "expected_capacity_epoch": quarantine.capacity_epoch,
        "expected_pool_revision": expected_pool_revision,
        "expected_pool_digest": expected_pool_digest,
        "target_provider_policy_revision": input.target_provider.policy_revision,
        "target_provider_digest": target_digest,
        "routing_digest": routing_digest,
        "remediation_summary": input.remediation_summary.trim(),
        "evidence_refs_digest": evidence_refs_digest,
        "prepared_by_user_id": input.prepared_by_user_id.trim(),
    }))
}

pub(super) fn recovery_plan_digest_from_stored(
    stored: &StoredRecoveryPlan,
    input: &PrepareComputeActivationRecoveryPlan,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema": RECOVERY_PLAN_SCHEMA,
        "quarantine_id": stored.quarantine_id,
        "application_id": stored.application_id,
        "request_id": stored.request_id,
        "provider_id": stored.provider_id,
        "pool_id": stored.pool_id,
        "expected_quarantine_digest": stored.expected_quarantine_digest,
        "expected_provider_policy_revision": stored.expected_provider_policy_revision,
        "expected_provider_digest": stored.expected_provider_digest,
        "expected_capacity_epoch": stored.expected_capacity_epoch,
        "expected_pool_revision": stored.expected_pool_revision,
        "expected_pool_digest": stored.expected_pool_digest,
        "target_provider_policy_revision": stored.target_provider_policy_revision,
        "target_provider_digest": stored.target_provider_digest,
        "routing_digest": stored.routing_digest,
        "remediation_summary": input.remediation_summary.trim(),
        "evidence_refs_digest": stored.evidence_refs_digest,
        "prepared_by_user_id": stored.prepared_by_user_id,
    }))
}

pub(super) fn review_request_digest(input: &ReviewComputeActivationRecoveryPlan) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema": "compute_federation.activation_recovery_review_request.v1",
        "request_id": input.request_id,
        "expected_plan_digest": input.expected_plan_digest,
        "review_note": normalize_note(input.review_note.clone())?,
        "reviewed_by_user_id": input.reviewed_by_user_id,
    }))
}

pub(super) fn recovery_review_digest(
    id: &str,
    plan: &ComputeActivationRecoveryPlan,
    input: &ReviewComputeActivationRecoveryPlan,
    request_digest: &str,
    reviewed_at: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema": RECOVERY_REVIEW_SCHEMA,
        "recovery_review_id": id,
        "recovery_plan_id": plan.recovery_plan_id,
        "request_id": plan.request_id,
        "plan_digest": plan.plan_digest,
        "prepared_by_user_id": plan.prepared_by_user_id,
        "reviewed_by_user_id": input.reviewed_by_user_id,
        "review_note": normalize_note(input.review_note.clone())?,
        "request_digest": request_digest,
        "reviewed_at": reviewed_at,
    }))
}

pub(super) fn recovery_application_digest(
    id: &str,
    plan: &ComputeActivationRecoveryPlan,
    review: &StoredRecoveryReview,
    lifecycle_event_id: &str,
    actor: &str,
    applied_at: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema": RECOVERY_APPLICATION_SCHEMA,
        "recovery_application_id": id,
        "recovery_plan_id": plan.recovery_plan_id,
        "recovery_review_id": review.recovery_review_id,
        "quarantine_id": plan.quarantine_id,
        "request_id": plan.request_id,
        "provider_id": plan.provider_id,
        "pool_id": plan.pool_id,
        "plan_digest": plan.plan_digest,
        "review_digest": review.review_digest,
        "recovered_provider_policy_revision": plan.target_provider_policy_revision,
        "recovered_provider_digest": plan.target_provider_digest,
        "capacity_epoch": plan.expected_capacity_epoch,
        "pool_lifecycle_event_id": lifecycle_event_id,
        "applied_by_user_id": actor,
        "applied_at": applied_at,
    }))
}

pub(super) fn ensure_plan_replay(
    plan: &ComputeActivationRecoveryPlan,
    expected_digest: &str,
) -> Result<()> {
    if plan.plan_digest != expected_digest {
        bail!("恢复计划幂等键或隔离回执已绑定不同目标")
    }
    Ok(())
}

pub(super) fn ensure_review_replay(
    conn: &Connection,
    review: &StoredRecoveryReview,
    request_digest: &str,
) -> Result<()> {
    if review.request_digest != request_digest {
        bail!("恢复复核幂等键或计划已绑定不同请求")
    }
    audit_recovery_review_on(conn, review)
}

pub(super) fn ensure_application_replay(
    conn: &Connection,
    application: &StoredRecoveryApplication,
    input: &ApplyComputeActivationRecoveryPlan,
) -> Result<()> {
    if application.request_id != input.request_id.trim()
        || application.plan_digest != input.expected_plan_digest.trim()
    {
        bail!("恢复应用幂等键已绑定不同计划")
    }
    audit_recovery_application_on(conn, application)
}

pub(super) fn validate_prepare_input(input: &PrepareComputeActivationRecoveryPlan) -> Result<()> {
    for (label, value, max) in [
        ("申请 ID", input.request_id.as_str(), 160),
        ("修复说明", input.remediation_summary.as_str(), 1000),
        ("幂等范围", input.idempotency_scope.as_str(), 200),
        ("幂等键", input.idempotency_key.as_str(), 160),
        ("准备人", input.prepared_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max)?
    }
    validate_digest("隔离摘要", &input.expected_quarantine_digest)?;
    validate_compute_provider_contract(&input.target_provider)
}

pub(super) fn validate_review_input(input: &ReviewComputeActivationRecoveryPlan) -> Result<()> {
    for (label, value, max) in [
        ("申请 ID", input.request_id.as_str(), 160),
        ("幂等范围", input.idempotency_scope.as_str(), 200),
        ("幂等键", input.idempotency_key.as_str(), 160),
        ("复核人", input.reviewed_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max)?
    }
    validate_digest("恢复计划摘要", &input.expected_plan_digest)?;
    normalize_note(input.review_note.clone()).map(|_| ())
}

pub(super) fn validate_apply_input(input: &ApplyComputeActivationRecoveryPlan) -> Result<()> {
    for (label, value, max) in [
        ("申请 ID", input.request_id.as_str(), 160),
        ("幂等范围", input.idempotency_scope.as_str(), 200),
        ("幂等键", input.idempotency_key.as_str(), 160),
        ("应用人", input.applied_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max)?
    }
    validate_digest("恢复计划摘要", &input.expected_plan_digest)
}

pub(super) fn normalize_refs(values: Vec<String>) -> Result<Vec<String>> {
    if values.is_empty() || values.len() > 20 {
        bail!("恢复证据引用必须为 1 至 20 项")
    }
    let mut normalized = Vec::new();
    for value in values {
        validate_exact("恢复证据引用", &value, 500)?;
        if !normalized.contains(&value) {
            normalized.push(value)
        }
    }
    normalized.sort();
    Ok(normalized)
}

pub(super) fn normalize_note(value: Option<String>) -> Result<Option<String>> {
    value
        .map(|value| {
            let value = value.trim().to_string();
            validate_exact("复核说明", &value, 1000)?;
            Ok(value)
        })
        .transpose()
}

pub(super) fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{label}必须是 64 位小写十六进制 SHA-256")
    }
    Ok(())
}

pub(super) fn validate_exact(label: &str, value: &str, max: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符")
    }
    Ok(())
}

pub(super) fn digest_json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(digest_bytes(&serde_json::to_vec(value)?))
}

pub(super) fn digest_bytes(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}
