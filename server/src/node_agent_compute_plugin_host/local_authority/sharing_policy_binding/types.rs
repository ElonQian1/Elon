use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::super::ComputePluginAuthorityInstanceBinding;
use super::revocation::PreparedPolicyCapabilityRevocation;

pub(in crate::node_agent_compute_plugin_host) const COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.sharing_policy_binding_receipt.v1";
pub(in crate::node_agent_compute_plugin_host) const HASHED_COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_sharing_policy_binding_receipt.v1";
pub(super) const COMPUTE_PLUGIN_SHARING_POLICY_BINDING_REQUEST_SCHEMA: &str =
    "elon.compute_plugin.sharing_policy_binding_request.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginSharingPolicyBindingReceipt {
    pub schema: String,
    pub request_digest: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_id_digest: String,
    pub policy_revision: i64,
    pub policy_digest: String,
    pub policy_snapshot_digest: String,
    pub sharing_enabled: bool,
    pub sharing_authorization_ref: Option<String>,
    pub sharing_authorization_revision: Option<i64>,
    pub sharing_authorization_digest: Option<String>,
    pub source_preparation_id: Option<String>,
    pub source_bootstrap_instance_id: String,
    pub source_configuration_generation: u64,
    pub source_cancellation_generation: u64,
    pub state_revision_before: i64,
    pub state_revision_after: i64,
    pub inventory_revision_before: i64,
    pub inventory_revision_after: i64,
    pub inventory_digest_before: String,
    pub inventory_digest_after: String,
    pub authority_epoch_before: i64,
    pub authority_epoch_after: i64,
    pub process_owner_epoch: i64,
    pub trusted_time_before_ms: i64,
    pub bound_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginSharingPolicyBindingReceipt
{
    pub schema: String,
    pub receipt: ComputePluginSharingPolicyBindingReceipt,
    pub canonicalization: String,
    pub receipt_digest_algorithm: String,
    pub receipt_digest: String,
}

impl HashedComputePluginSharingPolicyBindingReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginSharingPolicyBindingReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SharingPolicyBindingRequestDigest<'a> {
    pub schema: &'static str,
    pub policy_snapshot: &'a homecli_proto::ComputePluginSharingPolicySnapshotV1,
    pub policy_snapshot_digest: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreparedSharingPolicyBindingRequest {
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_id_digest: String,
    pub policy_revision: i64,
    pub policy_digest: String,
    pub policy_snapshot_json: String,
    pub policy_snapshot_digest: String,
    pub sharing_enabled: bool,
    pub sharing_authorization_ref: Option<String>,
    pub sharing_authorization_revision: Option<i64>,
    pub sharing_authorization_digest: Option<String>,
    pub source_preparation_id: Option<String>,
    pub source_bootstrap_instance_id: String,
    pub source_configuration_generation: u64,
    pub source_cancellation_generation: u64,
    pub request_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PolicyBindingAuthorityState {
    pub installation_id_digest: String,
    pub state_revision: i64,
    pub inventory_revision: i64,
    pub inventory_digest: String,
    pub inventory_json: String,
    pub desired_policy_revision: i64,
    pub sharing_enabled: bool,
    pub sharing_authorization_ref: Option<String>,
    pub sharing_authorization_revision: Option<i64>,
    pub sharing_authorization_digest: Option<String>,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub trusted_time_high_water_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone)]
pub(super) struct ProjectedSharingPolicyBinding {
    pub request: PreparedSharingPolicyBindingRequest,
    pub before: PolicyBindingAuthorityState,
    pub inventory_after_json: String,
    pub hashed_receipt: HashedComputePluginSharingPolicyBindingReceipt,
}

pub(super) struct ComputePluginSharingPolicyBindingRecoveryKey {
    pub authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    pub root_identity_digest: String,
    pub clock_epoch_digest: String,
    pub prepared_at: Instant,
    pub request: PreparedSharingPolicyBindingRequest,
    pub before: PolicyBindingAuthorityState,
    pub inventory_after_json: String,
    pub hashed_receipt: HashedComputePluginSharingPolicyBindingReceipt,
    pub prepared_revocation: PreparedPolicyCapabilityRevocation,
}
