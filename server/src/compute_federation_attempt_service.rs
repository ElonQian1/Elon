use anyhow::{bail, Result};
use serde::Deserialize;

use crate::store::{
    AbortComputeAttemptRequest, ActivateComputeAttemptRequest, ComputeAttemptAbortReceipt,
    ComputeAttemptActivationReceipt, ComputeAttemptConsumerReviewReceipt,
    ComputeAttemptLeaseRenewalReceipt, ComputeAttemptLeaseStateReceipt,
    ComputeAttemptPlatformObservationReceipt, ComputeAttemptTerminalCandidateReceipt,
    ComputeAttemptUsageDeclarationReceipt, ComputeAttemptUsageTemplateReceipt,
    ComputeAttemptVerificationDecisionReceipt, ComputeDeclaredResultArtifactInput,
    ComputeDeclaredUsageInput, ComputeObservedUsageInput, ComputeReservationRegistrationReceipt,
    DecideComputeAttemptVerificationRequest, DeclareComputeAttemptTerminalCandidateRequest,
    DeclareComputeAttemptUsageRequest, ObserveComputeAttemptTerminalCandidateRequest,
    RenewComputeAttemptLeaseRequest, ReviewComputeAttemptTerminalCandidateRequest, Store,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclareMyComputeAttemptUsageRequest {
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub sequence_no: i64,
    pub executor_usage_ref: String,
    pub cumulative_declared_usage: Vec<ComputeDeclaredUsageInput>,
    pub idempotency_key: String,
    pub confirm_provider_declaration_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclareMyComputeAttemptTerminalCandidateRequest {
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub final_usage_snapshot_id: String,
    pub final_usage_sequence_no: i64,
    pub final_cumulative_usage_digest: String,
    pub executor_terminal_ref: String,
    pub outcome: String,
    pub reason_code: String,
    pub diagnostic_ref: Option<String>,
    pub output_digest: Option<String>,
    pub result_artifacts: Vec<ComputeDeclaredResultArtifactInput>,
    pub idempotency_key: String,
    pub confirm_provider_declaration_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewMyComputeAttemptTerminalCandidateRequest {
    pub expected_terminal_candidate_id: String,
    pub expected_terminal_candidate_event_digest: String,
    pub decision: String,
    pub reason_code: String,
    pub consumer_review_ref: String,
    pub evidence_refs: Vec<String>,
    pub idempotency_key: String,
    pub confirm_consumer_attestation_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObserveComputeAttemptTerminalCandidateBody {
    pub expected_terminal_candidate_id: String,
    pub expected_terminal_candidate_event_digest: String,
    pub observation_source: String,
    pub observer_ref: String,
    pub observed_outcome: String,
    pub cumulative_observed_usage: Vec<ComputeObservedUsageInput>,
    pub evidence_refs: Vec<String>,
    pub idempotency_key: String,
    pub confirm_platform_observation_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecideComputeAttemptVerificationBody {
    pub expected_terminal_candidate_id: String,
    pub expected_terminal_candidate_event_digest: String,
    pub expected_consumer_review_id: String,
    pub expected_consumer_review_event_digest: String,
    pub expected_platform_observation_id: String,
    pub expected_platform_observation_event_digest: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub decision: String,
    pub reason_codes: Vec<String>,
    pub decision_ref: String,
    pub idempotency_key: String,
    pub confirm_no_state_or_settlement_effect: bool,
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

pub(crate) fn list_activation_candidates_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    limit: usize,
) -> Result<Vec<ComputeReservationRegistrationReceipt>> {
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    store.list_compute_attempt_activation_candidates(provider_id, limit)
}

pub(crate) fn list_leases_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    limit: usize,
) -> Result<Vec<ComputeAttemptLeaseStateReceipt>> {
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    store.list_compute_attempt_lease_states(provider_id, limit)
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

pub(crate) fn declare_usage_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    lease_id: &str,
    request: DeclareMyComputeAttemptUsageRequest,
) -> Result<ComputeAttemptUsageDeclarationReceipt> {
    if !request.confirm_provider_declaration_only {
        bail!("登记用量前必须确认该快照只是 Provider 声明，不是验证或结算结果");
    }
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    store.declare_compute_attempt_usage(&DeclareComputeAttemptUsageRequest {
        lease_id: lease_id.to_string(),
        provider_id: provider_id.to_string(),
        expected_lease_revision: request.expected_lease_revision,
        expected_lease_digest: request.expected_lease_digest,
        expected_fencing_generation: request.expected_fencing_generation,
        sequence_no: request.sequence_no,
        executor_usage_ref: request.executor_usage_ref,
        cumulative_declared_usage: request.cumulative_declared_usage,
        idempotency_key: request.idempotency_key,
        declared_by_user_id: user_id.to_string(),
    })
}

pub(crate) fn get_usage_template_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptUsageTemplateReceipt> {
    store.compute_attempt_usage_template(provider_id, user_id, lease_id)
}

pub(crate) fn get_latest_usage_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptUsageDeclarationReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.latest_compute_attempt_usage_declaration(lease_id)
}

pub(crate) fn get_usage_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
    sequence_no: i64,
) -> Result<ComputeAttemptUsageDeclarationReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_usage_declaration(lease_id, sequence_no)
}

pub(crate) fn declare_terminal_candidate_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    lease_id: &str,
    request: DeclareMyComputeAttemptTerminalCandidateRequest,
) -> Result<ComputeAttemptTerminalCandidateReceipt> {
    if !request.confirm_provider_declaration_only {
        bail!("登记终态候选前必须确认该事件只是 Provider 声明，不是验证或结算结果");
    }
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    store.declare_compute_attempt_terminal_candidate(
        &DeclareComputeAttemptTerminalCandidateRequest {
            lease_id: lease_id.to_string(),
            provider_id: provider_id.to_string(),
            expected_lease_revision: request.expected_lease_revision,
            expected_lease_digest: request.expected_lease_digest,
            expected_fencing_generation: request.expected_fencing_generation,
            final_usage_snapshot_id: request.final_usage_snapshot_id,
            final_usage_sequence_no: request.final_usage_sequence_no,
            final_cumulative_usage_digest: request.final_cumulative_usage_digest,
            executor_terminal_ref: request.executor_terminal_ref,
            outcome: request.outcome,
            reason_code: request.reason_code,
            diagnostic_ref: request.diagnostic_ref,
            output_digest: request.output_digest,
            result_artifacts: request.result_artifacts,
            idempotency_key: request.idempotency_key,
            declared_by_user_id: user_id.to_string(),
        },
    )
}

pub(crate) fn get_terminal_candidate_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptTerminalCandidateReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_terminal_candidate(lease_id)
}

pub(crate) fn review_terminal_candidate_for_consumer(
    store: &Store,
    user_id: &str,
    lease_id: &str,
    request: ReviewMyComputeAttemptTerminalCandidateRequest,
) -> Result<ComputeAttemptConsumerReviewReceipt> {
    if !request.confirm_consumer_attestation_only {
        bail!("提交终态审核前必须确认该记录只是消费者证据，不是平台验证或结算决定");
    }
    store.review_compute_attempt_terminal_candidate(&ReviewComputeAttemptTerminalCandidateRequest {
        lease_id: lease_id.to_string(),
        expected_terminal_candidate_id: request.expected_terminal_candidate_id,
        expected_terminal_candidate_event_digest: request.expected_terminal_candidate_event_digest,
        decision: request.decision,
        reason_code: request.reason_code,
        consumer_review_ref: request.consumer_review_ref,
        evidence_refs: request.evidence_refs,
        idempotency_key: request.idempotency_key,
        reviewed_by_user_id: user_id.to_string(),
    })
}

pub(crate) fn get_consumer_review_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptConsumerReviewReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_consumer_review(lease_id)
}

pub(crate) fn observe_terminal_candidate_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    lease_id: &str,
    request: ObserveComputeAttemptTerminalCandidateBody,
) -> Result<ComputeAttemptPlatformObservationReceipt> {
    if !request.confirm_platform_observation_only {
        bail!("登记平台观测前必须确认该记录只是待验证证据，不是可信终态或结算决定");
    }
    store.observe_compute_attempt_terminal_candidate(
        &ObserveComputeAttemptTerminalCandidateRequest {
            lease_id: lease_id.to_string(),
            expected_terminal_candidate_id: request.expected_terminal_candidate_id,
            expected_terminal_candidate_event_digest: request
                .expected_terminal_candidate_event_digest,
            observation_source: request.observation_source,
            observer_ref: request.observer_ref,
            observed_outcome: request.observed_outcome,
            cumulative_observed_usage: request.cumulative_observed_usage,
            evidence_refs: request.evidence_refs,
            idempotency_key: request.idempotency_key,
            observed_by_user_id: admin_user_id.to_string(),
        },
    )
}

pub(crate) fn get_platform_observation_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptPlatformObservationReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_platform_observation(lease_id)
}

pub(crate) fn decide_verification_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    lease_id: &str,
    request: DecideComputeAttemptVerificationBody,
) -> Result<ComputeAttemptVerificationDecisionReceipt> {
    if !request.confirm_no_state_or_settlement_effect {
        bail!("提交 Verification 决定前必须确认本操作不推进状态、容量或结算");
    }
    store.decide_compute_attempt_verification(&DecideComputeAttemptVerificationRequest {
        lease_id: lease_id.to_string(),
        expected_terminal_candidate_id: request.expected_terminal_candidate_id,
        expected_terminal_candidate_event_digest: request.expected_terminal_candidate_event_digest,
        expected_consumer_review_id: request.expected_consumer_review_id,
        expected_consumer_review_event_digest: request.expected_consumer_review_event_digest,
        expected_platform_observation_id: request.expected_platform_observation_id,
        expected_platform_observation_event_digest: request
            .expected_platform_observation_event_digest,
        policy_id: request.policy_id,
        policy_version: request.policy_version,
        decision: request.decision,
        reason_codes: request.reason_codes,
        decision_ref: request.decision_ref,
        idempotency_key: request.idempotency_key,
        decided_by_user_id: admin_user_id.to_string(),
    })
}

pub(crate) fn get_verification_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptVerificationDecisionReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_verification_decision(lease_id)
}
