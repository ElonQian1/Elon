use std::fmt;

use super::capability::AuthorizedComputePluginCandidateStaging;
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, local_authority::ComputePluginAuthorityInstanceBinding,
};

/// Immutable receipt facts known before the staging Store is attempted.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateStagingReceiptExpectation
{
    pub(in crate::node_agent_compute_plugin_host) owner_plan_id: String,
    pub(in crate::node_agent_compute_plugin_host) owner_plan_digest: String,
    pub(in crate::node_agent_compute_plugin_host) verification_id: String,
    pub(in crate::node_agent_compute_plugin_host) verification_generation: i64,
    pub(in crate::node_agent_compute_plugin_host) candidate_generation: i64,
    pub(in crate::node_agent_compute_plugin_host) application_inventory_revision: i64,
    pub(in crate::node_agent_compute_plugin_host) verification_result_digest: String,
    pub(in crate::node_agent_compute_plugin_host) verification_resolved_at_ms: i64,
    pub(in crate::node_agent_compute_plugin_host) root_identity_digest: String,
    pub(in crate::node_agent_compute_plugin_host) staging_run_digest: String,
    pub(in crate::node_agent_compute_plugin_host) extraction_plan_digest: String,
    pub(in crate::node_agent_compute_plugin_host) extraction_evidence_digest: String,
    pub(in crate::node_agent_compute_plugin_host) staging_seal_payload_digest: String,
    pub(in crate::node_agent_compute_plugin_host) staging_seal_file_digest: String,
    pub(in crate::node_agent_compute_plugin_host) staging_seal_identity_digest: String,
    pub(in crate::node_agent_compute_plugin_host) staging_seal_size_bytes: i64,
    pub(in crate::node_agent_compute_plugin_host) extracted_file_count: i64,
    pub(in crate::node_agent_compute_plugin_host) extracted_bytes: i64,
    pub(in crate::node_agent_compute_plugin_host) authority_state_revision_before: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_revision_before: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_digest_before: String,
    pub(in crate::node_agent_compute_plugin_host) authority_epoch_before: i64,
    pub(in crate::node_agent_compute_plugin_host) process_owner_epoch: i64,
    pub(in crate::node_agent_compute_plugin_host) trusted_time_high_water_ms_before: i64,
}

/// Candidate slot identity required to prove that an absent receipt was never created.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateStagingSlotExpectation {
    pub(in crate::node_agent_compute_plugin_host) plugin_id: String,
    pub(in crate::node_agent_compute_plugin_host) slot_ref: String,
    pub(in crate::node_agent_compute_plugin_host) release: ComputePluginReleaseRef,
}

/// Process-local identity for classifying an uncertain staging commit. It is not a retry permit.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateStagingRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    staging_id: String,
    candidate_token: String,
    candidate_token_digest: String,
    receipt: ComputePluginCandidateStagingReceiptExpectation,
    slot: ComputePluginCandidateStagingSlotExpectation,
}

impl fmt::Debug for ComputePluginCandidateStagingRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCandidateStagingRecoveryKey")
            .field("staging_id", &"<redacted>")
            .field("candidate_token", &"<redacted>")
            .field("candidate_token_digest", &self.candidate_token_digest)
            .field("verification_id", &"<redacted>")
            .field("staging_run_digest", &"<redacted>")
            .field("process_owner_epoch", &self.receipt.process_owner_epoch)
            .field(
                "authority_state_revision_before",
                &self.receipt.authority_state_revision_before,
            )
            .field(
                "inventory_revision_before",
                &self.receipt.inventory_revision_before,
            )
            .field(
                "authority_epoch_before",
                &self.receipt.authority_epoch_before,
            )
            .finish()
    }
}

impl ComputePluginCandidateStagingRecoveryKey {
    pub(super) fn from_authorized(
        authorized: &AuthorizedComputePluginCandidateStaging<'_, '_>,
        staging_id: String,
    ) -> Self {
        let archive = authorized.revalidated.archive();
        let key = archive.verification_recovery_key();
        let binding = &authorized.binding;
        let evidence = archive.evidence();
        let seal = archive.seal_evidence();
        Self {
            authority_instance_binding: key.authority_instance_binding().clone(),
            installation_id_digest: key.installation_id_digest().to_string(),
            clock_epoch_digest: key.clock_epoch_digest().to_string(),
            staging_id,
            candidate_token: key.candidate_token().to_string(),
            candidate_token_digest: key.candidate_token_digest().to_string(),
            receipt: ComputePluginCandidateStagingReceiptExpectation {
                owner_plan_id: key.owner_plan_id().to_string(),
                owner_plan_digest: key.owner_plan_digest().to_string(),
                verification_id: key.verification_id().to_string(),
                verification_generation: key.verification_generation(),
                candidate_generation: key.candidate_generation(),
                application_inventory_revision: key.application_inventory_revision(),
                verification_result_digest: binding.verification_result_digest().to_string(),
                verification_resolved_at_ms: binding.verification_resolved_at_ms(),
                root_identity_digest: key.root_identity_digest().to_string(),
                staging_run_digest: evidence.evidence.staging_run_digest.clone(),
                extraction_plan_digest: archive.plan().envelope().plan_digest.clone(),
                extraction_evidence_digest: evidence.evidence_digest.clone(),
                staging_seal_payload_digest: seal.payload_digest.clone(),
                staging_seal_file_digest: seal.file_digest.clone(),
                staging_seal_identity_digest: seal.file_identity_digest.clone(),
                staging_seal_size_bytes: seal.size_bytes,
                extracted_file_count: evidence.evidence.extracted_file_count,
                extracted_bytes: evidence.evidence.extracted_bytes,
                authority_state_revision_before: binding.authority_state_revision(),
                inventory_revision_before: binding.inventory_revision(),
                inventory_digest_before: binding.inventory_digest().to_string(),
                authority_epoch_before: binding.authority_epoch(),
                process_owner_epoch: binding.process_owner_epoch(),
                trusted_time_high_water_ms_before: binding.trusted_time_high_water_ms(),
            },
            slot: ComputePluginCandidateStagingSlotExpectation {
                plugin_id: binding.candidate_plugin_id().to_string(),
                slot_ref: binding.candidate_slot_ref().to_string(),
                release: binding.candidate_release().clone(),
            },
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_id(&self) -> &str {
        &self.staging_id
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

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token(&self) -> &str {
        &self.candidate_token
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn verification_id(&self) -> &str {
        &self.receipt.verification_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_run_digest(&self) -> &str {
        &self.receipt.staging_run_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        &self.receipt.root_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.receipt.process_owner_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_expectation(
        &self,
    ) -> &ComputePluginCandidateStagingReceiptExpectation {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn slot_expectation(
        &self,
    ) -> &ComputePluginCandidateStagingSlotExpectation {
        &self.slot
    }
}
