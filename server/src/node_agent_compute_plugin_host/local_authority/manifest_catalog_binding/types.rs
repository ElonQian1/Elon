use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::{
    keyring::ComputePluginKeyringBinding, local_authority::ComputePluginAuthorityInstanceBinding,
};

pub(in crate::node_agent_compute_plugin_host) const COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.manifest_catalog_binding_receipt.v1";
pub(in crate::node_agent_compute_plugin_host) const HASHED_COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_manifest_catalog_binding_receipt.v1";
pub(super) const COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_REQUEST_SCHEMA: &str =
    "elon.compute_plugin.manifest_catalog_binding_request_digest.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginManifestCatalogBindingReceipt {
    pub schema: String,
    pub request_id: String,
    pub request_digest: String,
    pub installation_id_digest: String,
    pub manifest_catalog_revision_before: i64,
    pub catalog_revision: i64,
    pub catalog_digest: String,
    pub signed_catalog_envelope_digest: String,
    pub control_signing_key_id: String,
    pub control_signing_key_fingerprint: String,
    pub signed_manifest_set_digest: String,
    pub catalog_entry_count: i64,
    pub node_profile_digest: String,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: u32,
    pub keyring_bundle_revision: i64,
    pub publisher_keyring: ComputePluginKeyringBinding,
    pub control_keyring: ComputePluginKeyringBinding,
    pub state_revision_before: i64,
    pub state_revision_after: i64,
    pub inventory_revision: i64,
    pub inventory_digest: String,
    pub authority_epoch_before: i64,
    pub authority_epoch_after: i64,
    pub process_owner_epoch: i64,
    pub trusted_time_before_ms: i64,
    pub bound_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginManifestCatalogBindingReceipt
{
    pub schema: String,
    pub receipt: ComputePluginManifestCatalogBindingReceipt,
    pub canonicalization: String,
    pub receipt_digest_algorithm: String,
    pub receipt_digest: String,
}

#[derive(Debug, Clone)]
pub(super) struct PreparedManifestCatalogBindingRequest {
    pub request_id: String,
    pub request_digest: String,
    pub installation_id_digest: String,
    pub catalog_revision: i64,
    pub catalog_json: String,
    pub catalog_digest: String,
    pub signed_catalog_json: String,
    pub signed_catalog_envelope_digest: String,
    pub control_signing_key_id: String,
    pub control_signing_key_fingerprint: String,
    pub signed_manifests_json: String,
    pub signed_manifest_set_digest: String,
    pub catalog_entry_count: i64,
    pub node_profile_digest: String,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: u32,
    pub keyring_bundle_revision: i64,
    pub publisher_keyring: ComputePluginKeyringBinding,
    pub control_keyring: ComputePluginKeyringBinding,
}

#[derive(Serialize)]
pub(super) struct ManifestCatalogBindingRequestDigest<'a> {
    pub schema: &'static str,
    pub request_id: &'a str,
    pub installation_id_digest: &'a str,
    pub catalog_revision: i64,
    pub catalog_digest: &'a str,
    pub signed_catalog_envelope_digest: &'a str,
    pub control_signing_key_id: &'a str,
    pub control_signing_key_fingerprint: &'a str,
    pub signed_manifest_set_digest: &'a str,
    pub node_profile_digest: &'a str,
    pub target_id: &'a str,
    pub host_api_protocol_id: &'a str,
    pub host_api_revision: u32,
    pub keyring_bundle_revision: i64,
    pub publisher_keyring: &'a ComputePluginKeyringBinding,
    pub control_keyring: &'a ComputePluginKeyringBinding,
}

#[derive(Debug, Clone)]
pub(super) struct ManifestCatalogAuthorityState {
    pub installation_id_digest: String,
    pub state_revision: i64,
    pub inventory_revision: i64,
    pub inventory_digest: String,
    pub inventory_json: String,
    pub desired_policy_revision: i64,
    pub sharing_enabled: bool,
    pub node_profile_digest: String,
    pub manifest_catalog_revision: i64,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: u32,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub trusted_time_high_water_ms: i64,
    pub updated_at_ms: i64,
    pub keyring_bundle_revision: i64,
    pub publisher_keyring: ComputePluginKeyringBinding,
    pub control_keyring: ComputePluginKeyringBinding,
}

#[derive(Debug, Clone)]
pub(super) struct ProjectedManifestCatalogBinding {
    pub request: PreparedManifestCatalogBindingRequest,
    pub before: ManifestCatalogAuthorityState,
    pub hashed_receipt: HashedComputePluginManifestCatalogBindingReceipt,
}

#[derive(Debug)]
pub(super) struct ComputePluginManifestCatalogBindingRecoveryKey {
    pub authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    pub root_identity_digest: String,
    pub clock_epoch_digest: String,
    pub prepared_at: Instant,
    pub request: PreparedManifestCatalogBindingRequest,
    pub before: ManifestCatalogAuthorityState,
    pub hashed_receipt: HashedComputePluginManifestCatalogBindingReceipt,
}
