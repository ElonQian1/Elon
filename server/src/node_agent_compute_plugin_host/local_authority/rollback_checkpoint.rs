use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::ComputePluginLocalAuthority;
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginInstallationIdentity,
    install_plan_admission_validation::is_identifier,
    lifecycle::{
        local_record_shape_is_valid, ComputePluginInventorySnapshot,
        COMPUTE_PLUGIN_INVENTORY_SCHEMA, MAX_COMPUTE_PLUGIN_INVENTORY_RECORDS,
    },
    local_authority_schema::COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION,
    manifest_validation::is_sha256,
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    signed_artifact_verification::jcs_sha256_hex,
};

pub(crate) const COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA: &str =
    "elon.compute_plugin.authority_rollback_checkpoint.v1";
pub(crate) const HASHED_COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA: &str =
    "elon.compute_plugin.hashed_authority_rollback_checkpoint.v1";
const SHARING_AUTHORIZATION_REF_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_SHARING_AUTHORIZATION_REF_V1";

/// Canonical monotonic facts exported for a future external rollback witness. Inventory content is
/// represented by its revalidated JCS digest; raw inventory and authorization references stay local.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginAuthorityRollbackCheckpoint {
    pub schema: String,
    pub authority_schema_version: i64,
    pub installation_id_digest: String,
    pub state_revision: i64,
    pub inventory_revision: i64,
    pub inventory_digest: String,
    pub desired_policy_revision: i64,
    pub sharing_enabled: bool,
    pub sharing_authorization_ref_digest: Option<String>,
    pub sharing_authorization_revision: Option<i64>,
    pub sharing_authorization_digest: Option<String>,
    pub node_profile_digest: String,
    pub manifest_catalog_revision: i64,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: i64,
    pub active_bundle_revision: Option<i64>,
    pub publisher_keyring_revision: Option<i64>,
    pub publisher_keyring_digest: Option<String>,
    pub control_keyring_revision: Option<i64>,
    pub control_keyring_digest: Option<String>,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub trusted_time_high_water_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct HashedComputePluginAuthorityRollbackCheckpoint {
    pub schema: String,
    pub checkpoint: ComputePluginAuthorityRollbackCheckpoint,
    pub canonicalization: String,
    pub checkpoint_digest_algorithm: String,
    pub checkpoint_digest: String,
}

struct RollbackCheckpointRow {
    schema_version: i64,
    installation_id_digest: String,
    state_revision: i64,
    inventory_revision: i64,
    inventory_digest: String,
    inventory_json: String,
    desired_policy_revision: i64,
    sharing_enabled: i64,
    sharing_authorization_ref: Option<String>,
    sharing_authorization_revision: Option<i64>,
    sharing_authorization_digest: Option<String>,
    node_profile_digest: String,
    manifest_catalog_revision: i64,
    target_id: String,
    host_api_protocol_id: String,
    host_api_revision: i64,
    active_bundle_revision: Option<i64>,
    publisher_keyring_revision: Option<i64>,
    publisher_keyring_digest: Option<String>,
    control_keyring_revision: Option<i64>,
    control_keyring_digest: Option<String>,
    authority_epoch: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms: Option<i64>,
    clock_status: String,
    updated_at_ms: i64,
}

impl ComputePluginLocalAuthority {
    /// Produces a side-effect-free checkpoint candidate. It is not externally anchored until a
    /// future witness protocol authenticates and durably acknowledges this exact digest.
    pub(crate) fn read_rollback_checkpoint(
        &self,
        expected_installation: &ComputePluginInstallationIdentity,
    ) -> Result<HashedComputePluginAuthorityRollbackCheckpoint> {
        self.with_deferred(|transaction| {
            let row = read_checkpoint_row(transaction)?;
            let checkpoint = validate_and_project_checkpoint(row, expected_installation)?;
            let checkpoint_digest = jcs_sha256_hex(&checkpoint)?;
            Ok(HashedComputePluginAuthorityRollbackCheckpoint {
                schema: HASHED_COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA.to_string(),
                checkpoint,
                canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
                checkpoint_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
                checkpoint_digest,
            })
        })
    }
}

fn read_checkpoint_row(transaction: &Transaction<'_>) -> Result<RollbackCheckpointRow> {
    transaction
        .query_row(
            r#"SELECT
                schema_version, installation_id_digest, state_revision,
                inventory_revision, inventory_digest, inventory_json,
                desired_policy_revision, sharing_enabled,
                sharing_authorization_ref, sharing_authorization_revision,
                sharing_authorization_digest,
                node_profile_digest, manifest_catalog_revision, target_id,
                host_api_protocol_id, host_api_revision, active_bundle_revision,
                publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest,
                authority_epoch, process_owner_epoch, trusted_time_high_water_ms,
                clock_status, updated_at_ms
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok(RollbackCheckpointRow {
                    schema_version: row.get(0)?,
                    installation_id_digest: row.get(1)?,
                    state_revision: row.get(2)?,
                    inventory_revision: row.get(3)?,
                    inventory_digest: row.get(4)?,
                    inventory_json: row.get(5)?,
                    desired_policy_revision: row.get(6)?,
                    sharing_enabled: row.get(7)?,
                    sharing_authorization_ref: row.get(8)?,
                    sharing_authorization_revision: row.get(9)?,
                    sharing_authorization_digest: row.get(10)?,
                    node_profile_digest: row.get(11)?,
                    manifest_catalog_revision: row.get(12)?,
                    target_id: row.get(13)?,
                    host_api_protocol_id: row.get(14)?,
                    host_api_revision: row.get(15)?,
                    active_bundle_revision: row.get(16)?,
                    publisher_keyring_revision: row.get(17)?,
                    publisher_keyring_digest: row.get(18)?,
                    control_keyring_revision: row.get(19)?,
                    control_keyring_digest: row.get(20)?,
                    authority_epoch: row.get(21)?,
                    process_owner_epoch: row.get(22)?,
                    trusted_time_high_water_ms: row.get(23)?,
                    clock_status: row.get(24)?,
                    updated_at_ms: row.get(25)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_ROLLBACK_CHECKPOINT_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))
}

fn validate_and_project_checkpoint(
    row: RollbackCheckpointRow,
    expected_installation: &ComputePluginInstallationIdentity,
) -> Result<ComputePluginAuthorityRollbackCheckpoint> {
    let sharing_enabled = match row.sharing_enabled {
        0 => false,
        1 => true,
        _ => bail!("COMPUTE_PLUGIN_ROLLBACK_CHECKPOINT_SHARING_FLAG"),
    };
    let trusted_time_high_water_ms = row
        .trusted_time_high_water_ms
        .filter(|_| row.clock_status == "trusted")
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_CHECKPOINT_CLOCK_UNTRUSTED"))?;
    let inventory: ComputePluginInventorySnapshot = serde_json::from_str(&row.inventory_json)
        .context("COMPUTE_PLUGIN_ROLLBACK_CHECKPOINT_INVENTORY_JSON")?;
    let inventory_observed_at = DateTime::parse_from_rfc3339(&inventory.observed_at)
        .context("COMPUTE_PLUGIN_ROLLBACK_CHECKPOINT_INVENTORY_TIME")?;

    if row.schema_version != COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION
        || row.installation_id_digest != expected_installation.digest()
        || !is_sha256(&row.installation_id_digest)
        || row.state_revision < 0
        || row.inventory_revision < 0
        || !is_sha256(&row.inventory_digest)
        || row.desired_policy_revision < 0
        || !is_sha256(&row.node_profile_digest)
        || row.manifest_catalog_revision < 0
        || !is_identifier(&row.target_id)
        || !is_identifier(&row.host_api_protocol_id)
        || !(0..=i64::from(u32::MAX)).contains(&row.host_api_revision)
        || row.authority_epoch < 0
        || row.process_owner_epoch < 0
        || trusted_time_high_water_ms < 0
        || row.updated_at_ms != trusted_time_high_water_ms
        || inventory.schema != COMPUTE_PLUGIN_INVENTORY_SCHEMA
        || inventory.inventory_revision != row.inventory_revision
        || inventory.desired_policy_revision != row.desired_policy_revision
        || inventory.sharing_enabled != sharing_enabled
        || inventory.plugins.len() > MAX_COMPUTE_PLUGIN_INVENTORY_RECORDS
        || inventory
            .plugins
            .windows(2)
            .any(|pair| pair[0].plugin_id >= pair[1].plugin_id)
        || inventory
            .plugins
            .iter()
            .any(|record| !local_record_shape_is_valid(record))
        || inventory_observed_at.offset().local_minus_utc() != 0
        || inventory_observed_at.with_timezone(&Utc).timestamp_millis() > trusted_time_high_water_ms
        || jcs_sha256_hex(&inventory)? != row.inventory_digest
        || !sharing_binding_is_valid(
            sharing_enabled,
            row.sharing_authorization_ref.as_deref(),
            row.sharing_authorization_revision,
            row.sharing_authorization_digest.as_deref(),
        )
        || !keyring_binding_is_valid(&row)
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_CHECKPOINT_AUTHORITY_CORRUPT");
    }

    Ok(ComputePluginAuthorityRollbackCheckpoint {
        schema: COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA.to_string(),
        authority_schema_version: row.schema_version,
        installation_id_digest: row.installation_id_digest,
        state_revision: row.state_revision,
        inventory_revision: row.inventory_revision,
        inventory_digest: row.inventory_digest,
        desired_policy_revision: row.desired_policy_revision,
        sharing_enabled,
        sharing_authorization_ref_digest: row
            .sharing_authorization_ref
            .as_deref()
            .map(sharing_authorization_ref_digest),
        sharing_authorization_revision: row.sharing_authorization_revision,
        sharing_authorization_digest: row.sharing_authorization_digest,
        node_profile_digest: row.node_profile_digest,
        manifest_catalog_revision: row.manifest_catalog_revision,
        target_id: row.target_id,
        host_api_protocol_id: row.host_api_protocol_id,
        host_api_revision: row.host_api_revision,
        active_bundle_revision: row.active_bundle_revision,
        publisher_keyring_revision: row.publisher_keyring_revision,
        publisher_keyring_digest: row.publisher_keyring_digest,
        control_keyring_revision: row.control_keyring_revision,
        control_keyring_digest: row.control_keyring_digest,
        authority_epoch: row.authority_epoch,
        process_owner_epoch: row.process_owner_epoch,
        trusted_time_high_water_ms,
        updated_at_ms: row.updated_at_ms,
    })
}

fn sharing_binding_is_valid(
    sharing_enabled: bool,
    reference: Option<&str>,
    revision: Option<i64>,
    digest: Option<&str>,
) -> bool {
    match (reference, revision, digest) {
        (None, None, None) => !sharing_enabled,
        (Some(reference), Some(revision), Some(digest)) => {
            is_identifier(reference) && revision >= 0 && is_sha256(digest)
        }
        _ => false,
    }
}

fn sharing_authorization_ref_digest(reference: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(SHARING_AUTHORIZATION_REF_DOMAIN);
    digest.update([0]);
    digest.update(reference.as_bytes());
    hex::encode(digest.finalize())
}

fn keyring_binding_is_valid(row: &RollbackCheckpointRow) -> bool {
    match (
        row.active_bundle_revision,
        row.publisher_keyring_revision,
        row.publisher_keyring_digest.as_deref(),
        row.control_keyring_revision,
        row.control_keyring_digest.as_deref(),
    ) {
        (None, None, None, None, None) => true,
        (
            Some(bundle),
            Some(publisher_revision),
            Some(publisher_digest),
            Some(control_revision),
            Some(control_digest),
        ) => {
            bundle > 0
                && publisher_revision > 0
                && control_revision > 0
                && is_sha256(publisher_digest)
                && is_sha256(control_digest)
        }
        _ => false,
    }
}
