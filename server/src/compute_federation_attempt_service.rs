use anyhow::{bail, Result};
use serde::Deserialize;

use crate::store::{ActivateComputeAttemptRequest, ComputeAttemptActivationReceipt, Store};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivateMyComputeAttemptRequest {
    pub lease_id: String,
    pub reservation_id: String,
    pub executor_id: String,
    pub shard_id: Option<String>,
    pub attempt_no: i64,
    pub fencing_generation: i64,
    pub executor_acceptance_ref: String,
    pub lease_credential_ref: String,
    pub lease_credential_hint: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub expected_claim_revision: i64,
    pub expected_claim_digest: String,
    pub expires_at: String,
    pub hard_deadline_at: String,
    pub idempotency_key: String,
    pub confirm_executor_accepted: bool,
}

pub(crate) fn activate_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    request: ActivateMyComputeAttemptRequest,
) -> Result<ComputeAttemptActivationReceipt> {
    if !request.confirm_executor_accepted {
        bail!("登记 Attempt 激活前必须显式确认外部执行器已经接受任务");
    }
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    store.activate_compute_attempt(&ActivateComputeAttemptRequest {
        lease_id: request.lease_id,
        reservation_id: request.reservation_id,
        provider_id: provider_id.to_string(),
        executor_id: request.executor_id,
        shard_id: request.shard_id,
        attempt_no: request.attempt_no,
        fencing_generation: request.fencing_generation,
        executor_acceptance_ref: request.executor_acceptance_ref,
        lease_credential_ref: request.lease_credential_ref,
        lease_credential_hint: request.lease_credential_hint,
        expected_job_revision: request.expected_job_revision,
        expected_job_digest: request.expected_job_digest,
        expected_reservation_revision: request.expected_reservation_revision,
        expected_reservation_digest: request.expected_reservation_digest,
        expected_claim_revision: request.expected_claim_revision,
        expected_claim_digest: request.expected_claim_digest,
        expires_at: request.expires_at,
        hard_deadline_at: request.hard_deadline_at,
        idempotency_key: request.idempotency_key,
        activated_by_user_id: user_id.to_string(),
    })
}

pub(crate) fn get_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptActivationReceipt> {
    let receipt = store.compute_attempt_activation(lease_id)?;
    let provider = store.compute_provider(&receipt.lease.provider_id)?;
    if provider.provider.owner_account_id == user_id {
        return Ok(receipt);
    }
    let job =
        store.compute_job_version(&receipt.source_job.job_id, receipt.source_job.job_revision)?;
    if job.job.consumer_account_id != user_id {
        bail!("只能读取自己作为消费者或 Provider 所有者参与的 Attempt 激活回执");
    }
    Ok(receipt)
}
