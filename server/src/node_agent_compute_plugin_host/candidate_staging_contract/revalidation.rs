use std::{error::Error as StdError, fmt};

use anyhow::Error;

use super::capability::RevalidatedComputePluginCandidateStaging;
use crate::node_agent_compute_plugin_host::candidate_extraction::ExtractedComputePluginCandidateArchive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateStagingRevalidationPhase {
    PinnedContent,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateStagingRevalidationFailure<'root> {
    phase: CandidateStagingRevalidationPhase,
    error: Error,
    archive: ExtractedComputePluginCandidateArchive<'root>,
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
        Ok(revalidated_at) => {
            let cancellation_guard = archive.snapshot_cancellation_guard();
            Ok(RevalidatedComputePluginCandidateStaging {
                archive,
                revalidated_at,
                cancellation_guard,
            })
        }
        Err(error) => Err(CandidateStagingRevalidationFailure {
            phase: CandidateStagingRevalidationPhase::PinnedContent,
            error,
            archive,
        }),
    }
}
