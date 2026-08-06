use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::candidate_health_contract::HashedComputePluginCandidateHealthObservation;

pub(super) const CANDIDATE_HEALTH_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.candidate_health_receipt.v1";
pub(super) const HASHED_CANDIDATE_HEALTH_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_candidate_health_receipt.v1";
pub(super) const CANDIDATE_HEALTH_RECEIPT_CANONICALIZATION: &str = "RFC8785-JCS";
pub(super) const CANDIDATE_HEALTH_RECEIPT_DIGEST_ALGORITHM: &str = "sha256";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateHealthReceipt {
    pub(super) schema: String,
    pub(super) health_id: String,
    pub(super) evaluation_id: String,
    pub(super) candidate_token_digest: String,
    pub(super) staging_id: String,
    pub(super) staging_receipt_digest: String,
    pub(super) staging_run_digest: String,
    pub(super) health_observation_digest: String,
    pub(super) authority_state_revision: i64,
    pub(super) inventory_revision: i64,
    pub(super) inventory_digest: String,
    pub(super) authority_epoch: i64,
    pub(super) process_owner_epoch: i64,
    pub(super) recorded_at_ms: i64,
    pub(super) expires_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateHealthReceipt {
    pub(super) schema: String,
    pub(super) observation: HashedComputePluginCandidateHealthObservation,
    pub(super) receipt: ComputePluginCandidateHealthReceipt,
    pub(super) canonicalization: String,
    pub(super) digest_algorithm: String,
    pub(super) receipt_digest: String,
}

impl ComputePluginCandidateHealthReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn health_id(&self) -> &str {
        &self.health_id
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

    pub(in crate::node_agent_compute_plugin_host) fn health_observation_digest(&self) -> &str {
        &self.health_observation_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision(&self) -> i64 {
        self.authority_state_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision(&self) -> i64 {
        self.inventory_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest(&self) -> &str {
        &self.inventory_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch(&self) -> i64 {
        self.authority_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_owner_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn recorded_at_ms(&self) -> i64 {
        self.recorded_at_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn expires_at_ms(&self) -> i64 {
        self.expires_at_ms
    }
}

impl HashedComputePluginCandidateHealthReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn observation(
        &self,
    ) -> &HashedComputePluginCandidateHealthObservation {
        &self.observation
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginCandidateHealthReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}
