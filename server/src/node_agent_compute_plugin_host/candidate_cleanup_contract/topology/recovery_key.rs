use std::{fmt, time::Instant};

use super::{
    types::HashedComputePluginCandidateCleanupExecutionPlan, PreparedCandidateCleanupTopology,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::CandidateCleanupOwnerExpectation,
    local_authority::{
        ComputePluginAuthorityInstanceBinding,
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
    },
};

/// Process-local identity for classifying an uncertain topology transaction. It contains the
/// expected immutable rows but no filesystem operation and cannot authorize deletion by itself.
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupTopologyRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    prepared_at: Instant,
    candidate_token: String,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    authorization_receipt: HashedComputePluginCandidateCleanupAuthorizationReceipt,
    owner: CandidateCleanupOwnerExpectation,
}

impl CandidateCleanupTopologyRecoveryKey {
    pub(super) fn from_prepared(prepared: &PreparedCandidateCleanupTopology<'_>) -> Self {
        let state = &prepared.state;
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
            candidate_token: prepared
                .state
                .staging_recovery_key()
                .candidate_token()
                .to_string(),
            plan: prepared.plan.clone(),
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

impl fmt::Debug for CandidateCleanupTopologyRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupTopologyRecoveryKey")
            .field("cleanup_id", &"<redacted>")
            .field("candidate_token", &"<redacted>")
            .field("plan_digest", &self.plan.plan_digest())
            .finish()
    }
}
