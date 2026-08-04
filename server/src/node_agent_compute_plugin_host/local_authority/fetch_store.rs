use std::{fmt, time::Instant};

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    verification_store::{
        self, ComputePluginCandidateVerificationAuthorityFacts,
        ComputePluginPostPinVerificationAuthoritySession,
    },
    ComputePluginFetchProcessFence, ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    fetch_contract::{
        ComputePluginFetchCancellationGuard, ValidatedComputePluginFetchAbortPermit,
        ValidatedComputePluginFetchClaimPermit, ValidatedComputePluginFetchCommitPermit,
    },
    identity::ComputePluginReleaseRef,
    install_plan::ComputePluginPlannedDownload,
    install_plan_admission::ComputePluginLiveAdmissionState,
    keyring::ComputePluginBootstrapRootKeyResolver,
    lifecycle::ComputePluginInventorySnapshot,
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod claim;
mod read;
mod recovery;
mod resolution;

/// One authenticated trusted-time observation paired with the process-owner fence that may later
/// perform the exact claim CAS. It carries no network or filesystem capability.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginFetchAuthoritySession<'authority>
{
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
}

/// A fetch authority session that can only be minted from a sealed trusted-time observation. Its
/// process-local monotonic point is retained so durable binding can prove the observation happened
/// after the exact file handle crossed its fsync and identity barrier.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPostSyncFetchAuthoritySession<
    'authority,
> {
    authority_session: ComputePluginFetchAuthoritySession<'authority>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

/// Store-native projection used by the fetch contract. The candidate token never leaves the
/// transaction; only its digest is projected across this boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginFetchAuthorityFacts {
    pub inventory: ComputePluginInventorySnapshot,
    pub live: ComputePluginLiveAdmissionState,
    pub trusted_now: DateTime<Utc>,
    pub observed_trusted_time_high_water_ms: i64,
    pub applied_plan_id: String,
    pub applied_plan_digest: String,
    pub application_inventory_revision: i64,
    pub execution_inventory_revision: i64,
    pub authority_state_revision: i64,
    pub inventory_digest: String,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub candidate_token_digest: String,
    pub candidate_generation: i64,
    pub candidate_owner_plan_id: String,
    pub candidate_owner_plan_digest: String,
    pub candidate_application_inventory_revision: i64,
    pub candidate_state: String,
    pub candidate_release: ComputePluginReleaseRef,
    pub candidate_permission_grant_digest: String,
    pub slot_ref: String,
    pub planned_download: ComputePluginPlannedDownload,
    pub part_relative_path: String,
    pub committed_offset: i64,
    pub download_cursor_generation: i64,
    pub download_state: String,
    pub download_updated_at_ms: i64,
    pub prepared_claim: Option<ComputePluginPreparedFetchClaimFacts>,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPreparedFetchClaimFacts {
    pub claim_id: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub ordinal: usize,
    pub candidate_token_digest: String,
    pub part_relative_path: String,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub cursor_generation: i64,
    pub redirect_generation: i64,
    pub offset_bytes: i64,
    pub length_bytes: i64,
    pub end_offset_bytes: i64,
    pub prepared_at_ms: i64,
}

impl fmt::Debug for ComputePluginPreparedFetchClaimFacts {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginPreparedFetchClaimFacts")
            .field("claim_id", &"<redacted>")
            .field("plan_id", &self.plan_id)
            .field("ordinal", &self.ordinal)
            .field("cursor_generation", &self.cursor_generation)
            .field("redirect_generation", &self.redirect_generation)
            .field("offset_bytes", &self.offset_bytes)
            .field("length_bytes", &self.length_bytes)
            .field("prepared_at_ms", &self.prepared_at_ms)
            .finish()
    }
}

impl ComputePluginLocalAuthority {
    /// Consumes one sealed trusted-time observation and a fence acquired after the NodeAgent
    /// instance lock. Opening a session does not touch SQLite.
    pub(in crate::node_agent_compute_plugin_host) fn fetch_authority_session<'authority>(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginFetchAuthoritySession<'authority>> {
        self.fetch_authority_session_from_observation(process_fence, &observation, roots)
    }

    fn fetch_authority_session_from_observation<'authority>(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: &ComputePluginTrustedTimeObservation,
        roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginFetchAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observation.observed_at() <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_FETCH_SESSION_FENCE_INVALID");
        }
        Ok(ComputePluginFetchAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            roots,
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn bind_post_sync_fetch_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginPostSyncFetchAuthoritySession<'authority>> {
        if !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
        {
            bail!("COMPUTE_PLUGIN_POST_SYNC_TIME_BINDING_INVALID");
        }
        let observed_at = observation.observed_at();
        let clock_epoch_digest = observation.clock_epoch_digest().to_string();
        let authority_session =
            self.fetch_authority_session_from_observation(process_fence, &observation, roots)?;
        Ok(ComputePluginPostSyncFetchAuthoritySession {
            authority_session,
            observed_at,
            clock_epoch_digest,
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn bind_post_pin_verification_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginPostPinVerificationAuthoritySession<'authority>> {
        if !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
        {
            bail!("COMPUTE_PLUGIN_POST_PIN_TIME_BINDING_INVALID");
        }
        let observed_at = observation.observed_at();
        let clock_epoch_digest = observation.clock_epoch_digest().to_string();
        let authority_session =
            self.fetch_authority_session_from_observation(process_fence, &observation, roots)?;
        Ok(ComputePluginPostPinVerificationAuthoritySession::new(
            authority_session,
            observed_at,
            clock_epoch_digest,
        ))
    }
}

impl<'authority> ComputePluginPostSyncFetchAuthoritySession<'authority> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_session(
        &self,
    ) -> &ComputePluginFetchAuthoritySession<'authority> {
        &self.authority_session
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_now_ms(&self) -> i64 {
        self.authority_session.trusted_now.timestamp_millis()
    }

    pub(in crate::node_agent_compute_plugin_host) fn was_observed_strictly_after(
        &self,
        barrier: Instant,
    ) -> bool {
        self.observed_at > barrier && is_sha256(&self.clock_epoch_digest)
    }
}

impl ComputePluginFetchAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_fetch_cancellation_guard(
        &self,
        guard: &ComputePluginFetchCancellationGuard,
    ) -> Result<()> {
        guard.validate_source(self.process_fence.cancellation_source())
    }

    pub(in crate::node_agent_compute_plugin_host) fn recovery_authority_instance_binding(
        &self,
    ) -> &super::ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }

    /// Rebuilds all authority from one stable read snapshot. This method never advances trusted
    /// time and never creates or mutates a fetch claim.
    pub(in crate::node_agent_compute_plugin_host) fn read_fresh_segment_authority(
        &self,
        plan_id: &str,
        plan_digest: &str,
        ordinal: usize,
    ) -> Result<ComputePluginFetchAuthorityFacts> {
        self.authority.with_deferred(|transaction| {
            read::read_fresh_segment_authority(
                transaction,
                self.process_fence,
                self.trusted_now.clone(),
                self.roots,
                plan_id,
                plan_digest,
                ordinal,
            )
        })
    }

    /// Reads one exact, signed candidate download closure without opening files or mutating Store.
    /// The caller must acquire a fresh post-pin session and compare durable projections before any
    /// future verification claim CAS.
    pub(in crate::node_agent_compute_plugin_host) fn read_fresh_candidate_verification_authority(
        &self,
        plan_id: &str,
        plan_digest: &str,
        candidate_token: &str,
    ) -> Result<ComputePluginCandidateVerificationAuthorityFacts> {
        self.authority.with_deferred(|transaction| {
            verification_store::read_fresh_candidate_verification_authority(
                transaction,
                self.process_fence,
                self.trusted_now.clone(),
                self.roots,
                plan_id,
                plan_digest,
                candidate_token,
            )
        })
    }

    /// Contract-only CAS seam. A commit error has an uncertain outcome and never returns a usable
    /// claim; the caller must discard the session and enter authority recovery.
    pub(in crate::node_agent_compute_plugin_host) fn claim_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchClaimPermit<'_>,
    ) -> Result<ComputePluginPreparedFetchClaimFacts> {
        let command = claim::FetchClaimCommand {
            plan_id: permit.plan_id(),
            plan_digest: permit.plan_digest(),
            ordinal: permit.ordinal(),
            offset_bytes: permit.offset_bytes(),
            length_bytes: permit.length_bytes(),
            redirect_generation: i64::from(permit.redirect_hop()),
            redirect_from_claim_id: permit.redirect_from_claim_id(),
            new_claim_id: (permit.redirect_hop() == 0).then(|| permit.claim_id()),
        };
        self.authority.with_immediate(|transaction| {
            claim::claim_validated_segment(
                transaction,
                self.process_fence,
                self.trusted_now.clone(),
                self.roots,
                &command,
                permit.facts(),
            )
        })
    }

    /// Contract-only commit seam. The permit is unreachable until the downloader can prove an
    /// exact file identity was durably synchronized through the claimed end offset.
    pub(in crate::node_agent_compute_plugin_host) fn commit_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchCommitPermit<'_>,
    ) -> Result<()> {
        self.authority.with_immediate(|transaction| {
            resolution::commit_validated_segment(
                transaction,
                self.process_fence,
                self.trusted_now.clone(),
                self.roots,
                permit,
            )
        })
    }

    /// Contract-only abort seam. A commit error is outcome-uncertain and consumes the old permit;
    /// callers must use the subsequent outcome/recovery layer instead of retrying this mutation.
    pub(in crate::node_agent_compute_plugin_host) fn abort_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchAbortPermit<'_>,
    ) -> Result<()> {
        self.authority.with_immediate(|transaction| {
            resolution::abort_validated_segment(
                transaction,
                self.process_fence,
                self.trusted_now.clone(),
                permit,
            )
        })
    }
}
