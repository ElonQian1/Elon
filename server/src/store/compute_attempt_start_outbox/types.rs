use std::fmt;

use crate::compute_federation::start_outbox::{
    ComputeStartOutboxClaimProjection, ComputeStartOutboxClaimReceiptEnvelope,
    ComputeStartOutboxOperationEnvelope, ComputeStartOutboxRemoteObservationEnvelope,
    ComputeStartOutboxSendAttemptEnvelope,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartOutboxEnqueueReceipt {
    pub outbox_id: String,
    pub outbox_digest: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartOutboxObservationReceipt {
    pub observation_id: String,
    pub observation_digest: String,
    pub outbox_id: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartOutboxNoStartProofReceipt {
    pub proof_id: String,
    pub proof_digest: String,
    pub command_id: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum StartNoStartDerivation<'a> {
    LocalNeverSent {
        command_id: &'a str,
        proven_at: &'a str,
    },
    PrepareRejected {
        command_id: &'a str,
        observation_id: &'a str,
        proven_at: &'a str,
    },
    RemoteNeverCommitted {
        command_id: &'a str,
        observation_id: &'a str,
        proven_at: &'a str,
    },
}

impl<'a> StartNoStartDerivation<'a> {
    pub(super) fn command_id(self) -> &'a str {
        match self {
            Self::LocalNeverSent { command_id, .. }
            | Self::PrepareRejected { command_id, .. }
            | Self::RemoteNeverCommitted { command_id, .. } => command_id,
        }
    }

    pub(super) fn observation_id(self) -> Option<&'a str> {
        match self {
            Self::LocalNeverSent { .. } => None,
            Self::PrepareRejected { observation_id, .. }
            | Self::RemoteNeverCommitted { observation_id, .. } => Some(observation_id),
        }
    }

    pub(super) fn proven_at(self) -> &'a str {
        match self {
            Self::LocalNeverSent { proven_at, .. }
            | Self::PrepareRejected { proven_at, .. }
            | Self::RemoteNeverCommitted { proven_at, .. } => proven_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartResolutionProofReceipt {
    pub proof_id: String,
    pub proof_digest: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BrokerFinishStartResolutionBinding<'a> {
    pub reservation_id: &'a str,
    pub reservation_revision: i64,
    pub reservation_digest: &'a str,
    pub job_id: &'a str,
    pub job_revision: i64,
    pub job_digest: &'a str,
    pub capacity_claim_id: &'a str,
    pub capacity_claim_revision: i64,
    pub capacity_claim_digest: &'a str,
    pub budget_reservation_id: &'a str,
    pub budget_refunded_fen: i64,
}

#[derive(Clone)]
pub(super) struct StoredStartOutboxOperation {
    pub envelope: ComputeStartOutboxOperationEnvelope,
    pub provider_id: String,
    pub adapter_id: String,
    pub projection: ComputeStartOutboxClaimProjection,
}

impl fmt::Debug for StoredStartOutboxOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredStartOutboxOperation")
            .field("outbox_id", &self.envelope.outbox_id)
            .field("operation_kind", &self.envelope.operation_kind)
            .field("state", &self.projection.state)
            .field("state_revision", &self.projection.state_revision)
            .finish()
    }
}

/// Local work ownership. The raw token is memory-only and deliberately omitted from Debug.
pub(crate) struct StartOutboxClaimHandle {
    pub(super) operation: StoredStartOutboxOperation,
    pub(super) receipt: ComputeStartOutboxClaimReceiptEnvelope,
    pub(super) raw_claim_token: String,
}

impl fmt::Debug for StartOutboxClaimHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StartOutboxClaimHandle")
            .field("outbox_id", &self.receipt.outbox_id)
            .field("claim_generation", &self.receipt.claim_generation)
            .field("claim_token", &"<redacted>")
            .finish()
    }
}

/// Future transport seal. No constructor exists until a concrete Adapter seals a request.
pub(crate) struct PreparedStartSendRequest {
    pub(super) request_digest: String,
}

impl fmt::Debug for PreparedStartSendRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedStartSendRequest")
            .field("request", &"<sealed digest>")
            .finish()
    }
}

/// Authority issued only after send-start evidence and the unknown-delivery transition commit.
/// It is non-Clone and has no consumer in this architecture batch.
pub(crate) struct CommittedStartSendAuthority {
    pub(super) attempt: ComputeStartOutboxSendAttemptEnvelope,
    pub(super) claim: StartOutboxClaimHandle,
    pub(super) request: PreparedStartSendRequest,
}

impl fmt::Debug for CommittedStartSendAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommittedStartSendAuthority")
            .field("send_attempt_id", &self.attempt.send_attempt_id)
            .field("outbox_id", &self.attempt.outbox_id)
            .field("request", &"<sealed>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartOutboxCurrentnessPhase {
    Prepare,
    Commit,
    CleanupCancel,
    CleanupReconcile,
}

impl StartOutboxCurrentnessPhase {
    pub(super) fn required_capability(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::Commit => "idempotent_commit",
            Self::CleanupCancel => "cancel_no_start",
            Self::CleanupReconcile => "reconcile",
        }
    }

    pub(super) fn required_actor_phase(self) -> &'static str {
        match self {
            Self::Commit => "application",
            Self::Prepare | Self::CleanupCancel | Self::CleanupReconcile => "dispatch",
        }
    }

    pub(super) fn uses_cleanup_horizon(self) -> bool {
        matches!(self, Self::CleanupCancel | Self::CleanupReconcile)
    }
}

pub(super) struct StoredVerifiedObservation {
    pub envelope: ComputeStartOutboxRemoteObservationEnvelope,
}

pub(super) struct NoStartProofSource {
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
    pub fencing_generation: i64,
    pub adapter_id: String,
    pub adapter_revision: i64,
    pub adapter_registry_digest: String,
    pub adapter_binding_digest: String,
    pub route_authorization_id: String,
    pub route_authorization_digest: String,
    pub prepare_state: String,
    pub prepare_state_revision: i64,
    pub prepare_attempt_count: i64,
    pub prepare_claim_generation: i64,
    pub prepare_claim_expires_at: Option<String>,
    pub prepare_not_after: String,
}
