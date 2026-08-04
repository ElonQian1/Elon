use std::fmt;

use crate::node_agent_compute_plugin_host::{
    fetch_contract::recovery::ComputePluginFetchClaimRecoveryKey,
    install_plan_admission::AdmittedComputePluginDownload,
    local_authority::{ComputePluginFetchAuthorityFacts, ComputePluginFetchAuthoritySession},
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
    pub(super) recovery_key: ComputePluginFetchClaimRecoveryKey,
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

    pub(in crate::node_agent_compute_plugin_host) fn ordinal(&self) -> usize {
        self.claim.ordinal
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.claim.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn part_relative_path(&self) -> &str {
        &self.claim.part_relative_path
    }

    pub(in crate::node_agent_compute_plugin_host) fn artifact_digest(&self) -> &str {
        &self.download.download.digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn artifact_size_bytes(&self) -> i64 {
        self.download.download.size_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn end_offset_bytes(&self) -> i64 {
        self.claim.end_offset_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.recovery_key.installation_id_digest()
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

/// Fixed internal audit vocabulary for a downloader-owned abort. Arbitrary transport or peer
/// strings never cross into the authority database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComputePluginFetchAbortReason {
    DownloaderCanceled,
    TransportFailed,
    DurableWriteFailed,
    FileBindingMismatch,
    AuthorityRecovery,
}

impl ComputePluginFetchAbortReason {
    pub(in crate::node_agent_compute_plugin_host) fn as_str(self) -> &'static str {
        match self {
            Self::DownloaderCanceled => "downloader_canceled",
            Self::TransportFailed => "transport_failed",
            Self::DurableWriteFailed => "durable_write_failed",
            Self::FileBindingMismatch => "file_binding_mismatch",
            Self::AuthorityRecovery => "authority_recovery",
        }
    }
}

/// Opaque evidence that a future downloader kept the exact `.part` file open through a successful
/// durability barrier and observed the claimed end offset on that same file identity. There is no
/// production constructor until the downloader/file-handle layer lands, so Store commit cannot be
/// reached from a caller-provided `fsynced: bool` or scalar length assertion.
pub(crate) struct DurablyWrittenComputePluginSegment<'authority> {
    pub(super) _file: std::fs::File,
    pub(super) resolution_session: ComputePluginFetchAuthoritySession<'authority>,
    pub(super) claim_id: String,
    pub(super) part_relative_path: String,
    pub(super) file_identity_digest: String,
    pub(super) cursor_generation: i64,
    pub(super) offset_bytes: i64,
    pub(super) end_offset_bytes: i64,
    pub(super) durable_file_length_bytes: i64,
    pub(super) fsync_through_offset_bytes: i64,
    pub(super) durably_synced_at_ms: i64,
}

impl fmt::Debug for DurablyWrittenComputePluginSegment<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurablyWrittenComputePluginSegment")
            .field("claim_id", &"<redacted>")
            .field("part_relative_path", &self.part_relative_path)
            .field("file_identity_digest", &"<redacted>")
            .field("cursor_generation", &self.cursor_generation)
            .field("offset_bytes", &self.offset_bytes)
            .field("end_offset_bytes", &self.end_offset_bytes)
            .field("durable_file_length_bytes", &self.durable_file_length_bytes)
            .field(
                "fsync_through_offset_bytes",
                &self.fsync_through_offset_bytes,
            )
            .field("durably_synced_at_ms", &self.durably_synced_at_ms)
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommittedComputePluginDownloadSegment {
    pub(crate) ordinal: usize,
    pub(crate) committed_offset: i64,
    pub(crate) artifact_complete: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AbortedComputePluginDownloadSegment {
    pub(crate) ordinal: usize,
    pub(crate) committed_offset: i64,
    pub(crate) reason: ComputePluginFetchAbortReason,
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
    claim_id: &'permit str,
    request: &'permit ComputePluginDownloadSegmentRequest,
    snapshot: &'permit ComputePluginFetchAuthoritySnapshot,
}

impl<'permit> ValidatedComputePluginFetchClaimPermit<'permit> {
    pub(super) fn new(
        plan_id: &'permit str,
        plan_digest: &'permit str,
        claim_id: &'permit str,
        request: &'permit ComputePluginDownloadSegmentRequest,
        snapshot: &'permit ComputePluginFetchAuthoritySnapshot,
    ) -> Self {
        Self {
            plan_id,
            plan_digest,
            claim_id,
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

    pub(in crate::node_agent_compute_plugin_host) fn claim_id(&self) -> &str {
        self.claim_id
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

/// One-shot proof that the contract bound an opaque durable file observation to the current exact
/// prepared claim and a fresh authority snapshot.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedComputePluginFetchCommitPermit<
    'permit,
> {
    claim: &'permit PreparedComputePluginFetchClaim,
    snapshot: &'permit ComputePluginFetchAuthoritySnapshot,
    file_identity_digest: &'permit str,
}

impl<'permit> ValidatedComputePluginFetchCommitPermit<'permit> {
    pub(super) fn new(
        claim: &'permit PreparedComputePluginFetchClaim,
        snapshot: &'permit ComputePluginFetchAuthoritySnapshot,
        durable: &'permit DurablyWrittenComputePluginSegment<'_>,
    ) -> Self {
        Self {
            claim,
            snapshot,
            file_identity_digest: &durable.file_identity_digest,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn claim_id(&self) -> &str {
        &self.claim.claim_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan_id(&self) -> &str {
        &self.claim.plan_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan_digest(&self) -> &str {
        &self.claim.plan_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn ordinal(&self) -> usize {
        self.claim.ordinal
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.claim.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch(&self) -> i64 {
        self.claim.authority_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.claim.process_owner_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn cursor_generation(&self) -> i64 {
        self.claim.cursor_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn redirect_generation(&self) -> i64 {
        self.claim.redirect_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn offset_bytes(&self) -> i64 {
        self.claim.offset_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn length_bytes(&self) -> i64 {
        self.claim.length_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn end_offset_bytes(&self) -> i64 {
        self.claim.end_offset_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn prepared_at_ms(&self) -> i64 {
        self.claim.prepared_at_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn part_relative_path(&self) -> &str {
        &self.claim.part_relative_path
    }

    pub(in crate::node_agent_compute_plugin_host) fn file_identity_digest(&self) -> &str {
        self.file_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn facts(
        &self,
    ) -> &ComputePluginFetchAuthorityFacts {
        &self.snapshot.store
    }
}

/// One-shot proof derived from an authorized handle. Abort deliberately needs no file evidence and
/// may clean up an exact prepared claim after transport or durable-write failure.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedComputePluginFetchAbortPermit<'permit>
{
    claim: &'permit PreparedComputePluginFetchClaim,
    reason: ComputePluginFetchAbortReason,
}

impl<'permit> ValidatedComputePluginFetchAbortPermit<'permit> {
    pub(super) fn new(
        claim: &'permit PreparedComputePluginFetchClaim,
        reason: ComputePluginFetchAbortReason,
    ) -> Self {
        Self { claim, reason }
    }

    pub(in crate::node_agent_compute_plugin_host) fn claim_id(&self) -> &str {
        &self.claim.claim_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan_id(&self) -> &str {
        &self.claim.plan_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan_digest(&self) -> &str {
        &self.claim.plan_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn ordinal(&self) -> usize {
        self.claim.ordinal
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.claim.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch(&self) -> i64 {
        self.claim.authority_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.claim.process_owner_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn cursor_generation(&self) -> i64 {
        self.claim.cursor_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn redirect_generation(&self) -> i64 {
        self.claim.redirect_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn offset_bytes(&self) -> i64 {
        self.claim.offset_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn length_bytes(&self) -> i64 {
        self.claim.length_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn end_offset_bytes(&self) -> i64 {
        self.claim.end_offset_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn prepared_at_ms(&self) -> i64 {
        self.claim.prepared_at_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn part_relative_path(&self) -> &str {
        &self.claim.part_relative_path
    }

    pub(in crate::node_agent_compute_plugin_host) fn reason(
        &self,
    ) -> ComputePluginFetchAbortReason {
        self.reason
    }
}
