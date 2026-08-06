use std::time::Instant;

use anyhow::Error;
use serde::{Deserialize, Serialize};

use super::super::ValidatedComputePluginArchiveExtractionPlan;
use crate::{
    node_agent_compute_plugin_host::{
        candidate_verification_contract::{
            ComputePluginCandidateVerificationOutcome,
            ComputePluginCandidateVerificationRecoveryKey,
            VerifiedComputePluginCandidateArtifactSet,
        },
        fetch_contract::ComputePluginFetchCancellationGuard,
        fetch_file::PreparedComputePluginCandidateStaging,
    },
    node_agent_managed_fs::{PinnedManagedDirectory, PinnedManagedFile},
};

pub(in crate::node_agent_compute_plugin_host) const EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA: &str =
    "elon.compute_plugin.extracted_archive_evidence.v1";
pub(in crate::node_agent_compute_plugin_host) const HASHED_EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA: &str =
    "elon.compute_plugin.hashed_extracted_archive_evidence.v1";
pub(in crate::node_agent_compute_plugin_host) const STAGING_SEAL_PAYLOAD_SCHEMA: &str =
    "elon.compute_plugin.staging_seal_payload.v1";
pub(in crate::node_agent_compute_plugin_host) const STAGING_SEAL_EVIDENCE_SCHEMA: &str =
    "elon.compute_plugin.staging_seal_evidence.v1";
pub(in crate::node_agent_compute_plugin_host) const STAGING_EVIDENCE_CANONICALIZATION: &str =
    "RFC8785-JCS";
pub(in crate::node_agent_compute_plugin_host) const STAGING_EVIDENCE_DIGEST_ALGORITHM: &str =
    "sha256";

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginExtractedFileEvidence {
    pub relative_path: String,
    pub digest: String,
    pub size_bytes: i64,
    pub file_identity_digest: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginExtractedArchiveEvidence {
    pub schema: String,
    pub installation_id_digest: String,
    pub root_identity_digest: String,
    pub candidate_token_digest: String,
    pub staging_run_digest: String,
    pub extraction_plan_digest: String,
    pub extracted_file_count: i64,
    pub extracted_bytes: i64,
    pub files: Vec<ComputePluginExtractedFileEvidence>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginExtractedArchiveEvidence {
    pub schema: String,
    pub evidence: ComputePluginExtractedArchiveEvidence,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub evidence_digest: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginStagingSealPayload {
    pub schema: String,
    pub installation_id_digest: String,
    pub root_identity_digest: String,
    pub candidate_token_digest: String,
    pub staging_run_digest: String,
    pub extraction_plan_digest: String,
    pub extraction_evidence_digest: String,
    pub extracted_file_count: i64,
    pub extracted_bytes: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginStagingSealEvidence {
    pub schema: String,
    pub payload: ComputePluginStagingSealPayload,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub payload_digest: String,
    pub file_digest: String,
    pub file_identity_digest: String,
    pub size_bytes: i64,
}

/// Extracted files remain handle-pinned beside the raw verified artifacts. This custody is not a
/// durable Store receipt and cannot be used as an installed, healthy or promotable slot.
#[must_use = "extracted archive custody must be resolved by the candidate staging Store"]
pub(in crate::node_agent_compute_plugin_host) struct ExtractedComputePluginCandidateArchive<'root> {
    pub(super) plan: ValidatedComputePluginArchiveExtractionPlan,
    pub(super) evidence: HashedComputePluginExtractedArchiveEvidence,
    pub(super) verified: VerifiedComputePluginCandidateArtifactSet,
    pub(super) staging: PreparedComputePluginCandidateStaging<'root>,
    pub(super) directories: Vec<PinnedManagedDirectory>,
    pub(super) files: Vec<PinnedManagedFile>,
    pub(super) seal: PinnedManagedFile,
    pub(super) seal_evidence: ComputePluginStagingSealEvidence,
    pub(super) completed_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct ExtractedComputePluginCandidateCleanupParts<
    'root,
> {
    pub(in crate::node_agent_compute_plugin_host) evidence:
        HashedComputePluginExtractedArchiveEvidence,
    pub(in crate::node_agent_compute_plugin_host) verified:
        VerifiedComputePluginCandidateArtifactSet,
    pub(in crate::node_agent_compute_plugin_host) staging:
        PreparedComputePluginCandidateStaging<'root>,
    pub(in crate::node_agent_compute_plugin_host) directories:
        Vec<(String, PinnedManagedDirectory)>,
    pub(in crate::node_agent_compute_plugin_host) files: Vec<(String, String, PinnedManagedFile)>,
    pub(in crate::node_agent_compute_plugin_host) seal: PinnedManagedFile,
    pub(in crate::node_agent_compute_plugin_host) seal_evidence: ComputePluginStagingSealEvidence,
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginArchiveExtractionFailure {
    pub(super) error: Error,
    pub(super) verified: VerifiedComputePluginCandidateArtifactSet,
    pub(super) staging_run_digest: Option<String>,
    pub(super) filesystem_mutated: bool,
}

impl ComputePluginArchiveExtractionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn filesystem_mutated(&self) -> bool {
        self.filesystem_mutated
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_run_digest(&self) -> Option<&str> {
        self.staging_run_digest.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        Error,
        VerifiedComputePluginCandidateArtifactSet,
        Option<String>,
        bool,
    ) {
        (
            self.error,
            self.verified,
            self.staging_run_digest,
            self.filesystem_mutated,
        )
    }
}

impl std::fmt::Debug for ComputePluginArchiveExtractionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComputePluginArchiveExtractionFailure")
            .field("staging_run_digest", &"<redacted>")
            .field("filesystem_mutated", &self.filesystem_mutated)
            .finish()
    }
}

impl std::fmt::Debug for ExtractedComputePluginCandidateArchive<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExtractedComputePluginCandidateArchive")
            .field("plan_digest", &"<redacted>")
            .field("evidence_digest", &"<redacted>")
            .field("staging", &self.staging)
            .field("directory_count", &self.directories.len())
            .field("file_count", &self.files.len())
            .field("seal", &"<retained>")
            .field("completed_at", &"<monotonic>")
            .finish()
    }
}

impl ExtractedComputePluginCandidateArchive<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> &ValidatedComputePluginArchiveExtractionPlan {
        &self.plan
    }

    pub(in crate::node_agent_compute_plugin_host) fn evidence(
        &self,
    ) -> &HashedComputePluginExtractedArchiveEvidence {
        &self.evidence
    }

    pub(in crate::node_agent_compute_plugin_host) fn completed_at(&self) -> Instant {
        self.completed_at
    }

    pub(in crate::node_agent_compute_plugin_host) fn seal_evidence(
        &self,
    ) -> &ComputePluginStagingSealEvidence {
        &self.seal_evidence
    }

    pub(in crate::node_agent_compute_plugin_host) fn verification_recovery_key(
        &self,
    ) -> &ComputePluginCandidateVerificationRecoveryKey {
        self.verified.recovery_key()
    }

    pub(in crate::node_agent_compute_plugin_host) fn verification_outcome(
        &self,
    ) -> &ComputePluginCandidateVerificationOutcome {
        self.verified.outcome()
    }

    pub(in crate::node_agent_compute_plugin_host) fn snapshot_cancellation_guard(
        &self,
    ) -> ComputePluginFetchCancellationGuard {
        self.verified.snapshot_cancellation_guard()
    }

    pub(in crate::node_agent_compute_plugin_host) fn pin_cleanup_ancestors(
        &self,
    ) -> anyhow::Result<(PinnedManagedDirectory, PinnedManagedDirectory)> {
        self.staging
            .pin_cleanup_ancestors(&self.evidence.evidence.candidate_token_digest)
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_cleanup_custody(
        &self,
    ) -> anyhow::Result<()> {
        if self.files.len() != self.evidence.evidence.files.len()
            || self.directories.len() != self.plan.envelope().plan.directories.len()
            || self.seal.identity_digest() != self.seal_evidence.file_identity_digest
        {
            anyhow::bail!("COMPUTE_PLUGIN_CLEANUP_STAGING_CUSTODY_CHANGED");
        }
        for (file, evidence) in self.files.iter().zip(&self.evidence.evidence.files) {
            if file.identity_digest() != evidence.file_identity_digest {
                anyhow::bail!("COMPUTE_PLUGIN_CLEANUP_STAGING_FILE_CHANGED");
            }
        }
        Ok(())
    }
}

impl<'root> ExtractedComputePluginCandidateArchive<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_cleanup_parts(
        self,
    ) -> ExtractedComputePluginCandidateCleanupParts<'root> {
        let directories = self
            .plan
            .envelope()
            .plan
            .directories
            .iter()
            .cloned()
            .zip(self.directories)
            .collect();
        let files = self
            .evidence
            .evidence
            .files
            .iter()
            .map(|file| (file.relative_path.clone(), file.digest.clone()))
            .zip(self.files)
            .map(|((path, digest), file)| (path, digest, file))
            .collect();
        ExtractedComputePluginCandidateCleanupParts {
            evidence: self.evidence,
            verified: self.verified,
            staging: self.staging,
            directories,
            files,
            seal: self.seal,
            seal_evidence: self.seal_evidence,
        }
    }
}
