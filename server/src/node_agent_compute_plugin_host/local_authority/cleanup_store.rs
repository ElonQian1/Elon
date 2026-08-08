use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority, PreparedCandidateCleanupDeletionGuard,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::ValidatedCandidateCleanupAuthorizationPermit,
    candidate_health_contract::DurableCandidateHealthQuarantine,
    fetch_contract::ComputePluginFetchCancellationGuard, manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(super) mod binding;
mod recovery;
mod types;
mod write;

pub(in crate::node_agent_compute_plugin_host) use recovery::{
    ComputePluginCandidateCleanupRecoveryAuthoritySession,
    ComputePluginCandidateCleanupRecoveryOutcome,
};
pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginCandidateCleanupAuthorizationReceipt,
    HashedComputePluginCandidateCleanupAuthorizationReceipt,
    CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION,
    CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM,
    CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
    HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupAuthorityFacts {
    authority_state_revision_before: i64,
    authority_state_revision_after: i64,
    inventory_revision: i64,
    inventory_digest: String,
    authority_epoch_before: i64,
    authority_epoch_after: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms_before: i64,
    authorized_at_ms: i64,
    candidate_token_digest: String,
    quarantine_id: String,
    quarantine_receipt_digest: String,
    staging_id: String,
    staging_run_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateCleanupAuthoritySession<'authority>> {
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
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORITY_SESSION_INVALID");
        }
        Ok(ComputePluginCandidateCleanupAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest,
        })
    }
}

impl ComputePluginCandidateCleanupAuthoritySession<'_> {
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
    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_cleanup_binding(
        &self,
        quarantined: &DurableCandidateHealthQuarantine<'_>,
    ) -> Result<ComputePluginCandidateCleanupAuthorityFacts> {
        let guard = quarantined.staged().archive().snapshot_cancellation_guard();
        self.validate_source(&guard)?;
        self.authority.with_deferred(|transaction| {
            binding::read_candidate_cleanup_binding(transaction, self, quarantined)
        })
    }
    pub(in crate::node_agent_compute_plugin_host) fn persist_candidate_cleanup_authorization(
        &self,
        permit: ValidatedCandidateCleanupAuthorizationPermit<'_, '_>,
    ) -> Result<HashedComputePluginCandidateCleanupAuthorizationReceipt> {
        let guard = permit
            .quarantined()
            .staged()
            .archive()
            .snapshot_cancellation_guard();
        self.validate_source(&guard)?;
        self.authority.with_immediate(|transaction| {
            write::persist_candidate_cleanup_authorization(transaction, self, permit)
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn prepare_cleanup_deletion_guard(
        &self,
        cleanup_id: String,
        facts: &ComputePluginCandidateCleanupAuthorityFacts,
        root_identity_digest: String,
    ) -> Result<PreparedCandidateCleanupDeletionGuard> {
        self.process_fence.prepare_candidate_cleanup_deletion_guard(
            cleanup_id,
            facts.candidate_token_digest.clone(),
            facts.quarantine_id.clone(),
            facts.quarantine_receipt_digest.clone(),
            facts.staging_id.clone(),
            facts.staging_run_digest.clone(),
            root_identity_digest,
        )
    }
}

macro_rules! cleanup_fact_getter {
    ($name:ident, $field:ident, i64) => {
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> i64 {
            self.$field
        }
    };
    ($name:ident, $field:ident, str) => {
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> &str {
            &self.$field
        }
    };
}

impl ComputePluginCandidateCleanupAuthorityFacts {
    cleanup_fact_getter!(
        authority_state_revision_before,
        authority_state_revision_before,
        i64
    );
    cleanup_fact_getter!(
        authority_state_revision_after,
        authority_state_revision_after,
        i64
    );
    cleanup_fact_getter!(inventory_revision, inventory_revision, i64);
    cleanup_fact_getter!(inventory_digest, inventory_digest, str);
    cleanup_fact_getter!(authority_epoch_before, authority_epoch_before, i64);
    cleanup_fact_getter!(authority_epoch_after, authority_epoch_after, i64);
    cleanup_fact_getter!(process_owner_epoch, process_owner_epoch, i64);
    cleanup_fact_getter!(
        trusted_time_high_water_ms_before,
        trusted_time_high_water_ms_before,
        i64
    );
    cleanup_fact_getter!(authorized_at_ms, authorized_at_ms, i64);
    cleanup_fact_getter!(candidate_token_digest, candidate_token_digest, str);
    cleanup_fact_getter!(quarantine_id, quarantine_id, str);
    cleanup_fact_getter!(quarantine_receipt_digest, quarantine_receipt_digest, str);
    cleanup_fact_getter!(staging_id, staging_id, str);
    cleanup_fact_getter!(staging_run_digest, staging_run_digest, str);
}
