use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_START_OUTBOX_OPERATION_SCHEMA: &str =
    "compute_federation.attempt_start_outbox.v1";
pub(crate) const COMPUTE_START_OUTBOX_CLAIM_RECEIPT_SCHEMA: &str =
    "compute_federation.start_outbox_claim_receipt.v1";
pub(crate) const COMPUTE_START_OUTBOX_SEND_ATTEMPT_SCHEMA: &str =
    "compute_federation.attempt_start_send_attempt.v1";
pub(crate) const COMPUTE_START_OUTBOX_REMOTE_OBSERVATION_SCHEMA: &str =
    "compute_federation.attempt_start_remote_observation.v1";
pub(crate) const COMPUTE_START_NO_START_PROOF_SCHEMA: &str =
    "compute_federation.attempt_no_start_proof.v1";
pub(crate) const COMPUTE_LEASE_AUTHORITY_BINDING_SCHEMA: &str =
    "compute_federation.attempt_lease_authority.v1";
pub(crate) const COMPUTE_ATTEMPT_DISPATCH_ACTOR_RECEIPT_SCHEMA: &str =
    "compute_federation.attempt_dispatch_actor.v1";
pub(crate) const COMPUTE_START_OUTBOX_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_START_OUTBOX_DIGEST_ALGORITHM: &str = "sha256";

pub(crate) const COMPUTE_START_OPERATION_PREPARE: &str = "prepare";
pub(crate) const COMPUTE_START_OPERATION_COMMIT: &str = "commit";
pub(crate) const COMPUTE_START_OPERATION_CANCEL: &str = "cancel";
pub(crate) const COMPUTE_START_OPERATION_RECONCILE: &str = "reconcile";

pub(crate) const COMPUTE_OUTBOX_STATE_BLOCKED: &str = "blocked";
pub(crate) const COMPUTE_OUTBOX_STATE_PENDING: &str = "pending";
pub(crate) const COMPUTE_OUTBOX_STATE_CLAIMED: &str = "claimed";
pub(crate) const COMPUTE_OUTBOX_STATE_IN_FLIGHT_UNKNOWN: &str = "in_flight_unknown";
pub(crate) const COMPUTE_OUTBOX_STATE_DELIVERY_OBSERVED: &str = "delivery_observed";
pub(crate) const COMPUTE_OUTBOX_STATE_ABANDONED_NO_SEND: &str = "abandoned_no_send";
pub(crate) const COMPUTE_OUTBOX_STATE_QUARANTINED: &str = "quarantined";

pub(crate) const COMPUTE_OBSERVATION_PREPARE_RESPONSE: &str = "prepare_response";
pub(crate) const COMPUTE_OBSERVATION_COMMIT_RESPONSE: &str = "commit_response";
pub(crate) const COMPUTE_OBSERVATION_CANCEL_RESPONSE: &str = "cancel_response";
pub(crate) const COMPUTE_OBSERVATION_RECONCILE_ATTESTATION: &str = "reconcile_attestation";
pub(crate) const COMPUTE_REMOTE_EXECUTION_ABSENT: &str = "absent";
pub(crate) const COMPUTE_REMOTE_EXECUTION_PREPARED: &str = "prepared";
pub(crate) const COMPUTE_REMOTE_EXECUTION_COMMITTED: &str = "committed";
pub(crate) const COMPUTE_REMOTE_EXECUTION_RUNNING: &str = "running";
pub(crate) const COMPUTE_REMOTE_EXECUTION_TERMINAL_NO_START: &str = "terminal_no_start";
pub(crate) const COMPUTE_REMOTE_EXECUTION_TERMINAL_AFTER_RUN: &str = "terminal_after_run";
pub(crate) const COMPUTE_REMOTE_EXECUTION_UNKNOWN: &str = "unknown";
pub(crate) const COMPUTE_REMOTE_EXECUTION_REJECTED: &str = "rejected";
pub(crate) const COMPUTE_REMOTE_TERMINALITY_NON_TERMINAL: &str = "non_terminal";
pub(crate) const COMPUTE_REMOTE_TERMINALITY_FINAL: &str = "final";

pub(crate) const COMPUTE_NO_START_PROOF_LOCAL_NEVER_SENT: &str = "local_never_sent";
pub(crate) const COMPUTE_NO_START_PROOF_PREPARE_REJECTED: &str = "prepare_rejected";
pub(crate) const COMPUTE_NO_START_PROOF_REMOTE_NEVER_COMMITTED: &str = "remote_never_committed";
pub(crate) const COMPUTE_ACTOR_RECEIPT_PHASE_DISPATCH: &str = "dispatch";
pub(crate) const COMPUTE_ACTOR_RECEIPT_PHASE_APPLICATION: &str = "application";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeStartOutboxOperationEnvelope {
    pub schema: String,
    pub outbox_id: String,
    pub outbox_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub operation_kind: String,
    pub operation_generation: i64,
    pub subject_outbox_id: Option<String>,
    pub command_id: String,
    pub command_digest: String,
    pub adapter_binding_digest: String,
    pub route_authorization_id: String,
    pub route_authorization_digest: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub lease_id: String,
    pub fencing_generation: i64,
    pub ack_id: Option<String>,
    pub ack_digest: Option<String>,
    pub application_id: Option<String>,
    pub application_digest: Option<String>,
    pub lease_authority_id: Option<String>,
    pub lease_authority_revision: Option<i64>,
    pub lease_authority_digest: Option<String>,
    pub actor_receipt_id: String,
    pub actor_receipt_digest: String,
    pub issued_at: String,
    pub not_before: String,
    pub not_after: String,
}

/// Mutable Store projection only. The raw claim token is never persisted or serialized here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeStartOutboxClaimProjection {
    pub state: String,
    pub state_revision: i64,
    pub attempt_count: i64,
    pub next_attempt_at: String,
    pub claim_owner_id: Option<String>,
    pub claim_token_digest: Option<String>,
    pub claim_generation: i64,
    pub claim_expires_at: Option<String>,
    pub last_failure_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeStartOutboxClaimReceiptEnvelope {
    pub schema: String,
    pub claim_receipt_id: String,
    pub claim_receipt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub outbox_id: String,
    pub outbox_digest: String,
    pub state_revision: i64,
    pub attempt_no: i64,
    pub claim_owner_id: String,
    pub claim_token_digest: String,
    pub claim_generation: i64,
    pub claimed_at: String,
    pub claim_expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeStartOutboxSendAttemptEnvelope {
    pub schema: String,
    pub send_attempt_id: String,
    pub send_attempt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub outbox_id: String,
    pub outbox_digest: String,
    pub attempt_no: i64,
    pub command_id: String,
    pub command_digest: String,
    pub operation_kind: String,
    pub route_authorization_id: String,
    pub route_authorization_digest: String,
    pub claim_generation: i64,
    pub claim_token_digest: String,
    pub request_digest: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeStartOutboxRemoteObservationEnvelope {
    pub schema: String,
    pub observation_id: String,
    pub observation_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub observation_kind: String,
    pub send_attempt_id: String,
    pub outbox_id: String,
    pub outbox_digest: String,
    pub operation_kind: String,
    pub command_id: String,
    pub command_digest: String,
    pub provider_id: String,
    pub adapter_id: String,
    pub adapter_binding_digest: String,
    pub adapter_observation_id: String,
    pub response_outcome: String,
    pub remote_execution_state: String,
    pub terminality: String,
    pub remote_execution_ref: Option<String>,
    pub remote_sequence: i64,
    pub no_commit_tombstone_id: Option<String>,
    pub no_commit_tombstone_digest: Option<String>,
    pub reason_code: Option<String>,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verification_digest: String,
    pub authenticated_at: String,
    pub observed_at: String,
    pub received_at: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeStartNoStartProofEnvelope {
    pub schema: String,
    pub proof_id: String,
    pub proof_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub proof_kind: String,
    pub outbox_id: String,
    pub outbox_digest: String,
    pub command_id: String,
    pub command_digest: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub provider_id: String,
    pub reservation_id: String,
    pub reservation_revision: i64,
    pub reservation_digest: String,
    pub job_id: String,
    pub job_revision: i64,
    pub job_digest: String,
    pub capacity_claim_id: String,
    pub capacity_claim_revision: i64,
    pub capacity_claim_digest: String,
    pub budget_reservation_id: String,
    pub budget_reserved_fen: i64,
    pub broker_request_digest: String,
    pub lease_id: String,
    /// No v185 Lease exists on any no-start path. The explicit JSON null prevents callers from
    /// inventing a post-activation digest for a pre-activation proof.
    pub lease_digest: Option<String>,
    pub fencing_generation: i64,
    pub adapter_id: String,
    pub adapter_revision: i64,
    pub adapter_registry_digest: String,
    pub adapter_binding_digest: String,
    pub route_authorization_id: String,
    pub route_authorization_digest: String,
    pub observation_id: Option<String>,
    pub observation_digest: Option<String>,
    pub no_commit_tombstone_id: Option<String>,
    pub no_commit_tombstone_digest: Option<String>,
    pub proven_at: String,
    pub recorded_at: String,
}

/// Sealed lookup authority only. The ref and hint cannot be used as bearer credentials.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeLeaseAuthorityBindingEnvelope {
    pub schema: String,
    pub lease_authority_id: String,
    pub authority_revision: i64,
    pub lease_authority_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub authority_kind: String,
    pub delivery_mode: String,
    pub non_bearer_authority_ref: String,
    pub authority_hint: String,
    pub command_id: String,
    pub command_digest: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub ack_id: String,
    pub ack_digest: String,
    pub application_id: String,
    pub application_digest: String,
    pub application_actor_receipt_id: String,
    pub application_actor_receipt_digest: String,
    pub lease_id: String,
    pub lease_digest: String,
    pub provider_id: String,
    pub executor_id: String,
    pub fencing_generation: i64,
    pub route_authorization_id: String,
    pub route_authorization_digest: String,
    pub audience: String,
    pub scopes: Vec<String>,
    pub scopes_digest: String,
    pub issued_at: String,
    pub expires_at: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptDispatchActorReceiptEnvelope {
    pub schema: String,
    pub actor_receipt_id: String,
    pub actor_receipt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub actor_phase: String,
    pub command_id: String,
    pub command_digest: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub service_actor_id: String,
    pub actor_authorization_id: String,
    pub actor_authorization_digest: String,
    pub route_authorization_id: String,
    pub route_authorization_digest: String,
    pub ack_id: Option<String>,
    pub ack_digest: Option<String>,
    pub application_id: Option<String>,
    pub application_digest: Option<String>,
    pub issued_at: String,
    pub valid_until: String,
    pub recorded_at: String,
}
