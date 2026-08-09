use anyhow::{bail, Context, Result};
use rusqlite::{OptionalExtension, Transaction};

use super::super::{
    types::{
        ComputePluginManifestCatalogBindingReceipt,
        HashedComputePluginManifestCatalogBindingReceipt, ManifestCatalogBindingRequestDigest,
        PreparedManifestCatalogBindingRequest,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_REQUEST_SCHEMA,
        HASHED_COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA,
    },
    validation::validate_hashed_receipt,
};
use crate::node_agent_compute_plugin_host::{
    keyring::ComputePluginKeyringBinding,
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) struct StoredManifestCatalogBinding {
    pub request: PreparedManifestCatalogBindingRequest,
    pub hashed_receipt: HashedComputePluginManifestCatalogBindingReceipt,
}

pub(super) fn read_binding_by_revision(
    transaction: &Transaction<'_>,
    catalog_revision: i64,
) -> Result<Option<StoredManifestCatalogBinding>> {
    let row = transaction
        .query_row(
            r#"SELECT manifest_catalog_revision_before, request_digest, request_id,
                installation_id_digest, catalog_json, catalog_digest,
                signed_catalog_json, signed_catalog_envelope_digest,
                control_signing_key_id, control_signing_key_fingerprint,
                signed_manifests_json, signed_manifest_set_digest,
                catalog_entry_count, node_profile_digest,
                target_id, host_api_protocol_id, host_api_revision,
                keyring_bundle_revision, publisher_keyring_revision,
                publisher_keyring_digest, control_keyring_revision,
                control_keyring_digest, state_revision_before, state_revision_after,
                inventory_revision, inventory_digest, authority_epoch_before,
                authority_epoch_after, process_owner_epoch, trusted_time_before_ms,
                clock_status_before, authority_updated_at_ms_before, bound_at_ms,
                receipt_json, receipt_digest
            FROM manifest_catalog_binding_receipts WHERE catalog_revision = ?1"#,
            [catalog_revision],
            |row| {
                Ok(StoredManifestCatalogBindingRow {
                    manifest_catalog_revision_before: row.get(0)?,
                    request_digest: row.get(1)?,
                    request_id: row.get(2)?,
                    installation_id_digest: row.get(3)?,
                    catalog_json: row.get(4)?,
                    catalog_digest: row.get(5)?,
                    signed_catalog_json: row.get(6)?,
                    signed_catalog_envelope_digest: row.get(7)?,
                    control_signing_key_id: row.get(8)?,
                    control_signing_key_fingerprint: row.get(9)?,
                    signed_manifests_json: row.get(10)?,
                    signed_manifest_set_digest: row.get(11)?,
                    catalog_entry_count: row.get(12)?,
                    node_profile_digest: row.get(13)?,
                    target_id: row.get(14)?,
                    host_api_protocol_id: row.get(15)?,
                    host_api_revision: row.get(16)?,
                    keyring_bundle_revision: row.get(17)?,
                    publisher_keyring_revision: row.get(18)?,
                    publisher_keyring_digest: row.get(19)?,
                    control_keyring_revision: row.get(20)?,
                    control_keyring_digest: row.get(21)?,
                    state_revision_before: row.get(22)?,
                    state_revision_after: row.get(23)?,
                    inventory_revision: row.get(24)?,
                    inventory_digest: row.get(25)?,
                    authority_epoch_before: row.get(26)?,
                    authority_epoch_after: row.get(27)?,
                    process_owner_epoch: row.get(28)?,
                    trusted_time_before_ms: row.get(29)?,
                    clock_status_before: row.get(30)?,
                    authority_updated_at_ms_before: row.get(31)?,
                    bound_at_ms: row.get(32)?,
                    receipt_json: row.get(33)?,
                    receipt_digest: row.get(34)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECEIPT_READ")?;
    row.map(|row| row.validate(catalog_revision)).transpose()
}

struct StoredManifestCatalogBindingRow {
    manifest_catalog_revision_before: i64,
    request_digest: String,
    request_id: String,
    installation_id_digest: String,
    catalog_json: String,
    catalog_digest: String,
    signed_catalog_json: String,
    signed_catalog_envelope_digest: String,
    control_signing_key_id: String,
    control_signing_key_fingerprint: String,
    signed_manifests_json: String,
    signed_manifest_set_digest: String,
    catalog_entry_count: i64,
    node_profile_digest: String,
    target_id: String,
    host_api_protocol_id: String,
    host_api_revision: u32,
    keyring_bundle_revision: i64,
    publisher_keyring_revision: i64,
    publisher_keyring_digest: String,
    control_keyring_revision: i64,
    control_keyring_digest: String,
    state_revision_before: i64,
    state_revision_after: i64,
    inventory_revision: i64,
    inventory_digest: String,
    authority_epoch_before: i64,
    authority_epoch_after: i64,
    process_owner_epoch: i64,
    trusted_time_before_ms: i64,
    clock_status_before: String,
    authority_updated_at_ms_before: i64,
    bound_at_ms: i64,
    receipt_json: String,
    receipt_digest: String,
}

impl StoredManifestCatalogBindingRow {
    fn validate(self, catalog_revision: i64) -> Result<StoredManifestCatalogBinding> {
        let publisher_keyring = ComputePluginKeyringBinding {
            revision: self.publisher_keyring_revision,
            digest: self.publisher_keyring_digest,
        };
        let control_keyring = ComputePluginKeyringBinding {
            revision: self.control_keyring_revision,
            digest: self.control_keyring_digest,
        };
        let request = PreparedManifestCatalogBindingRequest {
            request_id: self.request_id,
            request_digest: self.request_digest,
            installation_id_digest: self.installation_id_digest,
            catalog_revision,
            catalog_json: self.catalog_json,
            catalog_digest: self.catalog_digest,
            signed_catalog_json: self.signed_catalog_json,
            signed_catalog_envelope_digest: self.signed_catalog_envelope_digest,
            control_signing_key_id: self.control_signing_key_id,
            control_signing_key_fingerprint: self.control_signing_key_fingerprint,
            signed_manifests_json: self.signed_manifests_json,
            signed_manifest_set_digest: self.signed_manifest_set_digest,
            catalog_entry_count: self.catalog_entry_count,
            node_profile_digest: self.node_profile_digest,
            target_id: self.target_id,
            host_api_protocol_id: self.host_api_protocol_id,
            host_api_revision: self.host_api_revision,
            keyring_bundle_revision: self.keyring_bundle_revision,
            publisher_keyring,
            control_keyring,
        };
        let receipt: ComputePluginManifestCatalogBindingReceipt =
            serde_json::from_str(&self.receipt_json)
                .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECEIPT_STORED_JSON")?;
        let hashed_receipt = HashedComputePluginManifestCatalogBindingReceipt {
            schema: HASHED_COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA.to_string(),
            receipt,
            canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
            receipt_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
            receipt_digest: self.receipt_digest,
        };
        validate_hashed_receipt(&request, &hashed_receipt)?;
        let receipt = &hashed_receipt.receipt;
        let calculated_request_digest = jcs_sha256_hex(&ManifestCatalogBindingRequestDigest {
            schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_REQUEST_SCHEMA,
            request_id: &request.request_id,
            installation_id_digest: &request.installation_id_digest,
            catalog_revision: request.catalog_revision,
            catalog_digest: &request.catalog_digest,
            signed_catalog_envelope_digest: &request.signed_catalog_envelope_digest,
            control_signing_key_id: &request.control_signing_key_id,
            control_signing_key_fingerprint: &request.control_signing_key_fingerprint,
            signed_manifest_set_digest: &request.signed_manifest_set_digest,
            node_profile_digest: &request.node_profile_digest,
            target_id: &request.target_id,
            host_api_protocol_id: &request.host_api_protocol_id,
            host_api_revision: request.host_api_revision,
            keyring_bundle_revision: request.keyring_bundle_revision,
            publisher_keyring: &request.publisher_keyring,
            control_keyring: &request.control_keyring,
        })?;
        if self.clock_status_before != "trusted"
            || self.authority_updated_at_ms_before != self.trusted_time_before_ms
            || request.request_digest != calculated_request_digest
            || receipt.request_id != request.request_id
            || receipt.request_digest != request.request_digest
            || receipt.installation_id_digest != request.installation_id_digest
            || receipt.manifest_catalog_revision_before != self.manifest_catalog_revision_before
            || receipt.catalog_revision != request.catalog_revision
            || receipt.catalog_digest != request.catalog_digest
            || receipt.signed_catalog_envelope_digest != request.signed_catalog_envelope_digest
            || receipt.control_signing_key_id != request.control_signing_key_id
            || receipt.control_signing_key_fingerprint != request.control_signing_key_fingerprint
            || receipt.signed_manifest_set_digest != request.signed_manifest_set_digest
            || receipt.catalog_entry_count != request.catalog_entry_count
            || receipt.node_profile_digest != request.node_profile_digest
            || receipt.target_id != request.target_id
            || receipt.host_api_protocol_id != request.host_api_protocol_id
            || receipt.host_api_revision != request.host_api_revision
            || receipt.keyring_bundle_revision != request.keyring_bundle_revision
            || receipt.publisher_keyring != request.publisher_keyring
            || receipt.control_keyring != request.control_keyring
            || receipt.state_revision_before != self.state_revision_before
            || receipt.state_revision_after != self.state_revision_after
            || receipt.inventory_revision != self.inventory_revision
            || receipt.inventory_digest != self.inventory_digest
            || receipt.authority_epoch_before != self.authority_epoch_before
            || receipt.authority_epoch_after != self.authority_epoch_after
            || receipt.process_owner_epoch != self.process_owner_epoch
            || receipt.trusted_time_before_ms != self.trusted_time_before_ms
            || receipt.bound_at_ms != self.bound_at_ms
        {
            bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECEIPT_COLUMNS_CHANGED");
        }
        Ok(StoredManifestCatalogBinding {
            request,
            hashed_receipt,
        })
    }
}

pub(super) fn validate_exact_request(
    stored: &PreparedManifestCatalogBindingRequest,
    expected: &PreparedManifestCatalogBindingRequest,
) -> Result<()> {
    if stored.request_id != expected.request_id
        || stored.request_digest != expected.request_digest
        || stored.installation_id_digest != expected.installation_id_digest
        || stored.catalog_revision != expected.catalog_revision
        || stored.catalog_json != expected.catalog_json
        || stored.catalog_digest != expected.catalog_digest
        || stored.signed_catalog_json != expected.signed_catalog_json
        || stored.signed_catalog_envelope_digest != expected.signed_catalog_envelope_digest
        || stored.control_signing_key_id != expected.control_signing_key_id
        || stored.control_signing_key_fingerprint != expected.control_signing_key_fingerprint
        || stored.signed_manifests_json != expected.signed_manifests_json
        || stored.signed_manifest_set_digest != expected.signed_manifest_set_digest
        || stored.catalog_entry_count != expected.catalog_entry_count
        || stored.node_profile_digest != expected.node_profile_digest
        || stored.target_id != expected.target_id
        || stored.host_api_protocol_id != expected.host_api_protocol_id
        || stored.host_api_revision != expected.host_api_revision
        || stored.keyring_bundle_revision != expected.keyring_bundle_revision
        || stored.publisher_keyring != expected.publisher_keyring
        || stored.control_keyring != expected.control_keyring
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_REQUEST_CONFLICT");
    }
    Ok(())
}

pub(super) fn validate_current_catalog_head(
    transaction: &Transaction<'_>,
    receipt: &HashedComputePluginManifestCatalogBindingReceipt,
    trusted_now: &chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    let current = super::super::validation::read_state_at_or_before(transaction, trusted_now)?;
    let receipt = &receipt.receipt;
    if current.installation_id_digest != receipt.installation_id_digest
        || current.manifest_catalog_revision != receipt.catalog_revision
        || current.node_profile_digest != receipt.node_profile_digest
        || current.target_id != receipt.target_id
        || current.host_api_protocol_id != receipt.host_api_protocol_id
        || current.host_api_revision != receipt.host_api_revision
        || current.keyring_bundle_revision != receipt.keyring_bundle_revision
        || current.publisher_keyring != receipt.publisher_keyring
        || current.control_keyring != receipt.control_keyring
        || current.state_revision < receipt.state_revision_after
        || current.authority_epoch < receipt.authority_epoch_after
        || current.process_owner_epoch < receipt.process_owner_epoch
        || current.trusted_time_high_water_ms < receipt.bound_at_ms
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_NOT_CURRENT");
    }
    Ok(())
}
