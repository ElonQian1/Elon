use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension};

use super::ComputePluginLocalAuthority;
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginInstallationIdentity,
    install_plan_admission_validation::is_identifier,
    lifecycle::{ComputePluginInventorySnapshot, COMPUTE_PLUGIN_INVENTORY_SCHEMA},
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

#[derive(Debug, Clone)]
pub(crate) struct ComputePluginAuthorityInitialization {
    pub installation_identity: ComputePluginInstallationIdentity,
    pub inventory: ComputePluginInventorySnapshot,
    pub node_profile_digest: String,
    pub manifest_catalog_revision: i64,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: u32,
    pub initialized_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComputePluginAuthorityInitializationOutcome {
    Initialized,
    AlreadyInitialized,
}

impl ComputePluginLocalAuthority {
    /// Creates only the disabled, empty singleton. Sharing authorization and keyring activation
    /// remain separate purpose-specific transactions.
    pub(crate) fn initialize(
        &self,
        initial: &ComputePluginAuthorityInitialization,
    ) -> Result<ComputePluginAuthorityInitializationOutcome> {
        validate_initial_state(initial)?;
        let inventory_digest = jcs_sha256_hex(&initial.inventory)?;
        let inventory_json = serde_json::to_string(&initial.inventory)
            .context("COMPUTE_PLUGIN_AUTHORITY_INVENTORY_JSON")?;
        self.with_immediate(|transaction| {
            let existing = transaction
                .query_row(
                    "SELECT installation_id_digest FROM authority_meta WHERE singleton = 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .context("COMPUTE_PLUGIN_AUTHORITY_META_READ")?;
            if let Some(existing_digest) = existing {
                if existing_digest != initial.installation_identity.digest() {
                    bail!(
                        "COMPUTE_PLUGIN_AUTHORITY_INSTALLATION_CHANGED: database belongs to another node installation"
                    );
                }
                return Ok(ComputePluginAuthorityInitializationOutcome::AlreadyInitialized);
            }
            let inserted = transaction
                .execute(
                    r#"INSERT INTO authority_meta (
                        singleton, schema_version, installation_id_digest,
                        state_revision, inventory_revision, inventory_digest, inventory_json,
                        desired_policy_revision, sharing_enabled,
                        sharing_authorization_ref, sharing_authorization_revision,
                        sharing_authorization_digest, node_profile_digest,
                        manifest_catalog_revision, target_id, host_api_protocol_id,
                        host_api_revision, active_bundle_revision,
                        publisher_keyring_revision, publisher_keyring_digest,
                        control_keyring_revision, control_keyring_digest,
                        authority_epoch, process_owner_epoch,
                        trusted_time_high_water_ms, clock_status, updated_at_ms
                    ) VALUES (
                        1, 2, ?1,
                        0, 0, ?2, ?3,
                        ?4, 0,
                        NULL, NULL,
                        NULL, ?5,
                        ?6, ?7, ?8,
                        ?9, NULL,
                        NULL, NULL,
                        NULL, NULL,
                        0, 0,
                        NULL, 'uninitialized', ?10
                    )"#,
                    params![
                        initial.installation_identity.digest(),
                        inventory_digest,
                        inventory_json,
                        initial.inventory.desired_policy_revision,
                        &initial.node_profile_digest,
                        initial.manifest_catalog_revision,
                        &initial.target_id,
                        &initial.host_api_protocol_id,
                        i64::from(initial.host_api_revision),
                        initial.initialized_at.timestamp_millis(),
                    ],
                )
                .context("COMPUTE_PLUGIN_AUTHORITY_META_INSERT")?;
            if inserted != 1 {
                bail!("COMPUTE_PLUGIN_AUTHORITY_META_CAS: singleton initialization was not unique");
            }
            Ok(ComputePluginAuthorityInitializationOutcome::Initialized)
        })
    }
}

fn validate_initial_state(initial: &ComputePluginAuthorityInitialization) -> Result<()> {
    let observed_at = DateTime::parse_from_rfc3339(&initial.inventory.observed_at)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_INITIAL_TIME"))?;
    if initial.inventory.schema != COMPUTE_PLUGIN_INVENTORY_SCHEMA
        || initial.inventory.inventory_revision != 0
        || initial.inventory.desired_policy_revision < 0
        || initial.inventory.sharing_enabled
        || !initial.inventory.plugins.is_empty()
        || !is_sha256(initial.installation_identity.digest())
        || !is_sha256(&initial.node_profile_digest)
        || initial.manifest_catalog_revision < 0
        || !is_identifier(&initial.target_id)
        || !is_identifier(&initial.host_api_protocol_id)
        || observed_at.offset().local_minus_utc() != 0
        || observed_at
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true)
            != initial.inventory.observed_at
    {
        bail!("COMPUTE_PLUGIN_AUTHORITY_INITIAL_STATE: initial authority facts are not canonical");
    }
    Ok(())
}
