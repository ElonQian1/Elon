use std::time::Instant;

use anyhow::Result;

use super::{outcome, ComputePluginCandidateVerificationRecoveryAuthoritySession};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::{
        ComputePluginCandidateVerificationOutcome, ComputePluginCandidateVerificationOutcomeKind,
        ComputePluginCandidateVerificationRecoveryKey,
    },
    fetch_contract::ComputePluginFetchCancellationGuard,
    keyring::ComputePluginBootstrapRootKeyResolver,
    local_authority::{
        ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority,
    },
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

/// Purpose-specific trusted session minted only from an authenticated observation acquired after
/// hashing. It wraps the recovery reader so exact-run classification and provenance checks remain
/// single-sourced, while retaining the monotonic observation point needed by the hash barrier.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPostHashVerificationAuthoritySession<
    'authority,
> {
    recovery_session: ComputePluginCandidateVerificationRecoveryAuthoritySession<'authority>,
    observed_at: Instant,
    roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
}

/// Read-only S3 projection retained by the linear post-hash capability. The resolution kernel
/// must compare every field against its own transaction-local S4 replay before writing.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPostHashVerificationBindingFacts {
    pub outcome: ComputePluginCandidateVerificationOutcome,
    pub authority_state_revision: i64,
    pub inventory_revision: i64,
    pub inventory_digest: String,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub trusted_time_high_water_ms: i64,
    pub durable_candidate_closure_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_post_hash_verification_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginPostHashVerificationAuthoritySession<'authority>> {
        let observed_at = observation.observed_at();
        let recovery_session =
            self.candidate_verification_recovery_session(process_fence, observation)?;
        Ok(ComputePluginPostHashVerificationAuthoritySession {
            recovery_session,
            observed_at,
            roots,
        })
    }
}

impl ComputePluginPostHashVerificationAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.recovery_session.authority_instance_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.recovery_session.process_fence.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.recovery_session.process_fence.process_owner_epoch()
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        self.recovery_session.clock_epoch_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_now_ms(&self) -> i64 {
        self.recovery_session.trusted_now.timestamp_millis()
    }

    pub(in crate::node_agent_compute_plugin_host) fn bootstrap_roots(
        &self,
    ) -> &dyn ComputePluginBootstrapRootKeyResolver {
        self.roots
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_post_hash_source(
        &self,
        guard: &ComputePluginFetchCancellationGuard,
    ) -> Result<()> {
        guard.validate_source(self.recovery_session.process_fence.cancellation_source())?;
        guard.ensure_current()
    }

    pub(in crate::node_agent_compute_plugin_host) fn was_observed_strictly_after(
        &self,
        barrier: Instant,
    ) -> bool {
        self.observed_at > barrier && is_sha256(self.clock_epoch_digest())
    }

    /// Read-only S3 gate. Resolution must still replay current admission and repeat this exact
    /// classification inside its own `BEGIN IMMEDIATE` transaction before changing durable facts.
    pub(in crate::node_agent_compute_plugin_host) fn read_prepared_candidate_verification_binding(
        &self,
        key: &ComputePluginCandidateVerificationRecoveryKey,
    ) -> Result<ComputePluginPostHashVerificationBindingFacts> {
        if !key
            .authority_instance_binding()
            .matches(self.authority_instance_binding())
            || key.installation_id_digest() != self.installation_id_digest()
            || key.clock_epoch_digest() != self.clock_epoch_digest()
            || key.process_owner_epoch() != self.process_owner_epoch()
        {
            anyhow::bail!("COMPUTE_PLUGIN_VERIFICATION_POST_HASH_PROVENANCE_CHANGED");
        }
        self.recovery_session
            .authority
            .with_deferred(|transaction| {
                let snapshot = outcome::read_outcome_snapshot(
                    transaction,
                    self.recovery_session.process_fence,
                    key,
                )?;
                if snapshot.outcome.kind()
                    != ComputePluginCandidateVerificationOutcomeKind::Prepared
                    || snapshot.authority.clock_status != "trusted"
                    || self.trusted_now_ms() <= snapshot.authority.trusted_time_high_water_ms
                    || self.trusted_now_ms() <= key.prepared_at_ms()
                {
                    anyhow::bail!("COMPUTE_PLUGIN_VERIFICATION_POST_HASH_RUN_CHANGED");
                }
                let closure = snapshot.closure.ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_POST_HASH_CLOSURE_MISSING")
                })?;
                Ok(ComputePluginPostHashVerificationBindingFacts {
                    outcome: snapshot.outcome,
                    authority_state_revision: snapshot.authority.state_revision,
                    inventory_revision: snapshot.authority.inventory_revision,
                    inventory_digest: snapshot.authority.inventory_digest,
                    authority_epoch: snapshot.authority.authority_epoch,
                    process_owner_epoch: snapshot.authority.process_owner_epoch,
                    trusted_time_high_water_ms: snapshot.authority.trusted_time_high_water_ms,
                    durable_candidate_closure_digest: closure.durable_closure_digest,
                })
            })
    }
}
