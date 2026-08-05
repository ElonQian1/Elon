use std::time::Instant;

use anyhow::Error;
use serde::Serialize;

use super::super::ValidatedComputePluginArchiveExtractionPlan;
use crate::{
    node_agent_compute_plugin_host::{
        candidate_verification_contract::VerifiedComputePluginCandidateArtifactSet,
        fetch_file::PreparedComputePluginCandidateStaging,
    },
    node_agent_managed_fs::{PinnedManagedDirectory, PinnedManagedFile},
};

pub(super) const EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA: &str =
    "elon.compute_plugin.extracted_archive_evidence.v1";
pub(super) const HASHED_EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA: &str =
    "elon.compute_plugin.hashed_extracted_archive_evidence.v1";

#[derive(Debug, Serialize)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginExtractedFileEvidence {
    pub relative_path: String,
    pub digest: String,
    pub size_bytes: i64,
    pub file_identity_digest: String,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginExtractedArchiveEvidence {
    pub schema: String,
    pub evidence: ComputePluginExtractedArchiveEvidence,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub evidence_digest: String,
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
    pub(super) completed_at: Instant,
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
}
