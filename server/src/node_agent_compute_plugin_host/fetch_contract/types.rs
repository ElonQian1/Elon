use std::fmt;

use crate::node_agent_compute_plugin_host::{
    install_plan_admission::AdmittedComputePluginDownload,
    local_authority::ComputePluginFetchAuthorityFacts,
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

/// A side-effect-free authoritative read of persisted plan, key, candidate, download, inventory,
/// trusted-time and process fences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ComputePluginFetchAuthoritySnapshot {
    pub(super) store: ComputePluginFetchAuthorityFacts,
}

/// Unforgeable proof that the contract validated this exact request against this exact fresh
/// snapshot. Store CAS accepts this permit instead of caller-provided scalar authority facts.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedComputePluginFetchClaimPermit<'permit>
{
    plan_id: &'permit str,
    plan_digest: &'permit str,
    request: &'permit ComputePluginDownloadSegmentRequest,
    snapshot: &'permit ComputePluginFetchAuthoritySnapshot,
}

impl<'permit> ValidatedComputePluginFetchClaimPermit<'permit> {
    pub(super) fn new(
        plan_id: &'permit str,
        plan_digest: &'permit str,
        request: &'permit ComputePluginDownloadSegmentRequest,
        snapshot: &'permit ComputePluginFetchAuthoritySnapshot,
    ) -> Self {
        Self {
            plan_id,
            plan_digest,
            request,
            snapshot,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan_id(&self) -> &str {
        self.plan_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan_digest(&self) -> &str {
        self.plan_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn ordinal(&self) -> usize {
        self.request.ordinal
    }

    pub(in crate::node_agent_compute_plugin_host) fn offset_bytes(&self) -> i64 {
        self.request.offset_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn length_bytes(&self) -> i64 {
        self.request.length_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn redirect_hop(&self) -> u8 {
        self.request.redirect_hop
    }

    pub(in crate::node_agent_compute_plugin_host) fn redirect_from_claim_id(&self) -> Option<&str> {
        self.request.redirect_from_claim_id.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn facts(
        &self,
    ) -> &ComputePluginFetchAuthorityFacts {
        &self.snapshot.store
    }
}
