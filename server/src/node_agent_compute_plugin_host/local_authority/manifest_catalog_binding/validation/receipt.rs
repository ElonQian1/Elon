use anyhow::{bail, Result};

use super::super::types::{
    HashedComputePluginManifestCatalogBindingReceipt, PreparedManifestCatalogBindingRequest,
    COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA,
    HASHED_COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA,
};
use crate::node_agent_compute_plugin_host::{
    manifest_catalog::{
        validate_persisted_manifest_catalog_sources, COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA,
        MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_ENTRIES,
        MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_JSON_BYTES,
    },
    manifest_validation::is_sha256,
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    signed_artifact_verification::jcs_sha256_hex,
};

const I_JSON_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub(in crate::node_agent_compute_plugin_host::local_authority::manifest_catalog_binding) fn validate_hashed_receipt(
    request: &PreparedManifestCatalogBindingRequest,
    hashed: &HashedComputePluginManifestCatalogBindingReceipt,
) -> Result<()> {
    let receipt = &hashed.receipt;
    let catalog = validate_persisted_manifest_catalog_sources(
        &request.catalog_json,
        &request.signed_catalog_json,
        &request.signed_catalog_envelope_digest,
        &request.control_signing_key_id,
        &request.control_signing_key_fingerprint,
        &request.signed_manifests_json,
        &request.signed_manifest_set_digest,
    )?;
    if hashed.schema != HASHED_COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA
        || hashed.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || hashed.receipt_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || receipt.schema != COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA
        || jcs_sha256_hex(receipt)? != hashed.receipt_digest
        || catalog.schema != COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA
        || catalog.catalog_revision != receipt.catalog_revision
        || catalog.target_id != receipt.target_id
        || catalog.host_api_protocol_id != receipt.host_api_protocol_id
        || catalog.host_api_revision != receipt.host_api_revision
        || catalog.keyring_bundle_revision != receipt.keyring_bundle_revision
        || catalog.publisher_keyring != receipt.publisher_keyring
        || catalog.control_keyring != receipt.control_keyring
        || i64::try_from(catalog.entries.len()).ok() != Some(receipt.catalog_entry_count)
        || catalog.entries.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_ENTRIES
        || request.catalog_json.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_JSON_BYTES
        || jcs_sha256_hex(&catalog)? != receipt.catalog_digest
        || receipt.signed_catalog_envelope_digest != request.signed_catalog_envelope_digest
        || receipt.control_signing_key_id != request.control_signing_key_id
        || receipt.control_signing_key_fingerprint != request.control_signing_key_fingerprint
        || !is_sha256(&receipt.request_digest)
        || !is_sha256(&receipt.catalog_digest)
        || !is_sha256(&receipt.signed_catalog_envelope_digest)
        || !is_sha256(&receipt.control_signing_key_fingerprint)
        || !is_sha256(&receipt.signed_manifest_set_digest)
        || !is_sha256(&receipt.installation_id_digest)
        || !is_sha256(&receipt.node_profile_digest)
        || !is_sha256(&receipt.inventory_digest)
        || receipt.catalog_revision <= 0
        || receipt.catalog_revision > I_JSON_MAX_SAFE_INTEGER
        || receipt.catalog_revision < receipt.manifest_catalog_revision_before
        || receipt.state_revision_after != receipt.state_revision_before + 1
        || receipt.authority_epoch_after != receipt.authority_epoch_before + 1
        || receipt.bound_at_ms <= receipt.trusted_time_before_ms
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECEIPT_INVALID");
    }
    Ok(())
}
