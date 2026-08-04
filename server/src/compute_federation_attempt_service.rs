use anyhow::{bail, Result};
use serde::Deserialize;

use crate::store::{
    AbortComputeAttemptRequest, ActivateComputeAttemptRequest, ComputeAttemptAbortReceipt,
    ComputeAttemptActivationReceipt, ComputeAttemptLeaseRenewalReceipt,
    ComputeAttemptLeaseStateReceipt, RenewComputeAttemptLeaseRequest, Store,
};

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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenewMyComputeAttemptLeaseRequest {
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub executor_heartbeat_ref: String,
    pub expires_at: String,
    pub idempotency_key: String,
    pub confirm_executor_alive: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AbortMyComputeAttemptRequest {
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub expected_claim_revision: i64,
    pub expected_claim_digest: String,
    pub executor_abort_ref: String,
    pub reason_code: String,
    pub idempotency_key: String,
    pub confirm_no_execution_started: bool,
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

pub(crate) fn renew_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    lease_id: &str,
    request: RenewMyComputeAttemptLeaseRequest,
) -> Result<ComputeAttemptLeaseRenewalReceipt> {
    if !request.confirm_executor_alive {
        bail!("续租 Attempt Lease 前必须显式确认外部执行器仍存活");
    }
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    store.renew_compute_attempt_lease(&RenewComputeAttemptLeaseRequest {
        lease_id: lease_id.to_string(),
        provider_id: provider_id.to_string(),
        expected_lease_revision: request.expected_lease_revision,
        expected_lease_digest: request.expected_lease_digest,
        expected_fencing_generation: request.expected_fencing_generation,
        executor_heartbeat_ref: request.executor_heartbeat_ref,
        expires_at: request.expires_at,
        idempotency_key: request.idempotency_key,
        renewed_by_user_id: user_id.to_string(),
    })
}

pub(crate) fn get_state_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptLeaseStateReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_lease_state(lease_id)
}

pub(crate) fn abort_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    lease_id: &str,
    request: AbortMyComputeAttemptRequest,
) -> Result<ComputeAttemptAbortReceipt> {
    if !request.confirm_no_execution_started {
        bail!("无用量中止前必须显式确认外部执行器从未开始执行");
    }
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    store.abort_compute_attempt(&AbortComputeAttemptRequest {
        lease_id: lease_id.to_string(),
        provider_id: provider_id.to_string(),
        expected_lease_revision: request.expected_lease_revision,
        expected_lease_digest: request.expected_lease_digest,
        expected_fencing_generation: request.expected_fencing_generation,
        expected_job_revision: request.expected_job_revision,
        expected_job_digest: request.expected_job_digest,
        expected_reservation_revision: request.expected_reservation_revision,
        expected_reservation_digest: request.expected_reservation_digest,
        expected_claim_revision: request.expected_claim_revision,
        expected_claim_digest: request.expected_claim_digest,
        executor_abort_ref: request.executor_abort_ref,
        reason_code: request.reason_code,
        idempotency_key: request.idempotency_key,
        aborted_by_user_id: user_id.to_string(),
    })
}

pub(crate) fn get_abort_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptAbortReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_abort(lease_id)
}
