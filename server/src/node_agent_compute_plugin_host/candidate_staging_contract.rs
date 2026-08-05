use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::Error;

use super::candidate_extraction::ExtractedComputePluginCandidateArchive;

/// Linear custody proving every retained extracted file and the staging seal were re-hashed on
/// their original handles. A later trusted-time session must be observed after `revalidated_at`.
#[must_use = "revalidated candidate staging must be resolved by the local authority Store"]
pub(in crate::node_agent_compute_plugin_host) struct RevalidatedComputePluginCandidateStaging<'root>
{
    archive: ExtractedComputePluginCandidateArchive<'root>,
    revalidated_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateStagingRevalidationPhase {
    PinnedContent,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateStagingRevalidationFailure<'root> {
    phase: CandidateStagingRevalidationPhase,
    error: Error,
    archive: ExtractedComputePluginCandidateArchive<'root>,
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
}

impl<'root> CandidateStagingRevalidationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateStagingRevalidationPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, ExtractedComputePluginCandidateArchive<'root>) {
        (self.error, self.archive)
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

impl fmt::Debug for CandidateStagingRevalidationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateStagingRevalidationFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("archive", &self.archive)
            .finish()
    }
}

impl fmt::Display for CandidateStagingRevalidationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for CandidateStagingRevalidationFailure<'_> {}

pub(in crate::node_agent_compute_plugin_host) fn revalidate_extracted_candidate_for_staging<
    'root,
>(
    mut archive: ExtractedComputePluginCandidateArchive<'root>,
) -> Result<
    RevalidatedComputePluginCandidateStaging<'root>,
    CandidateStagingRevalidationFailure<'root>,
> {
    match archive.revalidate_for_staging_store() {
        Ok(revalidated_at) => Ok(RevalidatedComputePluginCandidateStaging {
            archive,
            revalidated_at,
        }),
        Err(error) => Err(CandidateStagingRevalidationFailure {
            phase: CandidateStagingRevalidationPhase::PinnedContent,
            error,
            archive,
        }),
    }
}
