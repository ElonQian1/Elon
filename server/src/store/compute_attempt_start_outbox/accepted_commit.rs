use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use crate::{
    compute_federation::{
        attempt_gateway::{
            ComputeAttemptAdapterAckEnvelope, ComputeAttemptAdapterBinding,
            ComputeAttemptDispatchApplicationEnvelope, ComputeAttemptDispatchCommandEnvelope,
        },
        execution_plan::ComputeAttemptExecutionPlanEnvelope,
        route_authority::{
            ComputeRouteAuthorizationEnvelope, ComputeServiceActorAuthorizationEnvelope,
        },
        start_outbox::{
            ComputeAttemptDispatchActorReceiptEnvelope, ComputeLeaseAuthorityBindingEnvelope,
            ComputeStartOutboxOperationEnvelope,
        },
    },
    store::{compute_attempt_dispatches::PreparedApplication, ComputeAttemptActivationReceipt},
};

use super::types::StoredStartOutboxOperation;

mod currentness;
mod derive;
mod persist;
mod readback;
mod route_audit;
mod source;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct AcceptedStartCommitClosureReceipt {
    pub application_actor_receipt_id: String,
    pub application_actor_receipt_digest: String,
    pub lease_authority_id: String,
    pub lease_authority_revision: i64,
    pub lease_authority_digest: String,
    pub commit_outbox_id: String,
    pub commit_outbox_digest: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::store) enum AcceptedStartCommitFreshness {
    Current,
    Quarantine { reason_code: &'static str },
}

struct AcceptedCommitBase {
    command: ComputeAttemptDispatchCommandEnvelope,
    adapter: ComputeAttemptAdapterBinding,
    lease_credential_ref: String,
    lease_credential_hint: String,
    activated_by_user_id: String,
    prepare: StoredStartOutboxOperation,
    plan: ComputeAttemptExecutionPlanEnvelope,
    route: ComputeRouteAuthorizationEnvelope,
    actor_authority: ComputeServiceActorAuthorizationEnvelope,
}

struct AcceptedApplicationFact {
    envelope: ComputeAttemptDispatchApplicationEnvelope,
}

struct AcceptedCommitSource {
    base: AcceptedCommitBase,
    ack: ComputeAttemptAdapterAckEnvelope,
    activation: ComputeAttemptActivationReceipt,
    application: AcceptedApplicationFact,
}

struct DerivedAcceptedCommitClosure {
    actor: ComputeAttemptDispatchActorReceiptEnvelope,
    actor_json: String,
    authority: ComputeLeaseAuthorityBindingEnvelope,
    authority_json: String,
    commit: ComputeStartOutboxOperationEnvelope,
    commit_json: String,
    provider_id: String,
    adapter_id: String,
}

impl DerivedAcceptedCommitClosure {
    fn receipt(&self, replayed: bool) -> AcceptedStartCommitClosureReceipt {
        AcceptedStartCommitClosureReceipt {
            application_actor_receipt_id: self.actor.actor_receipt_id.clone(),
            application_actor_receipt_digest: self.actor.actor_receipt_digest.clone(),
            lease_authority_id: self.authority.lease_authority_id.clone(),
            lease_authority_revision: self.authority.authority_revision,
            lease_authority_digest: self.authority.lease_authority_digest.clone(),
            commit_outbox_id: self.commit.outbox_id.clone(),
            commit_outbox_digest: self.commit.outbox_digest.clone(),
            replayed,
        }
    }
}

/// Checks only fresh accepted-apply liveness. Immutable corruption remains an error; ordinary
/// route, plan, credential, or actor drift is returned as an explicit quarantine decision.
pub(in crate::store) fn ensure_fresh_accepted_start_commit_on(
    connection: &Connection,
    command_id: &str,
    checked_at: &str,
) -> Result<AcceptedStartCommitFreshness> {
    currentness::ensure_fresh_on(connection, command_id, checked_at)
}

/// Persists Store-derived application actor, lease authority, and pending Commit intent. The
/// caller-supplied application is a locally prepared digest fact, never an authority DTO.
pub(in crate::store) fn persist_accepted_start_commit_closure_on(
    connection: &Connection,
    command_id: &str,
    application: &PreparedApplication,
    closure_at: &str,
) -> Result<AcceptedStartCommitClosureReceipt> {
    persist::persist_on(connection, command_id, application, closure_at)
}

/// Historical replay audits only immutable closure. It intentionally ignores current liveness.
pub(in crate::store) fn audit_accepted_start_commit_closure_on(
    connection: &Connection,
    command_id: &str,
) -> Result<AcceptedStartCommitClosureReceipt> {
    readback::audit_on(connection, command_id, true)
}
