use std::fmt;

use chrono::{DateTime, Utc};

use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan::ComputePluginPlannedDownload,
    install_plan_admission::{AdmittedComputePluginDownload, ComputePluginLiveAdmissionState},
    lifecycle::ComputePluginInventorySnapshot,
};

#[derive(PartialEq, Eq)]
pub(crate) struct ComputePluginDownloadSegmentRequest {
    pub(super) ordinal: usize,
    pub(super) offset_bytes: i64,
    pub(super) length_bytes: i64,
    pub(super) redirect_hop: u8,
    pub(super) redirect_from_claim_id: Option<String>,
}

impl ComputePluginDownloadSegmentRequest {
    pub(crate) fn initial(ordinal: usize, offset_bytes: i64, length_bytes: i64) -> Self {
        Self {
            ordinal,
            offset_bytes,
            length_bytes,
            redirect_hop: 0,
            redirect_from_claim_id: None,
        }
    }
}

impl fmt::Debug for ComputePluginDownloadSegmentRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginDownloadSegmentRequest")
            .field("ordinal", &self.ordinal)
            .field("offset_bytes", &self.offset_bytes)
            .field("length_bytes", &self.length_bytes)
            .field("redirect_hop", &self.redirect_hop)
            .field(
                "redirect_from_claim_id",
                &self.redirect_from_claim_id.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Debug)]
pub(crate) struct AuthorizedComputePluginDownloadSegment {
    pub(super) download: AdmittedComputePluginDownload,
    pub(super) offset_bytes: i64,
    pub(super) length_bytes: i64,
    pub(super) redirect_hop: u8,
    pub(super) claim: PreparedComputePluginFetchClaim,
}

impl AuthorizedComputePluginDownloadSegment {
    pub(crate) fn download(&self) -> &AdmittedComputePluginDownload {
        &self.download
    }

    pub(crate) fn offset_bytes(&self) -> i64 {
        self.offset_bytes
    }

    pub(crate) fn length_bytes(&self) -> i64 {
        self.length_bytes
    }

    pub(crate) fn redirect_hop(&self) -> u8 {
        self.redirect_hop
    }
}

#[derive(PartialEq, Eq)]
pub(crate) struct PreparedComputePluginFetchClaim {
    pub(super) claim_id: String,
    pub(super) plan_id: String,
    pub(super) plan_digest: String,
    pub(super) ordinal: usize,
    pub(super) candidate_token_digest: String,
    pub(super) part_relative_path: String,
    pub(super) authority_epoch: i64,
    pub(super) process_owner_epoch: i64,
    pub(super) cursor_generation: i64,
    pub(super) redirect_generation: i64,
    pub(super) offset_bytes: i64,
    pub(super) length_bytes: i64,
    pub(super) end_offset_bytes: i64,
    pub(super) prepared_at_ms: i64,
}

impl fmt::Debug for PreparedComputePluginFetchClaim {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedComputePluginFetchClaim")
            .field("claim_id", &"<redacted>")
            .field("plan_id", &self.plan_id)
            .field("plan_digest", &self.plan_digest)
            .field("ordinal", &self.ordinal)
            .field("candidate_token_digest", &self.candidate_token_digest)
            .field("part_relative_path", &self.part_relative_path)
            .field("authority_epoch", &self.authority_epoch)
            .field("process_owner_epoch", &self.process_owner_epoch)
            .field("cursor_generation", &self.cursor_generation)
            .field("redirect_generation", &self.redirect_generation)
            .field("offset_bytes", &self.offset_bytes)
            .field("length_bytes", &self.length_bytes)
            .field("end_offset_bytes", &self.end_offset_bytes)
            .field("prepared_at_ms", &self.prepared_at_ms)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct ComputePluginPreparedFetchClaimSnapshot {
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

impl fmt::Debug for ComputePluginPreparedFetchClaimSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginPreparedFetchClaimSnapshot")
            .field("claim_id", &"<redacted>")
            .field("plan_id", &self.plan_id)
            .field("ordinal", &self.ordinal)
            .field("cursor_generation", &self.cursor_generation)
            .field("redirect_generation", &self.redirect_generation)
            .field("offset_bytes", &self.offset_bytes)
            .field("length_bytes", &self.length_bytes)
            .finish()
    }
}

/// A side-effect-free authoritative read of persisted plan, key, candidate, download, inventory,
/// trusted-time and process fences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComputePluginFetchAuthoritySnapshot {
    pub(super) inventory: ComputePluginInventorySnapshot,
    pub(super) live: ComputePluginLiveAdmissionState,
    pub(super) trusted_now: DateTime<Utc>,
    pub(super) applied_plan_id: String,
    pub(super) applied_plan_digest: String,
    pub(super) application_inventory_revision: i64,
    pub(super) execution_inventory_revision: i64,
    pub(super) authority_state_revision: i64,
    pub(super) inventory_digest: String,
    pub(super) authority_epoch: i64,
    pub(super) process_owner_epoch: i64,
    pub(super) candidate_token_digest: String,
    pub(super) candidate_generation: i64,
    pub(super) candidate_owner_plan_id: String,
    pub(super) candidate_owner_plan_digest: String,
    pub(super) candidate_application_inventory_revision: i64,
    pub(super) candidate_state: String,
    pub(super) candidate_release: ComputePluginReleaseRef,
    pub(super) candidate_permission_grant_digest: String,
    pub(super) slot_ref: String,
    pub(super) planned_download: ComputePluginPlannedDownload,
    pub(super) part_relative_path: String,
    pub(super) committed_offset: i64,
    pub(super) download_cursor_generation: i64,
    pub(super) download_state: String,
    pub(super) prepared_claim: Option<ComputePluginPreparedFetchClaimSnapshot>,
}
