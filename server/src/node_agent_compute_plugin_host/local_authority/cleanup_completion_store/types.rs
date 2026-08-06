use serde::{Deserialize, Serialize};

pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA:
    &str = "elon.compute_plugin.candidate_cleanup_completion_receipt.v1";
pub(in crate::node_agent_compute_plugin_host) const HASHED_CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_candidate_cleanup_completion_receipt.v1";
pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_COMPLETION_RECEIPT_CANONICALIZATION: &str =
    "RFC8785-JCS";
pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_CLEANUP_COMPLETION_RECEIPT_DIGEST_ALGORITHM: &str =
    "sha256";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupCompletionReceipt
{
    pub(super) schema: String,
    pub(super) completion_id: String,
    pub(super) cleanup_id: String,
    pub(super) candidate_token_digest: String,
    pub(super) authorization_receipt_digest: String,
    pub(super) execution_plan_digest: String,
    pub(super) execution_evidence_digest: String,
    pub(super) terminal_journal_digest: String,
    pub(super) authority_state_revision_before: i64,
    pub(super) authority_state_revision_after: i64,
    pub(super) inventory_revision_before: i64,
    pub(super) inventory_revision_after: i64,
    pub(super) inventory_digest_before: String,
    pub(super) inventory_digest_after: String,
    pub(super) authority_epoch_before: i64,
    pub(super) authority_epoch_after: i64,
    pub(super) process_owner_epoch: i64,
    pub(super) trusted_time_high_water_ms_before: i64,
    pub(super) completed_at_ms: i64,
    pub(super) slot_phase_before: String,
    pub(super) slot_phase_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateCleanupCompletionReceipt
{
    pub(super) schema: String,
    pub(super) receipt: ComputePluginCandidateCleanupCompletionReceipt,
    pub(super) canonicalization: String,
    pub(super) digest_algorithm: String,
    pub(super) receipt_digest: String,
}

impl ComputePluginCandidateCleanupCompletionReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn schema(&self) -> &str {
        &self.schema
    }
    pub(in crate::node_agent_compute_plugin_host) fn completion_id(&self) -> &str {
        &self.completion_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn cleanup_id(&self) -> &str {
        &self.cleanup_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
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
    pub(in crate::node_agent_compute_plugin_host) fn slot_phase_before(&self) -> &str {
        &self.slot_phase_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn slot_phase_after(&self) -> &str {
        &self.slot_phase_after
    }
}

impl HashedComputePluginCandidateCleanupCompletionReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn schema(&self) -> &str {
        &self.schema
    }
    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginCandidateCleanupCompletionReceipt {
        &self.receipt
    }
    pub(in crate::node_agent_compute_plugin_host) fn canonicalization(&self) -> &str {
        &self.canonicalization
    }
    pub(in crate::node_agent_compute_plugin_host) fn digest_algorithm(&self) -> &str {
        &self.digest_algorithm
    }
    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

    fn receipt() -> ComputePluginCandidateCleanupCompletionReceipt {
        ComputePluginCandidateCleanupCompletionReceipt {
            schema: CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA.to_string(),
            completion_id: "ccc_test".to_string(),
            cleanup_id: "cca_test".to_string(),
            candidate_token_digest: "a".repeat(64),
            authorization_receipt_digest: "b".repeat(64),
            execution_plan_digest: "f".repeat(64),
            execution_evidence_digest: "c".repeat(64),
            terminal_journal_digest: "1".repeat(64),
            authority_state_revision_before: 11,
            authority_state_revision_after: 12,
            inventory_revision_before: 7,
            inventory_revision_after: 8,
            inventory_digest_before: "d".repeat(64),
            inventory_digest_after: "e".repeat(64),
            authority_epoch_before: 21,
            authority_epoch_after: 22,
            process_owner_epoch: 3,
            trusted_time_high_water_ms_before: 1_001,
            completed_at_ms: 1_002,
            slot_phase_before: "failed".to_string(),
            slot_phase_after: "removed".to_string(),
        }
    }

    #[test]
    fn completion_receipt_round_trips_with_stable_digest() {
        let receipt = receipt();
        let digest = jcs_sha256_hex(&receipt).unwrap();
        let encoded = serde_json::to_string(&receipt).unwrap();
        let decoded: ComputePluginCandidateCleanupCompletionReceipt =
            serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, receipt);
        assert_eq!(jcs_sha256_hex(&decoded).unwrap(), digest);
    }

    #[test]
    fn completion_receipt_rejects_unknown_fields() {
        let mut value = serde_json::to_value(receipt()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("untrusted_extension".to_string(), serde_json::json!(true));

        assert!(
            serde_json::from_value::<ComputePluginCandidateCleanupCompletionReceipt>(value)
                .is_err()
        );
    }
}
