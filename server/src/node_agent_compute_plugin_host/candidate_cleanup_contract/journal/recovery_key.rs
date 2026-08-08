use std::{fmt, time::Instant};

use super::{HashedComputePluginCandidateCleanupStepEvent, PreparedCandidateCleanupDeleteIntent};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        CandidateCleanupOwnerExpectation, HashedComputePluginCandidateCleanupExecutionPlan,
    },
    local_authority::{
        ComputePluginAuthorityInstanceBinding,
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
    },
};

/// Process-local identity for classifying an uncertain initial delete-intent transaction.
/// It carries no filesystem mutation authority and cannot be serialized or cloned.
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDeleteIntentRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    prepared_at: Instant,
    candidate_token: String,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    event: HashedComputePluginCandidateCleanupStepEvent,
    authorization_receipt: HashedComputePluginCandidateCleanupAuthorizationReceipt,
    owner: CandidateCleanupOwnerExpectation,
}

impl CandidateCleanupDeleteIntentRecoveryKey {
    pub(super) fn from_prepared(prepared: &PreparedCandidateCleanupDeleteIntent<'_>) -> Self {
        let state = prepared.sealed.state();
        Self {
            authority_instance_binding: prepared
                .authority_session
                .authority_instance_binding()
                .clone(),
            installation_id_digest: prepared
                .authority_session
                .installation_id_digest()
                .to_string(),
            clock_epoch_digest: prepared.authority_session.clock_epoch_digest().to_string(),
            prepared_at: prepared.prepared_at,
            candidate_token: state.staging_recovery_key().candidate_token().to_string(),
            plan: prepared.sealed.plan().clone(),
            event: prepared.event.clone(),
            authorization_receipt: state.authorization_receipt().clone(),
            owner: CandidateCleanupOwnerExpectation::from_staging(state.staging_recovery_key()),
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        &self.authority_instance_binding
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn prepared_at(&self) -> Instant {
        self.prepared_at
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token(&self) -> &str {
        &self.candidate_token
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> &HashedComputePluginCandidateCleanupExecutionPlan {
        &self.plan
    }

    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.event
    }

    pub(in crate::node_agent_compute_plugin_host) fn authorized_at_ms(&self) -> i64 {
        self.authorization_receipt.receipt().authorized_at_ms()
    }
    pub(in crate::node_agent_compute_plugin_host) fn authorization_receipt(
        &self,
    ) -> &HashedComputePluginCandidateCleanupAuthorizationReceipt {
        &self.authorization_receipt
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner(
        &self,
    ) -> &CandidateCleanupOwnerExpectation {
        &self.owner
    }
}

impl fmt::Debug for CandidateCleanupDeleteIntentRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupDeleteIntentRecoveryKey")
            .field("candidate_token", &"<redacted>")
            .field("plan_digest", &self.plan.plan_digest())
            .field("event_digest", &self.event.event_digest())
            .finish()
    }
}
