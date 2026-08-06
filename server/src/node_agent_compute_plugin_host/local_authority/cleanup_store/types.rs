use serde::{Deserialize, Serialize};

pub(super) const CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.candidate_cleanup_authorization_receipt.v1";
pub(super) const HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_candidate_cleanup_authorization_receipt.v1";
pub(super) const CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION: &str = "RFC8785-JCS";
pub(super) const CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM: &str = "sha256";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupAuthorizationReceipt
{
    pub(super) schema: String,
    pub(super) cleanup_id: String,
    pub(super) candidate_token_digest: String,
    pub(super) quarantine_id: String,
    pub(super) quarantine_receipt_digest: String,
    pub(super) staging_id: String,
    pub(super) staging_run_digest: String,
    pub(super) authority_state_revision_before: i64,
    pub(super) authority_state_revision_after: i64,
    pub(super) inventory_revision: i64,
    pub(super) inventory_digest: String,
    pub(super) authority_epoch_before: i64,
    pub(super) authority_epoch_after: i64,
    pub(super) process_owner_epoch: i64,
    pub(super) trusted_time_high_water_ms_before: i64,
    pub(super) authorized_at_ms: i64,
    pub(super) slot_phase_before: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateCleanupAuthorizationReceipt
{
    pub(super) schema: String,
    pub(super) receipt: ComputePluginCandidateCleanupAuthorizationReceipt,
    pub(super) canonicalization: String,
    pub(super) digest_algorithm: String,
    pub(super) receipt_digest: String,
}

impl ComputePluginCandidateCleanupAuthorizationReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn cleanup_id(&self) -> &str {
        &self.cleanup_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn quarantine_id(&self) -> &str {
        &self.quarantine_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn quarantine_receipt_digest(&self) -> &str {
        &self.quarantine_receipt_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn staging_id(&self) -> &str {
        &self.staging_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn staging_run_digest(&self) -> &str {
        &self.staging_run_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision_before(&self) -> i64 {
        self.authority_state_revision_before
    }
    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision_after(&self) -> i64 {
        self.authority_state_revision_after
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision(&self) -> i64 {
        self.inventory_revision
    }
    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest(&self) -> &str {
        &self.inventory_digest
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
    pub(in crate::node_agent_compute_plugin_host) fn authorized_at_ms(&self) -> i64 {
        self.authorized_at_ms
    }
    pub(in crate::node_agent_compute_plugin_host) fn slot_phase_before(&self) -> &str {
        &self.slot_phase_before
    }
}

impl HashedComputePluginCandidateCleanupAuthorizationReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginCandidateCleanupAuthorizationReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

    fn receipt() -> ComputePluginCandidateCleanupAuthorizationReceipt {
        ComputePluginCandidateCleanupAuthorizationReceipt {
            schema: CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA.to_string(),
            cleanup_id: "cca_test".to_string(),
            candidate_token_digest: "a".repeat(64),
            quarantine_id: "chq_test".to_string(),
            quarantine_receipt_digest: "b".repeat(64),
            staging_id: "cst_test".to_string(),
            staging_run_digest: "c".repeat(64),
            authority_state_revision_before: 10,
            authority_state_revision_after: 11,
            inventory_revision: 7,
            inventory_digest: "d".repeat(64),
            authority_epoch_before: 20,
            authority_epoch_after: 21,
            process_owner_epoch: 3,
            trusted_time_high_water_ms_before: 1_000,
            authorized_at_ms: 1_001,
            slot_phase_before: "failed".to_string(),
        }
    }

    #[test]
    fn cleanup_authorization_receipt_round_trips_with_stable_digest() {
        let receipt = receipt();
        let digest = jcs_sha256_hex(&receipt).unwrap();
        let encoded = serde_json::to_string(&receipt).unwrap();
        let decoded: ComputePluginCandidateCleanupAuthorizationReceipt =
            serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, receipt);
        assert_eq!(jcs_sha256_hex(&decoded).unwrap(), digest);
    }

    #[test]
    fn cleanup_authorization_receipt_rejects_unknown_fields() {
        let mut value = serde_json::to_value(receipt()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("untrusted_extension".to_string(), serde_json::json!(true));

        assert!(
            serde_json::from_value::<ComputePluginCandidateCleanupAuthorizationReceipt>(value)
                .is_err()
        );
    }
}
