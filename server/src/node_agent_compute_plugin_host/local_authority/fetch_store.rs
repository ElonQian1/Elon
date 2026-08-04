use std::fmt;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{ComputePluginFetchProcessFence, ComputePluginLocalAuthority};
use crate::node_agent_compute_plugin_host::{
    fetch_contract::ValidatedComputePluginFetchClaimPermit, identity::ComputePluginReleaseRef,
    install_plan::ComputePluginPlannedDownload,
    install_plan_admission::ComputePluginLiveAdmissionState,
    keyring::ComputePluginBootstrapRootKeyResolver, lifecycle::ComputePluginInventorySnapshot,
};

mod claim;
mod read;

/// One authenticated trusted-time observation paired with the process-owner fence that may later
/// perform the exact claim CAS. It carries no network or filesystem capability.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginFetchAuthoritySession<'authority>
{
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
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
    /// The caller must supply trusted time from the authenticated time kernel and a fence acquired
    /// after the NodeAgent instance lock. Opening a session does not touch SQLite.
    pub(in crate::node_agent_compute_plugin_host) fn fetch_authority_session<'authority>(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        trusted_now: DateTime<Utc>,
        roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginFetchAuthoritySession<'authority>> {
        if process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
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
}

impl ComputePluginFetchAuthoritySession<'_> {
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

    /// Contract-only CAS seam. A commit error has an uncertain outcome and never returns a usable
    /// claim; the caller must discard the session and enter authority recovery.
    pub(in crate::node_agent_compute_plugin_host) fn claim_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchClaimPermit<'_>,
    ) -> Result<ComputePluginPreparedFetchClaimFacts> {
        let new_claim_id = (permit.redirect_hop() == 0)
            .then(|| format!("fetch_{}", uuid::Uuid::new_v4().simple()));
        let command = claim::FetchClaimCommand {
            plan_id: permit.plan_id(),
            plan_digest: permit.plan_digest(),
            ordinal: permit.ordinal(),
            offset_bytes: permit.offset_bytes(),
            length_bytes: permit.length_bytes(),
            redirect_generation: i64::from(permit.redirect_hop()),
            redirect_from_claim_id: permit.redirect_from_claim_id(),
            new_claim_id: new_claim_id.as_deref(),
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
}
