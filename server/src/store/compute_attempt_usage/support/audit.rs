use anyhow::{bail, Result};

use super::{
    build_readings, contract_digest, declaration_request_digest, digest_json,
    normalize_declaration, overage_meters, usage_digest, StoredUsageDeclaration,
};
use crate::store::compute_attempt_usage::{
    ComputeDeclaredUsageInput, DeclareComputeAttemptUsageRequest,
    COMPUTE_ATTEMPT_USAGE_DECLARATION_SCHEMA,
};

pub(super) fn audit_declaration(stored: &StoredUsageDeclaration) -> Result<()> {
    if stored.source_lease_status != "running"
        || stored.sequence_no <= 0
        || stored.created_at != stored.declared_at
        || stored.cumulative_usage_digest != usage_digest(&stored.cumulative_usage)?
        || stored.reserved_contract_digest != contract_digest(&stored.reserved_contract)?
    {
        bail!("Attempt 用量声明基础字段审计失败");
    }
    let request = DeclareComputeAttemptUsageRequest {
        lease_id: stored.lease_id.clone(),
        provider_id: stored.provider_id.clone(),
        expected_lease_revision: stored.source_lease_revision,
        expected_lease_digest: stored.source_lease_digest.clone(),
        expected_fencing_generation: stored.fencing_generation,
        sequence_no: stored.sequence_no,
        executor_usage_ref: stored.executor_usage_ref.clone(),
        cumulative_declared_usage: stored
            .cumulative_usage
            .iter()
            .map(|reading| ComputeDeclaredUsageInput {
                meter: reading.meter.clone(),
                cumulative_quantity: reading.quantity,
            })
            .collect(),
        idempotency_key: stored.idempotency_key.clone(),
        declared_by_user_id: stored.declared_by_user_id.clone(),
    };
    let request = normalize_declaration(&request)?;
    if stored.idempotency_scope != format!("compute_attempt_usage:{}", stored.provider_id)
        || stored.request_digest != declaration_request_digest(&request)?
        || stored.overage_meters
            != overage_meters(
                &request.cumulative_declared_usage,
                &stored.reserved_contract,
            )?
    {
        bail!("Attempt 用量声明请求或合同审计失败");
    }
    audit_readings(stored, &request)?;
    let event = digest_json(&serde_json::json!({
        "schema":COMPUTE_ATTEMPT_USAGE_DECLARATION_SCHEMA,
        "snapshot_id":stored.snapshot_id,
        "lease_id":stored.lease_id,
        "provider_id":stored.provider_id,
        "consumer_account_id":stored.consumer_account_id,
        "sequence_no":stored.sequence_no,
        "source_lease_revision":stored.source_lease_revision,
        "source_lease_digest":stored.source_lease_digest,
        "fencing_generation":stored.fencing_generation,
        "job_id":stored.job_id,
        "job_revision":stored.job_revision,
        "job_digest":stored.job_digest,
        "reservation_id":stored.reservation_id,
        "reservation_revision":stored.reservation_revision,
        "reservation_digest":stored.reservation_digest,
        "capacity_claim_id":stored.capacity_claim_id,
        "capacity_claim_revision":stored.capacity_claim_revision,
        "capacity_claim_digest":stored.capacity_claim_digest,
        "executor_usage_ref":stored.executor_usage_ref,
        "cumulative_usage_digest":stored.cumulative_usage_digest,
        "reserved_contract_digest":stored.reserved_contract_digest,
        "overage_meters":stored.overage_meters,
        "request_digest":stored.request_digest,
        "declared_by_user_id":stored.declared_by_user_id,
        "declared_at":stored.declared_at,
    }))?;
    if stored.event_digest != event {
        bail!("Attempt 用量声明事件摘要审计失败");
    }
    Ok(())
}

fn audit_readings(
    stored: &StoredUsageDeclaration,
    request: &DeclareComputeAttemptUsageRequest,
) -> Result<()> {
    for (input, reading) in request
        .cumulative_declared_usage
        .iter()
        .zip(&stored.cumulative_usage)
    {
        let expected = build_readings(
            &DeclareComputeAttemptUsageRequest {
                cumulative_declared_usage: vec![input.clone()],
                ..request.clone()
            },
            &stored.declared_at,
        )?;
        if expected.first() != Some(reading) {
            bail!("Attempt meter 读数摘要审计失败");
        }
    }
    Ok(())
}
