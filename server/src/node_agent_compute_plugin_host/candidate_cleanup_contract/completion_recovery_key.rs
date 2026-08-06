use std::{fmt, time::Instant};

use super::{CandidateCleanupSlotExpectation, PreparedCandidateCleanupCompletion};
use crate::node_agent_compute_plugin_host::local_authority::ComputePluginAuthorityInstanceBinding;

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupCompletionReceiptExpectation {
    pub(in crate::node_agent_compute_plugin_host) cleanup_id: String,
    pub(in crate::node_agent_compute_plugin_host) candidate_token_digest: String,
    pub(in crate::node_agent_compute_plugin_host) authorization_receipt_digest: String,
    pub(in crate::node_agent_compute_plugin_host) execution_plan_digest: String,
    pub(in crate::node_agent_compute_plugin_host) execution_evidence_digest: String,
    pub(in crate::node_agent_compute_plugin_host) terminal_journal_digest: String,
    pub(in crate::node_agent_compute_plugin_host) authority_state_revision_before: i64,
    pub(in crate::node_agent_compute_plugin_host) authority_state_revision_after: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_revision_before: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_revision_after: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_digest_before: String,
    pub(in crate::node_agent_compute_plugin_host) inventory_digest_after: String,
    pub(in crate::node_agent_compute_plugin_host) authority_epoch_before: i64,
    pub(in crate::node_agent_compute_plugin_host) authority_epoch_after: i64,
    pub(in crate::node_agent_compute_plugin_host) process_owner_epoch: i64,
    pub(in crate::node_agent_compute_plugin_host) trusted_time_high_water_ms_before: i64,
    pub(in crate::node_agent_compute_plugin_host) completed_at_ms: i64,
}

/// Process-local identity for classifying one uncertain completion commit. It retains the exact
/// pre/post Store projection but cannot authorize another physical deletion.
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupCompletionRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    physical_completed_at: Instant,
    completion_id: String,
    candidate_token: String,
    receipt: CandidateCleanupCompletionReceiptExpectation,
    inventory_json_after: String,
    slot: CandidateCleanupSlotExpectation,
    candidate_generation: i64,
    owner_plan_id: String,
    owner_plan_digest: String,
    application_inventory_revision: i64,
}

impl CandidateCleanupCompletionRecoveryKey {
    pub(super) fn from_prepared(prepared: &PreparedCandidateCleanupCompletion<'_>) -> Self {
        let terminal = prepared.terminal();
        let physical = terminal.physical();
        let staging = physical.staging_recovery_key();
        let slot = staging.slot_expectation();
        let staging_receipt = staging.receipt_expectation();
        let facts = prepared.facts();
        Self {
            authority_instance_binding: prepared
                .authority_session()
                .authority_instance_binding()
                .clone(),
            installation_id_digest: prepared
                .authority_session()
                .installation_id_digest()
                .to_string(),
            clock_epoch_digest: prepared
                .authority_session()
                .clock_epoch_digest()
                .to_string(),
            physical_completed_at: physical.physical_completed_at(),
            completion_id: prepared.completion_id().to_string(),
            candidate_token: staging.candidate_token().to_string(),
            receipt: CandidateCleanupCompletionReceiptExpectation {
                cleanup_id: facts.cleanup_id().to_string(),
                candidate_token_digest: facts.candidate_token_digest().to_string(),
                authorization_receipt_digest: facts.authorization_receipt_digest().to_string(),
                execution_plan_digest: facts.execution_plan_digest().to_string(),
                execution_evidence_digest: facts.execution_evidence_digest().to_string(),
                terminal_journal_digest: facts.terminal_journal_digest().to_string(),
                authority_state_revision_before: facts.authority_state_revision_before(),
                authority_state_revision_after: facts.authority_state_revision_after(),
                inventory_revision_before: facts.inventory_revision_before(),
                inventory_revision_after: facts.inventory_revision_after(),
                inventory_digest_before: facts.inventory_digest_before().to_string(),
                inventory_digest_after: facts.inventory_digest_after().to_string(),
                authority_epoch_before: facts.authority_epoch_before(),
                authority_epoch_after: facts.authority_epoch_after(),
                process_owner_epoch: facts.process_owner_epoch(),
                trusted_time_high_water_ms_before: facts.trusted_time_high_water_ms_before(),
                completed_at_ms: facts.completed_at_ms(),
            },
            inventory_json_after: facts.inventory_json_after().to_string(),
            slot: CandidateCleanupSlotExpectation {
                plugin_id: slot.plugin_id.clone(),
                slot_ref: slot.slot_ref.clone(),
                release: slot.release.clone(),
            },
            candidate_generation: staging_receipt.candidate_generation,
            owner_plan_id: staging_receipt.owner_plan_id.clone(),
            owner_plan_digest: staging_receipt.owner_plan_digest.clone(),
            application_inventory_revision: staging_receipt.application_inventory_revision,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        &self.authority_instance_binding
    }
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn physical_completed_at(&self) -> Instant {
        self.physical_completed_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn completion_id(&self) -> &str {
        &self.completion_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn candidate_token(&self) -> &str {
        &self.candidate_token
    }
    pub(in crate::node_agent_compute_plugin_host) fn receipt_expectation(
        &self,
    ) -> &CandidateCleanupCompletionReceiptExpectation {
        &self.receipt
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_json_after(&self) -> &str {
        &self.inventory_json_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn slot_expectation(
        &self,
    ) -> &CandidateCleanupSlotExpectation {
        &self.slot
    }
    pub(in crate::node_agent_compute_plugin_host) fn candidate_generation(&self) -> i64 {
        self.candidate_generation
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner_plan_id(&self) -> &str {
        &self.owner_plan_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner_plan_digest(&self) -> &str {
        &self.owner_plan_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn application_inventory_revision(&self) -> i64 {
        self.application_inventory_revision
    }
}

impl fmt::Debug for CandidateCleanupCompletionRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupCompletionRecoveryKey")
            .field("completion_id", &"<redacted>")
            .field("candidate_token", &"<redacted>")
            .field(
                "candidate_token_digest",
                &self.receipt.candidate_token_digest,
            )
            .field(
                "authority_state_revision_after",
                &self.receipt.authority_state_revision_after,
            )
            .field(
                "inventory_revision_after",
                &self.receipt.inventory_revision_after,
            )
            .field("authority_epoch_after", &self.receipt.authority_epoch_after)
            .finish()
    }
}
