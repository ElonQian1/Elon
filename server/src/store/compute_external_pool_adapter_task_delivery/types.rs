use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    ExternalPoolAdapterTaskEventPollEnvelope, ExternalPoolAdapterTaskReconcilePollEnvelope,
};

pub(super) use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    TASK_PRODUCTION_POLL_CLAIMED as CLAIM_STATUS_CLAIMED,
    TASK_PRODUCTION_POLL_DELIVERY_OBSERVED as CLAIM_STATUS_DELIVERY_OBSERVED,
    TASK_PRODUCTION_POLL_IN_FLIGHT_UNKNOWN as CLAIM_STATUS_IN_FLIGHT_UNKNOWN,
    TASK_PRODUCTION_POLL_PENDING as CLAIM_STATUS_PENDING,
    TASK_PRODUCTION_POLL_QUARANTINED as CLAIM_STATUS_QUARANTINED,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PollClaimProjection {
    pub status: String,
    pub revision: u64,
    pub generation: u64,
    pub owner_id: Option<String>,
    pub token_digest: Option<String>,
    pub expires_at: Option<String>,
}

pub(super) struct AuditedReconcilePoll {
    pub envelope: ExternalPoolAdapterTaskReconcilePollEnvelope,
    pub claim: PollClaimProjection,
}

pub(super) struct AuditedEventPoll {
    pub envelope: ExternalPoolAdapterTaskEventPollEnvelope,
    pub claim: PollClaimProjection,
}

/// Process-local scheduling custody only. It is deliberately non-Clone/non-Debug/non-Serde.
#[allow(dead_code)]
pub(in crate::store) struct ExternalPoolAdapterTaskPollClaim {
    pub(super) poll_id: String,
    pub(super) poll_digest: String,
    pub(super) claim_revision: u64,
    pub(super) claim_generation: u64,
    pub(super) claim_owner_id: String,
    pub(super) raw_claim_token: String,
    pub(super) claim_expires_at: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ExternalPoolAdapterTaskDeliveryRecoveryReport {
    pub(super) audited_rows: usize,
    pub(super) recovered_rows: usize,
    pub(super) eligible_rows: usize,
}
