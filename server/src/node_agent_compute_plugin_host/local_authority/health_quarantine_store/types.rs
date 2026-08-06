use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::candidate_health_contract::HashedComputePluginCandidateHealthFailureObservation;

pub(super) const CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.candidate_health_quarantine_receipt.v1";
pub(super) const HASHED_CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_candidate_health_quarantine_receipt.v1";
pub(super) const CANDIDATE_HEALTH_QUARANTINE_RECEIPT_CANONICALIZATION: &str = "RFC8785-JCS";
pub(super) const CANDIDATE_HEALTH_QUARANTINE_RECEIPT_DIGEST_ALGORITHM: &str = "sha256";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateHealthQuarantineReceipt {
    pub(super) schema: String,
    pub(super) quarantine_id: String,
    pub(super) evaluation_id: String,
    pub(super) candidate_token_digest: String,
    pub(super) staging_id: String,
    pub(super) staging_receipt_digest: String,
    pub(super) staging_run_digest: String,
    pub(super) failure_observation_digest: String,
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
    pub(super) failed_at_ms: i64,
    pub(super) slot_phase_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateHealthQuarantineReceipt
{
    pub(super) schema: String,
    pub(super) observation: HashedComputePluginCandidateHealthFailureObservation,
    pub(super) receipt: ComputePluginCandidateHealthQuarantineReceipt,
    pub(super) canonicalization: String,
    pub(super) digest_algorithm: String,
    pub(super) receipt_digest: String,
}

impl ComputePluginCandidateHealthQuarantineReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn quarantine_id(&self) -> &str {
        &self.quarantine_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn evaluation_id(&self) -> &str {
        &self.evaluation_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn staging_id(&self) -> &str {
        &self.staging_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn staging_receipt_digest(&self) -> &str {
        &self.staging_receipt_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn staging_run_digest(&self) -> &str {
        &self.staging_run_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn failure_observation_digest(&self) -> &str {
        &self.failure_observation_digest
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
    pub(in crate::node_agent_compute_plugin_host) fn failed_at_ms(&self) -> i64 {
        self.failed_at_ms
    }
    pub(in crate::node_agent_compute_plugin_host) fn slot_phase_after(&self) -> &str {
        &self.slot_phase_after
    }
}

impl HashedComputePluginCandidateHealthQuarantineReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn observation(
        &self,
    ) -> &HashedComputePluginCandidateHealthFailureObservation {
        &self.observation
    }
    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginCandidateHealthQuarantineReceipt {
        &self.receipt
    }
    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}
