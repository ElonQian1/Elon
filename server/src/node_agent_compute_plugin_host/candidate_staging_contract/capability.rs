use std::{fmt, time::Instant};

use super::super::{
    candidate_extraction::ExtractedComputePluginCandidateArchive,
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

impl fmt::Debug for RevalidatedComputePluginCandidateStaging<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RevalidatedComputePluginCandidateStaging")
            .field("archive", &self.archive)
            .field("revalidated_at", &"<monotonic>")
            .finish()
    }
}
