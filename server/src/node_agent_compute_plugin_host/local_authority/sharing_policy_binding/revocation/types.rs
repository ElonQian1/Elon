use serde::{Deserialize, Serialize};

pub(in crate::node_agent_compute_plugin_host) const COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.sharing_policy_capability_revocation_receipt.v1";
pub(in crate::node_agent_compute_plugin_host) const HASHED_COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_sharing_policy_capability_revocation_receipt.v1";
pub(super) const COMPUTE_PLUGIN_SHARING_POLICY_PREPARED_WORK_SET_SCHEMA: &str =
    "elon.compute_plugin.sharing_policy_prepared_work_set.v1";
pub(super) const FETCH_POLICY_TERMINAL_REASON: &str = "sharing_policy_transition_aborted";
pub(super) const VERIFICATION_POLICY_TERMINAL_REASON: &str = "verification_aborted";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginSharingPolicyCapabilityRevocationReceipt
{
    pub schema: String,
    pub policy_revision: i64,
    pub request_digest: String,
    pub policy_binding_receipt_digest: String,
    pub installation_id_digest: String,
    pub authority_epoch_before: i64,
    pub process_owner_epoch: i64,
    pub trusted_time_before_ms: i64,
    pub bound_at_ms: i64,
    pub work_item_count: i64,
    pub fetch_claim_count: i64,
    pub verification_count: i64,
    pub work_set_digest: String,
    pub fetch_resolution_reason: String,
    pub verification_resolution_reason: String,
    pub verification_result_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginSharingPolicyCapabilityRevocationReceipt
{
    pub schema: String,
    pub receipt: ComputePluginSharingPolicyCapabilityRevocationReceipt,
    pub canonicalization: String,
    pub receipt_digest_algorithm: String,
    pub receipt_digest: String,
}

impl HashedComputePluginSharingPolicyCapabilityRevocationReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginSharingPolicyCapabilityRevocationReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct PolicyPreparedWorkSet {
    pub schema: String,
    pub items: Vec<PolicyPreparedWorkItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum PolicyPreparedWorkItem {
    FetchClaim {
        claim_id: String,
        plan_id: String,
        plan_digest: String,
        ordinal: i64,
        candidate_token: String,
        authority_epoch: i64,
        process_owner_epoch: i64,
        cursor_generation: i64,
        redirect_generation: i64,
        offset_bytes: i64,
        length_bytes: i64,
        end_offset_bytes: i64,
        prepared_at_ms: i64,
    },
    CandidateVerification {
        verification_id: String,
        candidate_token: String,
        owner_plan_id: String,
        owner_plan_digest: String,
        verification_generation: i64,
        candidate_generation: i64,
        application_inventory_revision: i64,
        authority_state_revision: i64,
        authority_epoch: i64,
        process_owner_epoch: i64,
        artifact_count: i64,
        artifact_bytes: i64,
        expected_artifact_set_digest: String,
        file_set_binding_digest: String,
        prepared_at_ms: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct StoredPolicyCapabilityRevocation {
    pub(in super::super) hashed_receipt:
        HashedComputePluginSharingPolicyCapabilityRevocationReceipt,
    pub(super) work_set: PolicyPreparedWorkSet,
    pub(super) work_set_json: String,
    pub(super) verification_result_json: String,
}

pub(in super::super) type PreparedPolicyCapabilityRevocation = StoredPolicyCapabilityRevocation;
