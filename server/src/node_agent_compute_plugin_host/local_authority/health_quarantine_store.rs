use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::{
        ValidatedCandidateHealthFailurePublication, ValidatedCandidateHealthQuarantinePermit,
    },
    fetch_contract::ComputePluginFetchCancellationGuard,
    identity::ComputePluginReleaseRef,
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod binding;
mod meta;
mod projection;
mod recovery;
mod types;
mod write;

pub(in crate::node_agent_compute_plugin_host) use recovery::{
    ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession,
    ComputePluginCandidateHealthQuarantineRecoveryOutcome,
};
pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginCandidateHealthQuarantineReceipt,
    HashedComputePluginCandidateHealthQuarantineReceipt,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateHealthQuarantineAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateHealthQuarantineAuthorityFacts
{
    authority_state_revision_before: i64,
    authority_state_revision_after: i64,
    inventory_revision_before: i64,
    inventory_revision_after: i64,
    inventory_digest_before: String,
    inventory_digest_after: String,
    authority_epoch_before: i64,
    authority_epoch_after: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms_before: i64,
    failed_at_ms: i64,
    candidate_token_digest: String,
    staging_id: String,
    staging_receipt_digest: String,
    staging_run_digest: String,
    plugin_id: String,
    slot_ref: String,
    release: ComputePluginReleaseRef,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_health_quarantine_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: &ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateHealthQuarantineAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observed_at <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_AUTHORITY_SESSION_INVALID");
        }
        Ok(ComputePluginCandidateHealthQuarantineAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
        })
    }
}

impl ComputePluginCandidateHealthQuarantineAuthoritySession<'_> {
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

    pub(in crate::node_agent_compute_plugin_host) fn validate_source(
        &self,
        guard: &ComputePluginFetchCancellationGuard,
    ) -> Result<()> {
        guard.validate_source(self.process_fence.cancellation_source())?;
        guard.ensure_current()
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_health_quarantine_binding(
        &self,
        publication: &ValidatedCandidateHealthFailurePublication<'_>,
    ) -> Result<ComputePluginCandidateHealthQuarantineAuthorityFacts> {
        let guard = publication.staged().archive().snapshot_cancellation_guard();
        self.validate_source(&guard)?;
        self.authority.with_deferred(|transaction| {
            binding::read_candidate_health_quarantine_binding(transaction, self, publication)
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn persist_candidate_health_quarantine(
        &self,
        permit: ValidatedCandidateHealthQuarantinePermit<'_, '_>,
    ) -> Result<HashedComputePluginCandidateHealthQuarantineReceipt> {
        self.authority.with_immediate(|transaction| {
            write::persist_candidate_health_quarantine(transaction, self, permit)
        })
    }
}

impl ComputePluginCandidateHealthQuarantineAuthorityFacts {
    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision_before(&self) -> i64 {
        self.authority_state_revision_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision_after(&self) -> i64 {
        self.authority_state_revision_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision_before(&self) -> i64 {
        self.inventory_revision_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision_after(&self) -> i64 {
        self.inventory_revision_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest_before(&self) -> &str {
        &self.inventory_digest_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest_after(&self) -> &str {
        &self.inventory_digest_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch_before(&self) -> i64 {
        self.authority_epoch_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch_after(&self) -> i64 {
        self.authority_epoch_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_owner_epoch
    }
    pub(in crate::node_agent_compute_plugin_host) fn trusted_time_high_water_ms_before(
        &self,
    ) -> i64 {
        self.trusted_time_high_water_ms_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn failed_at_ms(&self) -> i64 {
        self.failed_at_ms
    }
}
