use std::{fmt, time::Instant};

use super::super::{
    candidate_extraction::{
        ComputePluginStagingSealEvidence, ExtractedComputePluginCandidateArchive,
        HashedComputePluginArchiveExtractionPlan, HashedComputePluginExtractedArchiveEvidence,
    },
    candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
    fetch_contract::ComputePluginFetchCancellationGuard,
    local_authority::{
        ComputePluginCandidateStagingAuthorityFacts,
        ComputePluginPostRevalidationStagingAuthoritySession,
    },
};

/// Linear custody proving every retained extracted file and the staging seal were re-hashed on
/// their original handles. A later trusted-time session must be observed after `revalidated_at`.
#[must_use = "revalidated candidate staging must be resolved by the local authority Store"]
pub(in crate::node_agent_compute_plugin_host) struct RevalidatedComputePluginCandidateStaging<'root>
{
    pub(super) archive: ExtractedComputePluginCandidateArchive<'root>,
    pub(super) revalidated_at: Instant,
    pub(super) cancellation_guard: ComputePluginFetchCancellationGuard,
}

#[must_use = "authorized candidate staging must be consumed by its Store transaction"]
pub(in crate::node_agent_compute_plugin_host) struct AuthorizedComputePluginCandidateStaging<
    'root,
    'authority,
> {
    pub(super) revalidated: RevalidatedComputePluginCandidateStaging<'root>,
    pub(super) authority_session: ComputePluginPostRevalidationStagingAuthoritySession<'authority>,
    pub(super) binding: ComputePluginCandidateStagingAuthorityFacts,
}

/// The only capability accepted by the staging Store. Every field borrows one authorized custody,
/// so callers cannot synthesize a receipt from detached scalar values.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateStagingStorePermit<'permit> {
    staging_id: &'permit str,
    key: &'permit ComputePluginCandidateVerificationRecoveryKey,
    binding: &'permit ComputePluginCandidateStagingAuthorityFacts,
    plan: &'permit HashedComputePluginArchiveExtractionPlan,
    evidence: &'permit HashedComputePluginExtractedArchiveEvidence,
    seal: &'permit ComputePluginStagingSealEvidence,
    cancellation_guard: &'permit ComputePluginFetchCancellationGuard,
}

impl<'root> RevalidatedComputePluginCandidateStaging<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn revalidated_at(&self) -> Instant {
        self.revalidated_at
    }

    pub(super) fn archive(&self) -> &ExtractedComputePluginCandidateArchive<'root> {
        &self.archive
    }

    pub(super) fn archive_mut(&mut self) -> &mut ExtractedComputePluginCandidateArchive<'root> {
        &mut self.archive
    }

    pub(super) fn cancellation_guard(&self) -> &ComputePluginFetchCancellationGuard {
        &self.cancellation_guard
    }
}

impl<'permit> ValidatedCandidateStagingStorePermit<'permit> {
    pub(super) fn new(
        authorized: &'permit AuthorizedComputePluginCandidateStaging<'_, '_>,
        staging_id: &'permit str,
    ) -> Self {
        let archive = authorized.revalidated.archive();
        Self {
            staging_id,
            key: archive.verification_recovery_key(),
            binding: &authorized.binding,
            plan: archive.plan().envelope(),
            evidence: archive.evidence(),
            seal: archive.seal_evidence(),
            cancellation_guard: authorized.revalidated.cancellation_guard(),
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_id(&self) -> &str {
        self.staging_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn key(
        &self,
    ) -> &ComputePluginCandidateVerificationRecoveryKey {
        self.key
    }

    pub(in crate::node_agent_compute_plugin_host) fn binding(
        &self,
    ) -> &ComputePluginCandidateStagingAuthorityFacts {
        self.binding
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> &HashedComputePluginArchiveExtractionPlan {
        self.plan
    }

    pub(in crate::node_agent_compute_plugin_host) fn evidence(
        &self,
    ) -> &HashedComputePluginExtractedArchiveEvidence {
        self.evidence
    }

    pub(in crate::node_agent_compute_plugin_host) fn seal(
        &self,
    ) -> &ComputePluginStagingSealEvidence {
        self.seal
    }

    pub(in crate::node_agent_compute_plugin_host) fn cancellation_guard(
        &self,
    ) -> &ComputePluginFetchCancellationGuard {
        self.cancellation_guard
    }
}

impl fmt::Debug for RevalidatedComputePluginCandidateStaging<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RevalidatedComputePluginCandidateStaging")
            .field("archive", &self.archive)
            .field("revalidated_at", &"<monotonic>")
            .finish()
    }
}
