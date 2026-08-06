use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        DurableCandidateCleanupTerminalJournal, ValidatedCandidateCleanupCompletionPermit,
    },
    fetch_contract::ComputePluginFetchCancellationGuard,
    lifecycle::ComputePluginInventorySnapshot,
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod binding;
mod meta;
mod projection;
mod recovery;
mod terminal;
mod types;
mod write;

pub(in crate::node_agent_compute_plugin_host) use recovery::{
    ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession,
    ComputePluginCandidateCleanupCompletionRecoveryOutcome,
};
pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginCandidateCleanupCompletionReceipt,
    HashedComputePluginCandidateCleanupCompletionReceipt,
    CANDIDATE_CLEANUP_COMPLETION_RECEIPT_CANONICALIZATION,
    CANDIDATE_CLEANUP_COMPLETION_RECEIPT_DIGEST_ALGORITHM,
    CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
    HASHED_CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupCompletionAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupCompletionAuthorityFacts
{
    authority_state_revision_before: i64,
    authority_state_revision_after: i64,
    inventory_revision_before: i64,
    inventory_revision_after: i64,
    inventory_digest_before: String,
    inventory_digest_after: String,
    inventory_json_after: String,
    inventory_after: ComputePluginInventorySnapshot,
    authority_epoch_before: i64,
    authority_epoch_after: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms_before: i64,
    completed_at_ms: i64,
    candidate_token_digest: String,
    cleanup_id: String,
    authorization_receipt_digest: String,
    execution_plan_digest: String,
    execution_evidence_digest: String,
    terminal_journal_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_completion_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        physical_completed_at: Instant,
    ) -> Result<ComputePluginCandidateCleanupCompletionAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || !is_sha256(observation.installation_id_digest())
            || !is_sha256(observation.clock_epoch_digest())
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observed_at <= process_fence.acquired_observed_at()
            || observed_at <= physical_completed_at
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_SESSION_INVALID");
        }
        Ok(ComputePluginCandidateCleanupCompletionAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
        })
    }
}

impl ComputePluginCandidateCleanupCompletionAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }
    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_fence.process_owner_epoch()
    }
    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn observed_at(&self) -> Instant {
        self.observed_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn trusted_now_ms(&self) -> i64 {
        self.trusted_now.timestamp_millis()
    }
    pub(in crate::node_agent_compute_plugin_host) fn validate_source(
        &self,
        guard: &ComputePluginFetchCancellationGuard,
    ) -> Result<()> {
        guard.validate_source(self.process_fence.cancellation_source())?;
        guard.ensure_current()
    }
    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_cleanup_completion_binding(
        &self,
        terminal: &DurableCandidateCleanupTerminalJournal,
    ) -> Result<ComputePluginCandidateCleanupCompletionAuthorityFacts> {
        self.validate_source(terminal.physical().cancellation_guard())?;
        self.authority.with_deferred(|transaction| {
            binding::read_candidate_cleanup_completion_binding(transaction, self, terminal)
        })
    }
    pub(in crate::node_agent_compute_plugin_host) fn persist_candidate_cleanup_completion(
        &self,
        permit: ValidatedCandidateCleanupCompletionPermit<'_, '_>,
    ) -> Result<HashedComputePluginCandidateCleanupCompletionReceipt> {
        self.authority.with_immediate(|transaction| {
            write::persist_candidate_cleanup_completion(transaction, self, permit)
        })
    }
}

impl ComputePluginCandidateCleanupCompletionAuthorityFacts {
    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision_before(&self) -> i64 {
        self.authority_state_revision_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision_after(&self) -> i64 {
        self.authority_state_revision_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision_before(&self) -> i64 {
        self.inventory_revision_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision_after(&self) -> i64 {
        self.inventory_revision_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest_before(&self) -> &str {
        &self.inventory_digest_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest_after(&self) -> &str {
        &self.inventory_digest_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_json_after(&self) -> &str {
        &self.inventory_json_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_after(
        &self,
    ) -> &ComputePluginInventorySnapshot {
        &self.inventory_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch_before(&self) -> i64 {
        self.authority_epoch_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch_after(&self) -> i64 {
        self.authority_epoch_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_owner_epoch
    }
    pub(in crate::node_agent_compute_plugin_host) fn trusted_time_high_water_ms_before(
        &self,
    ) -> i64 {
        self.trusted_time_high_water_ms_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn completed_at_ms(&self) -> i64 {
        self.completed_at_ms
    }
    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn cleanup_id(&self) -> &str {
        &self.cleanup_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn authorization_receipt_digest(&self) -> &str {
        &self.authorization_receipt_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn execution_plan_digest(&self) -> &str {
        &self.execution_plan_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn execution_evidence_digest(&self) -> &str {
        &self.execution_evidence_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn terminal_journal_digest(&self) -> &str {
        &self.terminal_journal_digest
    }
}
