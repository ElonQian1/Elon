use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::super::{
    plan_application::{prepare_application_request, AuthorityPlanApplicationState},
    plan_application_projection::PersistedAdmissionBindings,
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan::{
        install_plan_shape_is_valid, ComputePluginGrantBinding, SignedComputePluginInstallPlan,
        PLAN_ACTION_REAUTHORIZE_EXISTING,
    },
    install_plan_reauthorization::{
        reauthorization_shape_is_valid, validate_reauthorization_source,
    },
    lifecycle::ComputePluginLocalRecord,
    manifest_catalog::ComputePluginManifestCatalog,
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) struct CurrentWorkAdmissionSource {
    pub(super) signed_manifest: SignedComputePluginManifest,
    pub(super) signed_manifest_envelope_digest: String,
    pub(super) grant: ComputePluginGrantBinding,
    pub(super) action: String,
    pub(super) plan_id: String,
    pub(super) plan_digest: String,
    pub(super) signed_plan_envelope_digest: String,
    pub(super) signed_manifest_set_digest: String,
    pub(super) application_request_digest: String,
    pub(super) application_receipt_digest: String,
    pub(super) admission_bindings_digest: String,
    pub(super) application_inventory_revision: i64,
    pub(super) policy_binding_receipt_digest: String,
    pub(super) policy_revocation_receipt_digest: String,
    pub(super) manifest_catalog_digest: String,
    pub(super) manifest_catalog_binding_receipt_digest: String,
}

struct StoredApplication {
    plan_digest: String,
    application_request_digest: String,
    application_receipt_digest: String,
    signed_plan_envelope_digest: String,
    signed_manifest_set_digest: String,
    signed_plan_json: String,
    signed_manifests_json: String,
    admission_bindings_json: String,
    admission_bindings_digest: String,
    inventory_after_json: String,
    inventory_after_digest: String,
    application_inventory_revision: i64,
    application_state_revision: i64,
    authority_epoch_at_apply: i64,
    keyring_bundle_revision: i64,
    publisher_keyring_revision: i64,
    publisher_keyring_digest: String,
    control_keyring_revision: i64,
    control_keyring_digest: String,
    applied_at_ms: i64,
    expires_at_ms: i64,
}

pub(super) fn read_current_source(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    authority_updated_at_ms: i64,
    record: &ComputePluginLocalRecord,
    release: &ComputePluginReleaseRef,
    admitted_at_ms: i64,
) -> Result<CurrentWorkAdmissionSource> {
    let plan_id = record
        .last_plan_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_PLAN_MISSING"))?;
    let stored = read_application(transaction, plan_id)?;
    let signed_plan: SignedComputePluginInstallPlan =
        serde_json::from_str(&stored.signed_plan_json)
            .context("COMPUTE_PLUGIN_WORK_ADMISSION_SIGNED_PLAN_PARSE")?;
    let signed_manifests: Vec<SignedComputePluginManifest> =
        serde_json::from_str(&stored.signed_manifests_json)
            .context("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_SET_PARSE")?;
    let admission: PersistedAdmissionBindings =
        serde_json::from_str(&stored.admission_bindings_json)
            .context("COMPUTE_PLUGIN_WORK_ADMISSION_BINDINGS_PARSE")?;
    let request = prepare_application_request(&signed_plan, &signed_manifests)?;
    validate_application(
        authority,
        authority_updated_at_ms,
        admitted_at_ms,
        plan_id,
        &stored,
        &signed_plan,
        &signed_manifests,
        &admission,
        &request,
    )?;

    let mut item_matches = signed_plan.plan.items.iter().filter(|item| {
        item.action == PLAN_ACTION_REAUTHORIZE_EXISTING
            && item.expected_current_release.as_ref() == Some(release)
    });
    let item = item_matches
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_PLAN_ITEM_MISSING"))?;
    if item_matches.next().is_some()
        || !reauthorization_shape_is_valid(item)
        || item.expected_install_generation != Some(record.install_generation)
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_PLAN_ITEM_CHANGED");
    }
    validate_reauthorization_source(item, record)?;
    let grant = item
        .grant
        .clone()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_GRANT_MISSING"))?;

    let mut manifest_matches = signed_manifests
        .into_iter()
        .filter(|signed| release_matches(signed, release));
    let signed_manifest = manifest_matches
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_MISSING"))?;
    if manifest_matches.next().is_some()
        || jcs_sha256_hex(&signed_manifest.manifest)? != signed_manifest.manifest_digest
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_CHANGED");
    }
    let signed_manifest_envelope_digest = jcs_sha256_hex(&signed_manifest)?;
    let (policy_binding_receipt_digest, policy_revocation_receipt_digest) =
        read_policy_receipts(transaction, authority, stored.applied_at_ms)?;
    let (manifest_catalog_digest, manifest_catalog_binding_receipt_digest) =
        validate_catalog_source(
            transaction,
            authority,
            release,
            &signed_manifest,
            &signed_manifest_envelope_digest,
        )?;
    validate_keyring_source(transaction, authority, admitted_at_ms)?;

    Ok(CurrentWorkAdmissionSource {
        signed_manifest,
        signed_manifest_envelope_digest,
        grant,
        action: PLAN_ACTION_REAUTHORIZE_EXISTING.to_string(),
        plan_id: plan_id.to_string(),
        plan_digest: stored.plan_digest,
        signed_plan_envelope_digest: stored.signed_plan_envelope_digest,
        signed_manifest_set_digest: stored.signed_manifest_set_digest,
        application_request_digest: stored.application_request_digest,
        application_receipt_digest: stored.application_receipt_digest,
        admission_bindings_digest: stored.admission_bindings_digest,
        application_inventory_revision: stored.application_inventory_revision,
        policy_binding_receipt_digest,
        policy_revocation_receipt_digest,
        manifest_catalog_digest,
        manifest_catalog_binding_receipt_digest,
    })
}

fn read_application(transaction: &Transaction<'_>, plan_id: &str) -> Result<StoredApplication> {
    transaction
        .query_row(
            r#"SELECT application.plan_digest, application.application_request_digest,
                application.receipt_digest, application.signed_plan_envelope_digest,
                application.signed_manifest_set_digest, application.signed_plan_json,
                application.signed_manifests_json, application.admission_bindings_json,
                application.admission_bindings_digest, application.inventory_after_json,
                application.inventory_after_digest, application.application_inventory_revision,
                application.application_state_revision, application.authority_epoch_at_apply,
                application.keyring_bundle_revision, application.publisher_keyring_revision,
                application.publisher_keyring_digest, application.control_keyring_revision,
                application.control_keyring_digest, application.applied_at_ms,
                application.expires_at_ms
            FROM plan_applications AS application
            JOIN plan_application_seals AS seal
              ON seal.plan_id = application.plan_id
             AND seal.plan_digest = application.plan_digest
             AND seal.application_request_digest = application.application_request_digest
             AND seal.receipt_digest = application.receipt_digest
             AND seal.sealed_at_ms = application.applied_at_ms
            WHERE application.plan_id = ?1"#,
            [plan_id],
            |row| {
                Ok(StoredApplication {
                    plan_digest: row.get(0)?,
                    application_request_digest: row.get(1)?,
                    application_receipt_digest: row.get(2)?,
                    signed_plan_envelope_digest: row.get(3)?,
                    signed_manifest_set_digest: row.get(4)?,
                    signed_plan_json: row.get(5)?,
                    signed_manifests_json: row.get(6)?,
                    admission_bindings_json: row.get(7)?,
                    admission_bindings_digest: row.get(8)?,
                    inventory_after_json: row.get(9)?,
                    inventory_after_digest: row.get(10)?,
                    application_inventory_revision: row.get(11)?,
                    application_state_revision: row.get(12)?,
                    authority_epoch_at_apply: row.get(13)?,
                    keyring_bundle_revision: row.get(14)?,
                    publisher_keyring_revision: row.get(15)?,
                    publisher_keyring_digest: row.get(16)?,
                    control_keyring_revision: row.get(17)?,
                    control_keyring_digest: row.get(18)?,
                    applied_at_ms: row.get(19)?,
                    expires_at_ms: row.get(20)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_PLAN_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_PLAN_UNSEALED"))
}

#[allow(clippy::too_many_arguments)]
fn validate_application(
    authority: &AuthorityPlanApplicationState,
    authority_updated_at_ms: i64,
    admitted_at_ms: i64,
    plan_id: &str,
    stored: &StoredApplication,
    signed_plan: &SignedComputePluginInstallPlan,
    signed_manifests: &[SignedComputePluginManifest],
    admission: &PersistedAdmissionBindings,
    request: &super::super::plan_application::PreparedPlanApplicationRequest,
) -> Result<()> {
    let sharing = authority
        .sharing_authorization
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_SHARING_MISSING"))?;
    if !install_plan_shape_is_valid(&signed_plan.plan)
        || signed_plan.plan.plan_id != plan_id
        || signed_plan.plan_digest != stored.plan_digest
        || jcs_sha256_hex(&signed_plan.plan)? != stored.plan_digest
        || serde_json::to_string(signed_plan)? != stored.signed_plan_json
        || serde_json::to_string(signed_manifests)? != stored.signed_manifests_json
        || serde_json::to_string(admission)? != stored.admission_bindings_json
        || jcs_sha256_hex(admission)? != stored.admission_bindings_digest
        || request.signed_plan_envelope_digest != stored.signed_plan_envelope_digest
        || request.signed_manifest_set_digest != stored.signed_manifest_set_digest
        || request.application_request_digest != stored.application_request_digest
        || request.signed_manifests != signed_manifests
        || stored.inventory_after_json != authority.inventory_json
        || stored.inventory_after_digest != authority.inventory_digest
        || stored.application_inventory_revision != authority.inventory.inventory_revision
        || stored.application_state_revision != authority.state_revision
        || stored.authority_epoch_at_apply != authority.authority_epoch
        || stored.keyring_bundle_revision != authority.keyring_bundle_revision
        || stored.publisher_keyring_revision != authority.publisher_keyring.revision
        || stored.publisher_keyring_digest != authority.publisher_keyring.digest
        || stored.control_keyring_revision != authority.control_keyring.revision
        || stored.control_keyring_digest != authority.control_keyring.digest
        || stored.applied_at_ms != authority.trusted_time_high_water_ms
        || stored.applied_at_ms != authority_updated_at_ms
        || admitted_at_ms <= stored.applied_at_ms
        || admitted_at_ms >= stored.expires_at_ms
        || !authority.sharing_enabled
        || signed_plan.plan.desired_policy_revision != authority.desired_policy_revision
        || signed_plan.plan.sharing_authorization.as_ref() != Some(sharing)
        || signed_plan.plan.node_profile_digest != authority.node_profile_digest
        || signed_plan.plan.manifest_catalog_revision != authority.manifest_catalog_revision
        || signed_plan.plan.publisher_keyring != authority.publisher_keyring
        || signed_plan.plan.control_keyring != authority.control_keyring
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_PLAN_SOURCE_CHANGED");
    }
    Ok(())
}

fn read_policy_receipts(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    application_applied_at_ms: i64,
) -> Result<(String, String)> {
    let sharing = authority
        .sharing_authorization
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_SHARING_MISSING"))?;
    transaction
        .query_row(
            r#"SELECT policy.receipt_digest, revocation.receipt_digest
            FROM sharing_policy_binding_receipts AS policy
            JOIN sharing_policy_binding_revocation_receipts AS revocation
              ON revocation.policy_revision = policy.policy_revision
             AND revocation.policy_binding_receipt_digest = policy.receipt_digest
             AND revocation.installation_id_digest = policy.installation_id_digest
            WHERE policy.policy_revision = ?1 AND policy.installation_id_digest = ?2
              AND policy.sharing_enabled = 1 AND policy.policy_digest = ?3
              AND policy.sharing_authorization_ref = ?4
              AND policy.sharing_authorization_revision = ?1
              AND policy.sharing_authorization_digest = ?3
              AND policy.bound_at_ms <= ?5
              AND revocation.bound_at_ms = policy.bound_at_ms"#,
            params![
                authority.desired_policy_revision,
                authority.installation_id_digest,
                sharing.digest,
                sharing.authorization_ref,
                application_applied_at_ms,
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_POLICY_SOURCE_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_POLICY_SOURCE_MISSING"))
}

fn validate_catalog_source(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    release: &ComputePluginReleaseRef,
    signed_manifest: &SignedComputePluginManifest,
    envelope_digest: &str,
) -> Result<(String, String)> {
    let row: (String, String, String, String) = transaction
        .query_row(
            r#"SELECT catalog_json, catalog_digest, signed_manifests_json, receipt_digest
            FROM manifest_catalog_binding_receipts
            WHERE catalog_revision = ?1 AND installation_id_digest = ?2
              AND node_profile_digest = ?3 AND target_id = ?4
              AND host_api_protocol_id = ?5 AND host_api_revision = ?6
              AND keyring_bundle_revision = ?7
              AND publisher_keyring_revision = ?8 AND publisher_keyring_digest = ?9
              AND control_keyring_revision = ?10 AND control_keyring_digest = ?11
              AND bound_at_ms <= ?12"#,
            params![
                authority.manifest_catalog_revision,
                authority.installation_id_digest,
                authority.node_profile_digest,
                authority.target_id,
                authority.host_api_protocol_id,
                i64::from(authority.host_api_revision),
                authority.keyring_bundle_revision,
                authority.publisher_keyring.revision,
                authority.publisher_keyring.digest,
                authority.control_keyring.revision,
                authority.control_keyring.digest,
                authority.trusted_time_high_water_ms,
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_CATALOG_SOURCE_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_CATALOG_SOURCE_MISSING"))?;
    let catalog: ComputePluginManifestCatalog = serde_json::from_str(&row.0)?;
    let manifests: Vec<SignedComputePluginManifest> = serde_json::from_str(&row.2)?;
    let entry_count = catalog
        .entries
        .iter()
        .filter(|entry| {
            &entry.release == release
                && entry.publisher_id == signed_manifest.manifest.publisher_id
                && entry.signed_manifest_envelope_digest == envelope_digest
        })
        .count();
    let manifest_count = manifests
        .iter()
        .filter(|candidate| *candidate == signed_manifest)
        .count();
    if serde_json::to_string(&catalog)? != row.0
        || serde_json::to_string(&manifests)? != row.2
        || jcs_sha256_hex(&catalog)? != row.1
        || entry_count != 1
        || manifest_count != 1
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_CATALOG_SOURCE_CHANGED");
    }
    Ok((row.1, row.3))
}

fn validate_keyring_source(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    admitted_at_ms: i64,
) -> Result<()> {
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM keyring_bundles AS bundle
            JOIN keyring_seals AS seal ON seal.bundle_revision = bundle.bundle_revision
            WHERE bundle.bundle_revision = ?1
              AND bundle.publisher_revision = ?2 AND bundle.publisher_digest = ?3
              AND bundle.control_revision = ?4 AND bundle.control_digest = ?5
              AND bundle.expires_at_ms > ?6"#,
            params![
                authority.keyring_bundle_revision,
                authority.publisher_keyring.revision,
                authority.publisher_keyring.digest,
                authority.control_keyring.revision,
                authority.control_keyring.digest,
                admitted_at_ms,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_KEYRING_SOURCE_READ")?;
    if count != 1 {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_KEYRING_SOURCE_CHANGED");
    }
    Ok(())
}

fn release_matches(
    signed: &SignedComputePluginManifest,
    release: &ComputePluginReleaseRef,
) -> bool {
    signed.manifest.plugin_id == release.plugin_id
        && signed.manifest.plugin_version == release.plugin_version
        && signed.manifest.target.target_id == release.target_id
        && signed.manifest_digest == release.manifest_digest
        && signed.manifest.package.package_digest == release.package_digest
}
