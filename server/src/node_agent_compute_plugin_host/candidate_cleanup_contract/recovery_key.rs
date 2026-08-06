use std::fmt;

use super::PreparedCandidateCleanupAuthorization;
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, local_authority::ComputePluginAuthorityInstanceBinding,
};

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupAuthorizationReceiptExpectation
{
    pub(in crate::node_agent_compute_plugin_host) candidate_token_digest: String,
    pub(in crate::node_agent_compute_plugin_host) quarantine_id: String,
    pub(in crate::node_agent_compute_plugin_host) quarantine_receipt_digest: String,
    pub(in crate::node_agent_compute_plugin_host) staging_id: String,
    pub(in crate::node_agent_compute_plugin_host) staging_run_digest: String,
    pub(in crate::node_agent_compute_plugin_host) authority_state_revision_before: i64,
    pub(in crate::node_agent_compute_plugin_host) authority_state_revision_after: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_revision: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_digest: String,
    pub(in crate::node_agent_compute_plugin_host) authority_epoch_before: i64,
    pub(in crate::node_agent_compute_plugin_host) authority_epoch_after: i64,
    pub(in crate::node_agent_compute_plugin_host) process_owner_epoch: i64,
    pub(in crate::node_agent_compute_plugin_host) trusted_time_high_water_ms_before: i64,
    pub(in crate::node_agent_compute_plugin_host) authorized_at_ms: i64,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupSlotExpectation {
    pub(in crate::node_agent_compute_plugin_host) plugin_id: String,
    pub(in crate::node_agent_compute_plugin_host) slot_ref: String,
    pub(in crate::node_agent_compute_plugin_host) release: ComputePluginReleaseRef,
}

/// Process-local identity for classifying one uncertain authorization commit. It cannot authorize
/// deletion, retry, installation, owner release or a completion receipt.
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupAuthorizationRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    cleanup_id: String,
    candidate_token: String,
    receipt: CandidateCleanupAuthorizationReceiptExpectation,
    slot: CandidateCleanupSlotExpectation,
    candidate_generation: i64,
    owner_plan_id: String,
    owner_plan_digest: String,
    application_inventory_revision: i64,
}

impl CandidateCleanupAuthorizationRecoveryKey {
    pub(super) fn from_prepared(prepared: &PreparedCandidateCleanupAuthorization<'_, '_>) -> Self {
        let staging = prepared.quarantined.staged().recovery_key();
        let slot = staging.slot_expectation();
        let staging_receipt = staging.receipt_expectation();
        let facts = &prepared.facts;
        Self {
            authority_instance_binding: prepared
                .authority_session
                .authority_instance_binding()
                .clone(),
            installation_id_digest: prepared
                .authority_session
                .installation_id_digest()
                .to_string(),
            clock_epoch_digest: prepared.authority_session.clock_epoch_digest().to_string(),
            cleanup_id: prepared.cleanup_id.clone(),
            candidate_token: staging.candidate_token().to_string(),
            receipt: CandidateCleanupAuthorizationReceiptExpectation {
                candidate_token_digest: facts.candidate_token_digest().to_string(),
                quarantine_id: facts.quarantine_id().to_string(),
                quarantine_receipt_digest: facts.quarantine_receipt_digest().to_string(),
                staging_id: facts.staging_id().to_string(),
                staging_run_digest: facts.staging_run_digest().to_string(),
                authority_state_revision_before: facts.authority_state_revision_before(),
                authority_state_revision_after: facts.authority_state_revision_after(),
                inventory_revision: facts.inventory_revision(),
                inventory_digest: facts.inventory_digest().to_string(),
                authority_epoch_before: facts.authority_epoch_before(),
                authority_epoch_after: facts.authority_epoch_after(),
                process_owner_epoch: facts.process_owner_epoch(),
                trusted_time_high_water_ms_before: facts.trusted_time_high_water_ms_before(),
                authorized_at_ms: facts.authorized_at_ms(),
            },
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
    pub(in crate::node_agent_compute_plugin_host) fn cleanup_id(&self) -> &str {
        &self.cleanup_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn candidate_token(&self) -> &str {
        &self.candidate_token
    }
    pub(in crate::node_agent_compute_plugin_host) fn receipt_expectation(
        &self,
    ) -> &CandidateCleanupAuthorizationReceiptExpectation {
        &self.receipt
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

impl fmt::Debug for CandidateCleanupAuthorizationRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupAuthorizationRecoveryKey")
            .field("cleanup_id", &"<redacted>")
            .field("candidate_token", &"<redacted>")
            .field(
                "candidate_token_digest",
                &self.receipt.candidate_token_digest,
            )
            .field(
                "authority_state_revision_after",
                &self.receipt.authority_state_revision_after,
            )
            .field("inventory_revision", &self.receipt.inventory_revision)
            .field("authority_epoch_after", &self.receipt.authority_epoch_after)
            .finish()
    }
}
