use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    verification_store::read_verified_candidate_staging_snapshot,
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
    fetch_contract::ComputePluginFetchCancellationGuard, identity::ComputePluginReleaseRef,
    keyring::ComputePluginBootstrapRootKeyResolver, manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod types;

pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginCandidateStagingReceipt, HashedComputePluginCandidateStagingReceipt,
};

/// A purpose-specific authority capability minted from authenticated time after file revalidation.
/// It is read-only until a later linear staging permit is supplied.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPostRevalidationStagingAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
    roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateStagingAuthorityFacts {
    verification_result_digest: String,
    verification_resolved_at_ms: i64,
    authority_state_revision: i64,
    inventory_revision: i64,
    inventory_digest: String,
    authority_epoch: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms: i64,
    candidate_token_digest: String,
    candidate_generation: i64,
    application_inventory_revision: i64,
    candidate_plugin_id: String,
    candidate_slot_ref: String,
    candidate_release: ComputePluginReleaseRef,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_post_revalidation_staging_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginPostRevalidationStagingAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
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
            || observed_at <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_STAGING_AUTHORITY_SESSION_INVALID");
        }
        Ok(ComputePluginPostRevalidationStagingAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest,
            roots,
        })
    }
}

impl ComputePluginPostRevalidationStagingAuthoritySession<'_> {
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

    pub(in crate::node_agent_compute_plugin_host) fn was_observed_strictly_after(
        &self,
        barrier: Instant,
    ) -> bool {
        self.observed_at > barrier && is_sha256(self.clock_epoch_digest())
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_source(
        &self,
        guard: &ComputePluginFetchCancellationGuard,
    ) -> Result<()> {
        guard.validate_source(self.process_fence.cancellation_source())?;
        guard.ensure_current()
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_verified_candidate_staging_binding(
        &self,
        key: &ComputePluginCandidateVerificationRecoveryKey,
        expected_result_digest: &str,
    ) -> Result<ComputePluginCandidateStagingAuthorityFacts> {
        if !key
            .authority_instance_binding()
            .matches(self.authority_instance_binding())
            || key.installation_id_digest() != self.installation_id_digest()
            || key.clock_epoch_digest() != self.clock_epoch_digest()
            || key.process_owner_epoch() != self.process_owner_epoch()
            || !is_sha256(expected_result_digest)
        {
            bail!("COMPUTE_PLUGIN_STAGING_AUTHORITY_PROVENANCE_CHANGED");
        }
        self.authority.with_deferred(|transaction| {
            let snapshot = read_verified_candidate_staging_snapshot(
                transaction,
                self.process_fence,
                self.trusted_now.clone(),
                self.roots,
                key,
                expected_result_digest,
            )?;
            let outcome = snapshot.outcome;
            let current = snapshot.current;
            Ok(ComputePluginCandidateStagingAuthorityFacts {
                verification_result_digest: expected_result_digest.to_string(),
                verification_resolved_at_ms: outcome.resolved_at_ms().ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_VERIFICATION_TIME_MISSING")
                })?,
                authority_state_revision: current.authority_state_revision,
                inventory_revision: current.execution_inventory_revision,
                inventory_digest: current.inventory_digest,
                authority_epoch: current.authority_epoch,
                process_owner_epoch: current.process_owner_epoch,
                trusted_time_high_water_ms: current.observed_trusted_time_high_water_ms,
                candidate_token_digest: current.candidate_token_digest,
                candidate_generation: current.candidate_generation,
                application_inventory_revision: current.candidate_application_inventory_revision,
                candidate_plugin_id: current.candidate_plugin_id,
                candidate_slot_ref: current.candidate_slot_ref,
                candidate_release: current.candidate_release,
            })
        })
    }
}

impl ComputePluginCandidateStagingAuthorityFacts {
    pub(in crate::node_agent_compute_plugin_host) fn verification_result_digest(&self) -> &str {
        &self.verification_result_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn verification_resolved_at_ms(&self) -> i64 {
        self.verification_resolved_at_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision(&self) -> i64 {
        self.authority_state_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision(&self) -> i64 {
        self.inventory_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest(&self) -> &str {
        &self.inventory_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch(&self) -> i64 {
        self.authority_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_owner_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_time_high_water_ms(&self) -> i64 {
        self.trusted_time_high_water_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_generation(&self) -> i64 {
        self.candidate_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn application_inventory_revision(&self) -> i64 {
        self.application_inventory_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_plugin_id(&self) -> &str {
        &self.candidate_plugin_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_slot_ref(&self) -> &str {
        &self.candidate_slot_ref
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_release(
        &self,
    ) -> &ComputePluginReleaseRef {
        &self.candidate_release
    }
}
