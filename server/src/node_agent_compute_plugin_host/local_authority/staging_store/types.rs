use serde::{Deserialize, Serialize};

pub(super) const CANDIDATE_STAGING_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.candidate_staging_receipt.v1";
pub(super) const HASHED_CANDIDATE_STAGING_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_candidate_staging_receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateStagingReceipt {
    pub(super) schema: String,
    pub(super) staging_id: String,
    pub(super) candidate_token_digest: String,
    pub(super) owner_plan_id: String,
    pub(super) owner_plan_digest: String,
    pub(super) verification_id: String,
    pub(super) verification_generation: i64,
    pub(super) candidate_generation: i64,
    pub(super) application_inventory_revision: i64,
    pub(super) verification_result_digest: String,
    pub(super) root_identity_digest: String,
    pub(super) staging_run_digest: String,
    pub(super) extraction_plan_digest: String,
    pub(super) extraction_evidence_digest: String,
    pub(super) staging_seal_payload_digest: String,
    pub(super) staging_seal_file_digest: String,
    pub(super) staging_seal_identity_digest: String,
    pub(super) staging_seal_size_bytes: i64,
    pub(super) extracted_file_count: i64,
    pub(super) extracted_bytes: i64,
    pub(super) authority_state_revision_before: i64,
    pub(super) authority_state_revision_after: i64,
    pub(super) inventory_revision_before: i64,
    pub(super) inventory_revision_after: i64,
    pub(super) inventory_digest_before: String,
    pub(super) inventory_digest_after: String,
    pub(super) authority_epoch_before: i64,
    pub(super) authority_epoch_after: i64,
    pub(super) process_owner_epoch: i64,
    pub(super) staged_at_ms: i64,
    pub(super) slot_phase_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateStagingReceipt {
    pub(super) schema: String,
    pub(super) receipt: ComputePluginCandidateStagingReceipt,
    pub(super) canonicalization: String,
    pub(super) digest_algorithm: String,
    pub(super) receipt_digest: String,
}

impl HashedComputePluginCandidateStagingReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginCandidateStagingReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}

impl ComputePluginCandidateStagingReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn staging_id(&self) -> &str {
        &self.staging_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn verification_id(&self) -> &str {
        &self.verification_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_run_digest(&self) -> &str {
        &self.staging_run_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision_after(&self) -> i64 {
        self.authority_state_revision_after
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision_after(&self) -> i64 {
        self.inventory_revision_after
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest_after(&self) -> &str {
        &self.inventory_digest_after
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch_after(&self) -> i64 {
        self.authority_epoch_after
    }

    pub(in crate::node_agent_compute_plugin_host) fn staged_at_ms(&self) -> i64 {
        self.staged_at_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn slot_phase_after(&self) -> &str {
        &self.slot_phase_after
    }
}
