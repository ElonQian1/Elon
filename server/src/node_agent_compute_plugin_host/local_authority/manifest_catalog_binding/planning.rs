use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Transaction};

use super::{
    types::{
        ManifestCatalogBindingRequestDigest, PreparedManifestCatalogBindingRequest,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_REQUEST_SCHEMA,
    },
    write::{read_binding_by_revision, validate_exact_request},
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    keyring::{ComputePluginBootstrapRootKeyResolver, ComputePluginKeyringBinding},
    local_authority::keyring_snapshot::{
        load_snapshot_for_state, read_authority_keyring_state, KeyringSnapshotValidation,
    },
    local_authority::plan_application::AuthorityPlanApplicationState,
    manifest_catalog::{
        verify_manifest_catalog_candidate, ComputePluginManifestCatalog,
        ComputePluginManifestCatalogCandidate, SignedComputePluginManifestCatalog,
    },
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
};

/// A planning-only catalog rebuilt from the stored signed source envelopes and the exact current
/// root-validated keyring. It grants no download, installation, root or runtime capability.
pub(in crate::node_agent_compute_plugin_host::local_authority) struct PlanningCatalogBinding {
    catalog: ComputePluginManifestCatalog,
    catalog_digest: String,
    binding_receipt_digest: String,
    node_profile_digest: String,
    catalog_revision: u64,
    keyring_bundle_revision: u64,
    publisher_keyring: homecli_proto::ComputePluginInstallPlanKeyringBindingV1,
    control_keyring: homecli_proto::ComputePluginInstallPlanKeyringBindingV1,
    target_id: String,
    host_api_protocol_id: String,
    host_api_revision: u32,
}

impl PlanningCatalogBinding {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn catalog(
        &self,
    ) -> &ComputePluginManifestCatalog {
        &self.catalog
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn catalog_digest(
        &self,
    ) -> &str {
        &self.catalog_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn binding_receipt_digest(
        &self,
    ) -> &str {
        &self.binding_receipt_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn node_profile_digest(
        &self,
    ) -> &str {
        &self.node_profile_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn catalog_revision(
        &self,
    ) -> u64 {
        self.catalog_revision
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn keyring_bundle_revision(
        &self,
    ) -> u64 {
        self.keyring_bundle_revision
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn publisher_keyring(
        &self,
    ) -> &homecli_proto::ComputePluginInstallPlanKeyringBindingV1 {
        &self.publisher_keyring
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn control_keyring(
        &self,
    ) -> &homecli_proto::ComputePluginInstallPlanKeyringBindingV1 {
        &self.control_keyring
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn target_id(&self) -> &str {
        &self.target_id
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn host_api_protocol_id(
        &self,
    ) -> &str {
        &self.host_api_protocol_id
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn host_api_revision(
        &self,
    ) -> u32 {
        self.host_api_revision
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn signed_manifest_envelope_digest_for(
        &self,
        plugin_id: &str,
        release: &ComputePluginReleaseRef,
    ) -> Result<Option<&str>> {
        if plugin_id.is_empty() || release.plugin_id != plugin_id {
            bail!("COMPUTE_PLUGIN_PLANNING_CATALOG_RELEASE_CHANGED");
        }
        let mut matches = self
            .catalog
            .entries
            .iter()
            .filter(|entry| entry.release == *release)
            .map(|entry| entry.signed_manifest_envelope_digest.as_str());
        let digest = matches.next();
        if matches.next().is_some() {
            bail!("COMPUTE_PLUGIN_PLANNING_CATALOG_RELEASE_AMBIGUOUS");
        }
        Ok(digest)
    }
}

pub(in crate::node_agent_compute_plugin_host::local_authority) fn read_planning_catalog_binding_on(
    transaction: &Transaction<'_>,
    trusted_now: DateTime<Utc>,
    authority: &AuthorityPlanApplicationState,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
) -> Result<PlanningCatalogBinding> {
    let state = planning_state(transaction, authority)?;
    if state.manifest_catalog_revision <= 0 {
        bail!("COMPUTE_PLUGIN_PLANNING_CATALOG_UNAVAILABLE");
    }

    let keyring_state = read_authority_keyring_state(transaction)?;
    if keyring_state.state_revision != state.state_revision
        || keyring_state.authority_epoch != state.authority_epoch
    {
        bail!("COMPUTE_PLUGIN_PLANNING_CATALOG_KEYRING_FENCE_CHANGED");
    }
    let keyring = load_snapshot_for_state(
        transaction,
        &keyring_state,
        KeyringSnapshotValidation::Current(trusted_now.clone()),
        roots,
    )?;
    if keyring.bundle_revision() != state.keyring_bundle_revision
        || keyring.publisher_binding() != &state.publisher_keyring
        || keyring.control_binding() != &state.control_keyring
    {
        bail!("COMPUTE_PLUGIN_PLANNING_CATALOG_KEYRING_CHANGED");
    }

    let stored = read_binding_by_revision(transaction, state.manifest_catalog_revision)?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_CATALOG_RECEIPT_UNAVAILABLE"))?;
    validate_planning_catalog_head(transaction, &state, &stored.hashed_receipt)?;

    let signed_catalog: SignedComputePluginManifestCatalog =
        serde_json::from_str(&stored.request.signed_catalog_json)
            .context("COMPUTE_PLUGIN_PLANNING_SIGNED_CATALOG_JSON")?;
    let signed_manifests: Vec<SignedComputePluginManifest> =
        serde_json::from_str(&stored.request.signed_manifests_json)
            .context("COMPUTE_PLUGIN_PLANNING_SIGNED_MANIFESTS_JSON")?;
    let candidate = ComputePluginManifestCatalogCandidate::new(
        stored.request.request_id.clone(),
        signed_catalog,
        signed_manifests,
    )?;
    let validated = verify_manifest_catalog_candidate(
        &candidate,
        &state.target_id,
        &state.host_api_protocol_id,
        state.host_api_revision,
        state.keyring_bundle_revision,
        &state.publisher_keyring,
        &state.control_keyring,
        trusted_now,
        &keyring,
        &keyring,
    )?;
    let expected = rebuild_verified_request(&stored.request.request_id, &state, &validated)?;
    validate_exact_request(&stored.request, &expected)?;

    let receipt = &stored.hashed_receipt.receipt;
    if receipt.catalog_digest != validated.catalog_digest()
        || receipt.keyring_bundle_revision != keyring.bundle_revision()
        || &receipt.publisher_keyring != keyring.publisher_binding()
        || &receipt.control_keyring != keyring.control_binding()
    {
        bail!("COMPUTE_PLUGIN_PLANNING_CATALOG_RECEIPT_CHANGED");
    }
    let catalog_revision = to_u64(receipt.catalog_revision, "CATALOG_REVISION")?;
    let binding_receipt_digest = stored.hashed_receipt.receipt_digest.clone();

    Ok(PlanningCatalogBinding {
        catalog: validated.catalog().clone(),
        catalog_digest: validated.catalog_digest().to_string(),
        binding_receipt_digest,
        node_profile_digest: state.node_profile_digest,
        catalog_revision,
        keyring_bundle_revision: to_u64(keyring.bundle_revision(), "KEYRING_BUNDLE_REVISION")?,
        publisher_keyring: wire_keyring(keyring.publisher_binding(), "PUBLISHER_KEYRING")?,
        control_keyring: wire_keyring(keyring.control_binding(), "CONTROL_KEYRING")?,
        target_id: state.target_id,
        host_api_protocol_id: state.host_api_protocol_id,
        host_api_revision: state.host_api_revision,
    })
}

fn planning_state(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
) -> Result<super::types::ManifestCatalogAuthorityState> {
    let updated_at_ms = transaction
        .query_row(
            "SELECT updated_at_ms FROM authority_meta WHERE singleton = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_PLANNING_CATALOG_UPDATED_AT_READ")?;
    Ok(super::types::ManifestCatalogAuthorityState {
        installation_id_digest: authority.installation_id_digest.clone(),
        state_revision: authority.state_revision,
        inventory_revision: authority.inventory.inventory_revision,
        inventory_digest: authority.inventory_digest.clone(),
        inventory_json: authority.inventory_json.clone(),
        desired_policy_revision: authority.desired_policy_revision,
        sharing_enabled: authority.sharing_enabled,
        node_profile_digest: authority.node_profile_digest.clone(),
        manifest_catalog_revision: authority.manifest_catalog_revision,
        target_id: authority.target_id.clone(),
        host_api_protocol_id: authority.host_api_protocol_id.clone(),
        host_api_revision: authority.host_api_revision,
        authority_epoch: authority.authority_epoch,
        process_owner_epoch: authority.process_owner_epoch,
        trusted_time_high_water_ms: authority.trusted_time_high_water_ms,
        updated_at_ms,
        keyring_bundle_revision: authority.keyring_bundle_revision,
        publisher_keyring: authority.publisher_keyring.clone(),
        control_keyring: authority.control_keyring.clone(),
    })
}

fn validate_planning_catalog_head(
    transaction: &Transaction<'_>,
    state: &super::types::ManifestCatalogAuthorityState,
    receipt: &super::types::HashedComputePluginManifestCatalogBindingReceipt,
) -> Result<()> {
    let receipt = &receipt.receipt;
    let matches = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM authority_meta WHERE singleton = 1
              AND installation_id_digest = ?1 AND state_revision = ?2
              AND inventory_revision = ?3 AND inventory_digest = ?4 AND inventory_json = ?5
              AND desired_policy_revision = ?6 AND sharing_enabled = ?7
              AND node_profile_digest = ?8 AND manifest_catalog_revision = ?9
              AND target_id = ?10 AND host_api_protocol_id = ?11 AND host_api_revision = ?12
              AND authority_epoch = ?13 AND process_owner_epoch = ?14
              AND trusted_time_high_water_ms = ?15 AND updated_at_ms = ?16
              AND active_bundle_revision = ?17
              AND publisher_keyring_revision = ?18 AND publisher_keyring_digest = ?19
              AND control_keyring_revision = ?20 AND control_keyring_digest = ?21
              AND clock_status = 'trusted'"#,
            params![
                &state.installation_id_digest,
                state.state_revision,
                state.inventory_revision,
                &state.inventory_digest,
                &state.inventory_json,
                state.desired_policy_revision,
                state.sharing_enabled,
                &state.node_profile_digest,
                state.manifest_catalog_revision,
                &state.target_id,
                &state.host_api_protocol_id,
                state.host_api_revision,
                state.authority_epoch,
                state.process_owner_epoch,
                state.trusted_time_high_water_ms,
                state.updated_at_ms,
                state.keyring_bundle_revision,
                state.publisher_keyring.revision,
                &state.publisher_keyring.digest,
                state.control_keyring.revision,
                &state.control_keyring.digest,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_PLANNING_CATALOG_CURRENT_HEAD_READ")?;
    if matches != 1
        || receipt.installation_id_digest != state.installation_id_digest
        || receipt.catalog_revision != state.manifest_catalog_revision
        || receipt.node_profile_digest != state.node_profile_digest
        || receipt.target_id != state.target_id
        || receipt.host_api_protocol_id != state.host_api_protocol_id
        || receipt.host_api_revision != state.host_api_revision
        || receipt.keyring_bundle_revision != state.keyring_bundle_revision
        || receipt.publisher_keyring != state.publisher_keyring
        || receipt.control_keyring != state.control_keyring
        || receipt.state_revision_after > state.state_revision
        || receipt.authority_epoch_after > state.authority_epoch
        || receipt.process_owner_epoch > state.process_owner_epoch
        || receipt.bound_at_ms > state.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_PLANNING_CATALOG_NOT_CURRENT");
    }
    Ok(())
}

fn rebuild_verified_request(
    request_id: &str,
    state: &super::types::ManifestCatalogAuthorityState,
    validated: &crate::node_agent_compute_plugin_host::manifest_catalog::ValidatedComputePluginManifestCatalog,
) -> Result<PreparedManifestCatalogBindingRequest> {
    let catalog_entry_count = i64::try_from(validated.catalog().entries.len())
        .context("COMPUTE_PLUGIN_PLANNING_CATALOG_ENTRY_COUNT_RANGE")?;
    let request_digest = jcs_sha256_hex(&ManifestCatalogBindingRequestDigest {
        schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_REQUEST_SCHEMA,
        request_id,
        installation_id_digest: &state.installation_id_digest,
        catalog_revision: validated.catalog().catalog_revision,
        catalog_digest: validated.catalog_digest(),
        signed_catalog_envelope_digest: validated.signed_catalog_envelope_digest(),
        control_signing_key_id: validated.control_signing_key_id(),
        control_signing_key_fingerprint: validated.control_signing_key_fingerprint(),
        signed_manifest_set_digest: validated.signed_manifest_set_digest(),
        node_profile_digest: &state.node_profile_digest,
        target_id: &state.target_id,
        host_api_protocol_id: &state.host_api_protocol_id,
        host_api_revision: state.host_api_revision,
        keyring_bundle_revision: state.keyring_bundle_revision,
        publisher_keyring: &state.publisher_keyring,
        control_keyring: &state.control_keyring,
    })?;
    Ok(PreparedManifestCatalogBindingRequest {
        request_id: request_id.to_string(),
        request_digest,
        installation_id_digest: state.installation_id_digest.clone(),
        catalog_revision: validated.catalog().catalog_revision,
        catalog_json: validated.catalog_json().to_string(),
        catalog_digest: validated.catalog_digest().to_string(),
        signed_catalog_json: validated.signed_catalog_json().to_string(),
        signed_catalog_envelope_digest: validated.signed_catalog_envelope_digest().to_string(),
        control_signing_key_id: validated.control_signing_key_id().to_string(),
        control_signing_key_fingerprint: validated.control_signing_key_fingerprint().to_string(),
        signed_manifests_json: validated.signed_manifests_json().to_string(),
        signed_manifest_set_digest: validated.signed_manifest_set_digest().to_string(),
        catalog_entry_count,
        node_profile_digest: state.node_profile_digest.clone(),
        target_id: state.target_id.clone(),
        host_api_protocol_id: state.host_api_protocol_id.clone(),
        host_api_revision: state.host_api_revision,
        keyring_bundle_revision: state.keyring_bundle_revision,
        publisher_keyring: state.publisher_keyring.clone(),
        control_keyring: state.control_keyring.clone(),
    })
}

fn wire_keyring(
    binding: &ComputePluginKeyringBinding,
    field: &'static str,
) -> Result<homecli_proto::ComputePluginInstallPlanKeyringBindingV1> {
    Ok(homecli_proto::ComputePluginInstallPlanKeyringBindingV1 {
        revision: to_u64(binding.revision, field)?,
        digest: binding.digest.clone(),
    })
}

fn to_u64(value: i64, field: &'static str) -> Result<u64> {
    u64::try_from(value).with_context(|| format!("COMPUTE_PLUGIN_PLANNING_{field}_RANGE"))
}
