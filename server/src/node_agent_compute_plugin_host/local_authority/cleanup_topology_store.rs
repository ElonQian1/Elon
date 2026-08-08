use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    AuthorizedCandidateCleanupDeletionGuard, ComputePluginAuthorityInstanceBinding,
    ComputePluginFetchProcessFence, ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        CandidateCleanupExecutionState, HashedComputePluginCandidateCleanupExecutionPlan,
        ValidatedCandidateCleanupTopologyPermit,
    },
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod recovery;
mod validation;
mod write;

pub(super) use validation::read_exact_sealed_plan;

pub(in crate::node_agent_compute_plugin_host) use recovery::{
    ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession,
    ComputePluginCandidateCleanupTopologyRecoveryOutcome,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupTopologyAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_topology_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        prepared_at: Instant,
    ) -> Result<ComputePluginCandidateCleanupTopologyAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || !is_sha256(observation.installation_id_digest())
            || !is_sha256(observation.clock_epoch_digest())
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observed_at <= process_fence.acquired_observed_at()
            || prepared_at <= observed_at
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_SESSION_INVALID");
        }
        Ok(ComputePluginCandidateCleanupTopologyAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
        })
    }
}

impl ComputePluginCandidateCleanupTopologyAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }
    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_fence.process_owner_epoch()
    }
    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn trusted_now_ms(&self) -> i64 {
        self.trusted_now.timestamp_millis()
    }
    pub(in crate::node_agent_compute_plugin_host) fn observed_at(&self) -> Instant {
        self.observed_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn validate_source(
        &self,
        guard: &AuthorizedCandidateCleanupDeletionGuard,
    ) -> Result<()> {
        guard.validate_process_fence(self.process_fence)
    }
    pub(in crate::node_agent_compute_plugin_host) fn validate_candidate_cleanup_topology(
        &self,
        state: &CandidateCleanupExecutionState,
        plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    ) -> Result<()> {
        self.validate_source(state.deletion_guard())?;
        let _operation = state.deletion_guard().enter_operation()?;
        self.authority.with_deferred(|transaction| {
            validation::validate_unsealed_binding(transaction, self, state, plan)
        })
    }
    pub(in crate::node_agent_compute_plugin_host) fn persist_candidate_cleanup_topology(
        &self,
        permit: ValidatedCandidateCleanupTopologyPermit<'_>,
    ) -> Result<HashedComputePluginCandidateCleanupExecutionPlan> {
        self.validate_source(permit.state().deletion_guard())?;
        let _operation = permit.state().deletion_guard().enter_operation()?;
        self.authority.with_immediate(|transaction| {
            write::persist_candidate_cleanup_topology(transaction, self, permit)
        })
    }
}
