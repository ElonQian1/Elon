use serde::Serialize;

use crate::{
    compute_federation::attempt_gateway::{
        ComputeAttemptAdapterAckEnvelope, ComputeAttemptAdapterBinding,
        ComputeAttemptDispatchCommandEnvelope,
    },
    store::{
        compute_attempt_start_outbox::AcceptedStartCommitClosureReceipt,
        ComputeAttemptActivationReceipt,
    },
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptDispatchCommandReceipt {
    pub command: ComputeAttemptDispatchCommandEnvelope,
    pub adapter: ComputeAttemptAdapterBinding,
    pub adapter_binding_digest: String,
    pub created_at: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptDispatchAckReceipt {
    pub ack: ComputeAttemptAdapterAckEnvelope,
    pub disposition: String,
    pub disposition_reason_code: Option<String>,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptDispatchApplicationReceipt {
    pub schema: String,
    pub application_id: String,
    pub application_digest: String,
    pub command_id: String,
    pub ack_id: String,
    pub action: String,
    pub lease_id: String,
    pub activation_request_digest: String,
    pub lease_digest: String,
    pub applied_at: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub(crate) enum ComputeAttemptDispatchAckCommit {
    Rejected {
        ack: ComputeAttemptDispatchAckReceipt,
    },
    Quarantined {
        ack: ComputeAttemptDispatchAckReceipt,
    },
    Activated {
        ack: ComputeAttemptDispatchAckReceipt,
        application: ComputeAttemptDispatchApplicationReceipt,
        accepted_closure: AcceptedStartCommitClosureReceipt,
        activation: ComputeAttemptActivationReceipt,
    },
}
