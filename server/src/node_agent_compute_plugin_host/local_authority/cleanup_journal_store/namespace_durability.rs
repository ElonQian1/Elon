use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    AuthorizedCandidateCleanupDeletionGuard, ComputePluginAuthorityInstanceBinding,
    ComputePluginFetchProcessFence, ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        HashedComputePluginCandidateCleanupStepEvent, PhysicallyDurableCandidateCleanupNamespace,
        ValidatedCandidateCleanupNamespaceDurabilityPermit,
    },
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod recovery;
mod validation;
mod write;

pub(in crate::node_agent_compute_plugin_host) use recovery::{
    ComputePluginCandidateCleanupNamespaceDurabilityRecoveryAuthoritySession,
    ComputePluginCandidateCleanupNamespaceDurabilityRecoveryOutcome,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupNamespaceDurabilityAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_namespace_durability_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        physical: &PhysicallyDurableCandidateCleanupNamespace,
        prepared_at: Instant,
    ) -> Result<ComputePluginCandidateCleanupNamespaceDurabilityAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        let namespace = physical.namespace();
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
            || physical.parent_absence_observed_at() <= physical.disposition_set_at()
            || namespace.barrier_completed_at() <= physical.parent_absence_observed_at()
            || namespace.post_absence_observed_at() <= namespace.barrier_completed_at()
            || namespace.completed_at() < namespace.post_absence_observed_at()
            || observed_at <= namespace.completed_at()
            || prepared_at <= observed_at
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
            || trusted_now.timestamp_millis() <= physical.absence_event().event().recorded_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_SESSION_INVALID");
        }
        Ok(
            ComputePluginCandidateCleanupNamespaceDurabilityAuthoritySession {
                authority: self,
                process_fence,
                trusted_now,
                observed_at,
                clock_epoch_digest: observation.clock_epoch_digest().to_string(),
            },
        )
    }
}

impl ComputePluginCandidateCleanupNamespaceDurabilityAuthoritySession<'_> {
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

    pub(in crate::node_agent_compute_plugin_host) fn validate_candidate_cleanup_namespace_durability(
        &self,
        physical: &PhysicallyDurableCandidateCleanupNamespace,
        event: &HashedComputePluginCandidateCleanupStepEvent,
    ) -> Result<()> {
        self.validate_source(physical.state().deletion_guard())?;
        let _operation = physical.state().deletion_guard().enter_operation()?;
        self.authority.with_deferred(|transaction| {
            validation::validate_unstored_namespace_durability(transaction, self, physical, event)
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn persist_candidate_cleanup_namespace_durability(
        &self,
        permit: ValidatedCandidateCleanupNamespaceDurabilityPermit<'_>,
    ) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
        self.validate_source(permit.physical().state().deletion_guard())?;
        let _operation = permit
            .physical()
            .state()
            .deletion_guard()
            .enter_operation()?;
        self.authority.with_immediate(|transaction| {
            write::persist_candidate_cleanup_namespace_durability(transaction, self, permit)
        })
    }
}
