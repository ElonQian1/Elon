use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        CandidateCleanupParentAbsenceRecoveryKey, HashedComputePluginCandidateCleanupStepEvent,
        ObservedCandidateCleanupParentAbsence, ValidatedCandidateCleanupParentAbsencePermit,
    },
    fetch_contract::ComputePluginFetchCancellationGuard,
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod recovery;
mod validation;
mod write;

pub(in crate::node_agent_compute_plugin_host) use recovery::{
    ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession,
    ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupParentAbsenceAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_parent_absence_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        observed: &ObservedCandidateCleanupParentAbsence,
        prepared_at: Instant,
    ) -> Result<ComputePluginCandidateCleanupParentAbsenceAuthoritySession<'authority>> {
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
            || observed.observed_at() <= observed.disposition_set_at()
            || observed_at <= observed.observed_at()
            || prepared_at <= observed_at
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
            || trusted_now.timestamp_millis()
                <= observed.disposition_event().event().recorded_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_SESSION_INVALID");
        }
        Ok(ComputePluginCandidateCleanupParentAbsenceAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
        })
    }
}

impl ComputePluginCandidateCleanupParentAbsenceAuthoritySession<'_> {
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
        guard: &ComputePluginFetchCancellationGuard,
    ) -> Result<()> {
        guard.validate_source(self.process_fence.cancellation_source())?;
        guard.ensure_current()
    }
    pub(in crate::node_agent_compute_plugin_host) fn validate_candidate_cleanup_parent_absence(
        &self,
        observed: &ObservedCandidateCleanupParentAbsence,
        event: &HashedComputePluginCandidateCleanupStepEvent,
    ) -> Result<()> {
        self.validate_source(observed.state().cancellation_guard())?;
        self.authority.with_deferred(|transaction| {
            validation::validate_unstored_parent_absence(transaction, self, observed, event)
        })
    }
    pub(in crate::node_agent_compute_plugin_host) fn persist_candidate_cleanup_parent_absence(
        &self,
        permit: ValidatedCandidateCleanupParentAbsencePermit<'_>,
    ) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
        self.validate_source(permit.observed().state().cancellation_guard())?;
        self.authority.with_immediate(|transaction| {
            write::persist_candidate_cleanup_parent_absence(transaction, self, permit)
        })
    }
}
