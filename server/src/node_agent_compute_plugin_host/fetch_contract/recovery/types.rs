use std::fmt;

use crate::node_agent_compute_plugin_host::local_authority::ComputePluginAuthorityInstanceBinding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginFetchClaimOutcomeKind {
    NotCreated,
    Prepared,
    Committed,
    Aborted,
    Revoked,
}

/// Exact pre-mutation facts retained only while an initial claim Store result may still be an
/// actual rollback. A successful Store return removes this snapshot before any handle escapes.
#[derive(PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginFetchInitialClaimAbsenceSnapshot {
    pub(super) expected_redirect_generation: i64,
    pub(super) authority_state_revision: i64,
    pub(super) trusted_time_high_water_ms: i64,
    pub(super) download_committed_offset: i64,
    pub(super) download_cursor_generation: i64,
    pub(super) download_state: String,
    pub(super) download_updated_at_ms: i64,
}

impl ComputePluginFetchInitialClaimAbsenceSnapshot {
    pub(in crate::node_agent_compute_plugin_host) fn expected_redirect_generation(&self) -> i64 {
        self.expected_redirect_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision(&self) -> i64 {
        self.authority_state_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_time_high_water_ms(&self) -> i64 {
        self.trusted_time_high_water_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn download_committed_offset(&self) -> i64 {
        self.download_committed_offset
    }

    pub(in crate::node_agent_compute_plugin_host) fn download_cursor_generation(&self) -> i64 {
        self.download_cursor_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn download_state(&self) -> &str {
        &self.download_state
    }

    pub(in crate::node_agent_compute_plugin_host) fn download_updated_at_ms(&self) -> i64 {
        self.download_updated_at_ms
    }
}

/// Non-authorizing, non-serializable identity probe retained before a claim handle is consumed.
/// It deliberately stores the observed redirect generation as a lower bound rather than immutable
/// identity, because a redirect CAS may have succeeded even when its commit result was uncertain.
#[derive(PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginFetchClaimRecoveryKey {
    pub(super) authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    pub(super) installation_id_digest: String,
    pub(super) claim_id: String,
    pub(super) plan_id: String,
    pub(super) plan_digest: String,
    pub(super) ordinal: usize,
    pub(super) candidate_token_digest: String,
    pub(super) part_relative_path: String,
    pub(super) artifact_digest: String,
    pub(super) artifact_size_bytes: i64,
    pub(super) authority_epoch: i64,
    pub(super) process_owner_epoch: i64,
    pub(super) cursor_generation: i64,
    pub(super) observed_redirect_generation: i64,
    pub(super) offset_bytes: i64,
    pub(super) length_bytes: i64,
    pub(super) end_offset_bytes: i64,
    pub(super) prepared_at_ms: i64,
    pub(super) initial_absence: Option<ComputePluginFetchInitialClaimAbsenceSnapshot>,
}

impl fmt::Debug for ComputePluginFetchClaimRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginFetchClaimRecoveryKey")
            .field("installation_id_digest", &"<redacted>")
            .field("claim_id", &"<redacted>")
            .field("plan_id", &self.plan_id)
            .field("ordinal", &self.ordinal)
            .field("cursor_generation", &self.cursor_generation)
            .field(
                "observed_redirect_generation",
                &self.observed_redirect_generation,
            )
            .field("offset_bytes", &self.offset_bytes)
            .field("end_offset_bytes", &self.end_offset_bytes)
            .field(
                "initial_absence",
                &self.initial_absence.as_ref().map(|_| "<retained>"),
            )
            .finish()
    }
}

impl ComputePluginFetchClaimRecoveryKey {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        &self.authority_instance_binding
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn ordinal(&self) -> usize {
        self.ordinal
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn part_relative_path(&self) -> &str {
        &self.part_relative_path
    }

    pub(in crate::node_agent_compute_plugin_host) fn artifact_digest(&self) -> &str {
        &self.artifact_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn artifact_size_bytes(&self) -> i64 {
        self.artifact_size_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch(&self) -> i64 {
        self.authority_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_owner_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn cursor_generation(&self) -> i64 {
        self.cursor_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn observed_redirect_generation(&self) -> i64 {
        self.observed_redirect_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn offset_bytes(&self) -> i64 {
        self.offset_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn length_bytes(&self) -> i64 {
        self.length_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn end_offset_bytes(&self) -> i64 {
        self.end_offset_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn prepared_at_ms(&self) -> i64 {
        self.prepared_at_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn initial_absence(
        &self,
    ) -> Option<&ComputePluginFetchInitialClaimAbsenceSnapshot> {
        self.initial_absence.as_ref()
    }
}

#[derive(PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginFetchClaimOutcome {
    kind: ComputePluginFetchClaimOutcomeKind,
    ordinal: usize,
    actual_redirect_generation: Option<i64>,
    current_authority_epoch: i64,
    current_process_owner_epoch: i64,
    current_cursor_generation: i64,
    current_committed_offset: i64,
    current_download_state: String,
    resolved_at_ms: Option<i64>,
    resolution_reason: Option<&'static str>,
}

impl fmt::Debug for ComputePluginFetchClaimOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginFetchClaimOutcome")
            .field("kind", &self.kind)
            .field("ordinal", &self.ordinal)
            .field(
                "actual_redirect_generation",
                &self.actual_redirect_generation,
            )
            .field("current_cursor_generation", &self.current_cursor_generation)
            .field("current_committed_offset", &self.current_committed_offset)
            .field("current_download_state", &self.current_download_state)
            .field("resolved_at_ms", &self.resolved_at_ms)
            .field("resolution_reason", &self.resolution_reason)
            .finish()
    }
}

impl ComputePluginFetchClaimOutcome {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_compute_plugin_host) fn from_store(
        kind: ComputePluginFetchClaimOutcomeKind,
        ordinal: usize,
        actual_redirect_generation: Option<i64>,
        current_authority_epoch: i64,
        current_process_owner_epoch: i64,
        current_cursor_generation: i64,
        current_committed_offset: i64,
        current_download_state: String,
        resolved_at_ms: Option<i64>,
        resolution_reason: Option<&'static str>,
    ) -> Self {
        Self {
            kind,
            ordinal,
            actual_redirect_generation,
            current_authority_epoch,
            current_process_owner_epoch,
            current_cursor_generation,
            current_committed_offset,
            current_download_state,
            resolved_at_ms,
            resolution_reason,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn kind(
        &self,
    ) -> ComputePluginFetchClaimOutcomeKind {
        self.kind
    }

    pub(in crate::node_agent_compute_plugin_host) fn actual_redirect_generation(
        &self,
    ) -> Option<i64> {
        self.actual_redirect_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn resolved_at_ms(&self) -> Option<i64> {
        self.resolved_at_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn resolution_reason(
        &self,
    ) -> Option<&'static str> {
        self.resolution_reason
    }
}

/// One-shot proof that the contract observed this exact claim as prepared immediately before the
/// Store recovery transaction. Store re-reads and compares it inside `BEGIN IMMEDIATE`.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedComputePluginFetchRecoveryAbortPermit<
    'permit,
> {
    key: &'permit ComputePluginFetchClaimRecoveryKey,
    observed: &'permit ComputePluginFetchClaimOutcome,
}

impl<'permit> ValidatedComputePluginFetchRecoveryAbortPermit<'permit> {
    pub(super) fn new(
        key: &'permit ComputePluginFetchClaimRecoveryKey,
        observed: &'permit ComputePluginFetchClaimOutcome,
    ) -> Self {
        Self { key, observed }
    }

    pub(in crate::node_agent_compute_plugin_host) fn key(
        &self,
    ) -> &ComputePluginFetchClaimRecoveryKey {
        self.key
    }

    pub(in crate::node_agent_compute_plugin_host) fn observed(
        &self,
    ) -> &ComputePluginFetchClaimOutcome {
        self.observed
    }
}
