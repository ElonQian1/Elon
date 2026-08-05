use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error, Result as AnyhowResult};

use super::{
    candidate_extraction::ExtractedComputePluginCandidateArchive,
    candidate_verification_contract::ComputePluginCandidateVerificationOutcomeKind,
    fetch_contract::ComputePluginFetchCancellationGuard,
    local_authority::{
        ComputePluginCandidateStagingAuthorityFacts,
        ComputePluginPostRevalidationStagingAuthoritySession,
    },
    manifest_validation::is_sha256,
};

/// Linear custody proving every retained extracted file and the staging seal were re-hashed on
/// their original handles. A later trusted-time session must be observed after `revalidated_at`.
#[must_use = "revalidated candidate staging must be resolved by the local authority Store"]
pub(in crate::node_agent_compute_plugin_host) struct RevalidatedComputePluginCandidateStaging<'root>
{
    archive: ExtractedComputePluginCandidateArchive<'root>,
    revalidated_at: Instant,
    cancellation_guard: ComputePluginFetchCancellationGuard,
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

#[must_use = "authorized candidate staging must be consumed by its Store transaction"]
pub(in crate::node_agent_compute_plugin_host) struct AuthorizedComputePluginCandidateStaging<
    'root,
    'authority,
> {
    revalidated: RevalidatedComputePluginCandidateStaging<'root>,
    authority_session: ComputePluginPostRevalidationStagingAuthoritySession<'authority>,
    binding: ComputePluginCandidateStagingAuthorityFacts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateStagingAuthorityBindingPhase {
    PreStoreAuthorityRead,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateStagingAuthorityBindingFailure<'root>
{
    phase: CandidateStagingAuthorityBindingPhase,
    error: Error,
    revalidated: RevalidatedComputePluginCandidateStaging<'root>,
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

impl<'root> CandidateStagingAuthorityBindingFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateStagingAuthorityBindingPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, RevalidatedComputePluginCandidateStaging<'root>) {
        (self.error, self.revalidated)
    }
}

impl fmt::Debug for CandidateStagingAuthorityBindingFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateStagingAuthorityBindingFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("revalidated", &self.revalidated)
            .finish()
    }
}

impl fmt::Display for CandidateStagingAuthorityBindingFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for CandidateStagingAuthorityBindingFailure<'_> {}

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

pub(in crate::node_agent_compute_plugin_host) fn authorize_revalidated_candidate_staging<
    'root,
    'authority,
>(
    revalidated: RevalidatedComputePluginCandidateStaging<'root>,
    authority_session: ComputePluginPostRevalidationStagingAuthoritySession<'authority>,
) -> Result<
    AuthorizedComputePluginCandidateStaging<'root, 'authority>,
    CandidateStagingAuthorityBindingFailure<'root>,
> {
    match bind_authority(&revalidated, &authority_session) {
        Ok(binding) => Ok(AuthorizedComputePluginCandidateStaging {
            revalidated,
            authority_session,
            binding,
        }),
        Err(error) => Err(CandidateStagingAuthorityBindingFailure {
            phase: CandidateStagingAuthorityBindingPhase::PreStoreAuthorityRead,
            error,
            revalidated,
        }),
    }
}

fn bind_authority(
    revalidated: &RevalidatedComputePluginCandidateStaging<'_>,
    authority_session: &ComputePluginPostRevalidationStagingAuthoritySession<'_>,
) -> AnyhowResult<ComputePluginCandidateStagingAuthorityFacts> {
    let archive = revalidated.archive();
    let key = archive.verification_recovery_key();
    let outcome = archive.verification_outcome();
    let result_digest = outcome
        .result_digest()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_VERIFICATION_RESULT_MISSING"))?;
    let evidence = &archive.evidence().evidence;
    if outcome.kind() != ComputePluginCandidateVerificationOutcomeKind::Verified
        || outcome.resolution_reason() != Some("artifact_set_verified")
        || outcome.mismatch().is_some()
        || !is_sha256(result_digest)
        || !authority_session.was_observed_strictly_after(revalidated.revalidated_at())
        || authority_session.installation_id_digest() != evidence.installation_id_digest.as_str()
        || key.root_identity_digest() != evidence.root_identity_digest.as_str()
        || key.candidate_token_digest() != evidence.candidate_token_digest.as_str()
    {
        bail!("COMPUTE_PLUGIN_STAGING_AUTHORITY_INPUT_CHANGED");
    }
    authority_session.validate_source(revalidated.cancellation_guard())?;
    let binding = authority_session.read_verified_candidate_staging_binding(key, result_digest)?;
    if binding.verification_result_digest() != result_digest
        || binding.verification_resolved_at_ms() != outcome.resolved_at_ms().unwrap_or(-1)
        || binding.authority_state_revision()
            != outcome.authority_state_revision_after().unwrap_or(-1)
        || binding.inventory_revision() != outcome.inventory_revision_after().unwrap_or(-1)
        || binding.inventory_digest() != outcome.inventory_digest_after().unwrap_or("")
        || binding.authority_epoch() != outcome.authority_epoch_after().unwrap_or(-1)
        || binding.process_owner_epoch() != key.process_owner_epoch()
        || binding.candidate_token_digest() != key.candidate_token_digest()
        || binding.candidate_generation() != key.candidate_generation()
        || binding.application_inventory_revision() != key.application_inventory_revision()
        || binding.candidate_release() != &archive.plan().envelope().plan.release
        || binding.candidate_plugin_id()
            != archive.plan().envelope().plan.release.plugin_id.as_str()
        || binding.trusted_time_high_water_ms() >= authority_session.trusted_now_ms()
    {
        bail!("COMPUTE_PLUGIN_STAGING_AUTHORITY_BINDING_CHANGED");
    }
    authority_session.validate_source(revalidated.cancellation_guard())?;
    Ok(binding)
}
