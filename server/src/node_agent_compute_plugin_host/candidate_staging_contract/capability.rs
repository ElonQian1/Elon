use std::{fmt, time::Instant};

use super::super::{
    candidate_extraction::{
        ComputePluginStagingSealEvidence, ExtractedComputePluginCandidateArchive,
        HashedComputePluginArchiveExtractionPlan, HashedComputePluginExtractedArchiveEvidence,
    },
    candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
    fetch_contract::ComputePluginFetchCancellationGuard,
    local_authority::{
        ComputePluginAuthorityInstanceBinding, ComputePluginCandidateStagingAuthorityFacts,
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

/// Process-local identity for classifying an uncertain staging commit. It is not a retry permit.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateStagingRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    staging_id: String,
    candidate_token: String,
    candidate_token_digest: String,
    verification_id: String,
    staging_run_digest: String,
    process_owner_epoch: i64,
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

impl fmt::Debug for ComputePluginCandidateStagingRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCandidateStagingRecoveryKey")
            .field("staging_id", &"<redacted>")
            .field("candidate_token", &"<redacted>")
            .field("candidate_token_digest", &self.candidate_token_digest)
            .field("verification_id", &"<redacted>")
            .field("staging_run_digest", &"<redacted>")
            .field("process_owner_epoch", &self.process_owner_epoch)
            .finish()
    }
}

impl ComputePluginCandidateStagingRecoveryKey {
    pub(super) fn from_authorized(
        authorized: &AuthorizedComputePluginCandidateStaging<'_, '_>,
        staging_id: String,
    ) -> Self {
        let key = authorized.revalidated.archive().verification_recovery_key();
        Self {
            authority_instance_binding: key.authority_instance_binding().clone(),
            installation_id_digest: key.installation_id_digest().to_string(),
            clock_epoch_digest: key.clock_epoch_digest().to_string(),
            staging_id,
            candidate_token: key.candidate_token().to_string(),
            candidate_token_digest: key.candidate_token_digest().to_string(),
            verification_id: key.verification_id().to_string(),
            staging_run_digest: authorized
                .revalidated
                .archive()
                .evidence()
                .evidence
                .staging_run_digest
                .clone(),
            process_owner_epoch: key.process_owner_epoch(),
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_id(&self) -> &str {
        &self.staging_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn verification_id(&self) -> &str {
        &self.verification_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_run_digest(&self) -> &str {
        &self.staging_run_digest
    }
}
