use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::super::{
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, trusted_time::ComputePluginTrustedTimeObservation,
};

/// Read-only authority for classifying one uncertain candidate-staging Store outcome.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateStagingRecoveryAuthoritySession<
    'authority,
> {
    pub(super) authority: &'authority ComputePluginLocalAuthority,
    pub(super) process_fence: &'authority ComputePluginFetchProcessFence,
    pub(super) trusted_now: DateTime<Utc>,
    clock_epoch_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn candidate_staging_recovery_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateStagingRecoveryAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let clock_epoch_digest = observation.clock_epoch_digest().to_string();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observation.observed_at() <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_SESSION_INVALID");
        }
        Ok(ComputePluginCandidateStagingRecoveryAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            clock_epoch_digest,
        })
    }
}

impl ComputePluginCandidateStagingRecoveryAuthoritySession<'_> {
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
}
