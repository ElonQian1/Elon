use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_registry::{
        canonical_registry_provider_binding_receipt_json_and_digest,
        canonical_registry_release_receipt_json_and_digest,
        validate_registry_provider_binding_receipt, validate_registry_release_receipt,
    },
    store::{
        compute_external_pool_adapter_adoption::external_pool_adapter_adoption_receipt_authority_on,
        compute_external_pool_adapter_artifact_package::artifact_package_authority_on,
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on,
        compute_external_pool_adapter_release::admission_by_id_on,
    },
};

use super::{projection::route_adapter_projection_id, read::release_by_id_on, types::*};

pub(super) fn audit_release(
    conn: &Connection,
    stored: StoredRegistryRelease,
) -> Result<StoredRegistryRelease> {
    validate_registry_release_receipt(&stored.receipt)?;
    let (json, digest) = canonical_registry_release_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.release;
    let admission = admission_by_id_on(conn, &item.admission_id)?
        .ok_or_else(|| anyhow::anyhow!("Adapter registry release lost V222 admission root"))?;
    let package =
        artifact_package_authority_on(conn, &item.admission_id, &item.package_receipt_digest)?
            .ok_or_else(|| anyhow::anyhow!("Adapter registry release lost V232 package root"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        conn,
        &item.admission_id,
        &item.admission_digest,
        &item.source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter registry release lost V227 source root"))?;
    let package_receipt = package.receipt();
    let package_item = &package_receipt.package;
    if json != stored.receipt_json
        || digest != stored.receipt.registry_release_digest
        || admission.admission_digest != item.admission_digest
        || admission.adapter_id != item.adapter_id
        || admission.release_version != item.release_version
        || admission.declared_implementation_sha256 != item.declared_implementation_sha256
        || admission.supported_capabilities != item.supported_capabilities
        || admission.capability_set_digest != item.capability_set_digest
        || admission.expected_credential_verifier != item.credential_verifier
        || package_receipt.package_receipt_id != item.package_receipt_id
        || package_receipt.package_receipt_digest != item.package_receipt_digest
        || package_receipt.package_material_digest != item.package_material_digest
        || package_item.admission_id != item.admission_id
        || package_item.admission_digest != item.admission_digest
        || package_item.source_receipt_digest != item.source_receipt_digest
        || package_item.archive_sha256 != item.archive_sha256
        || package_item.archive_size_bytes != item.archive_size_bytes
        || package_item.manifest != item.manifest
        || package_item.manifest_digest != item.manifest_digest
        || package_item.entry_inventory_digest != item.entry_inventory_digest
        || package_item.entry_count != item.entry_count
        || package_item.total_uncompressed_bytes != item.total_uncompressed_bytes
        || source.source_receipt_id() != item.source_receipt_id
        || source.source_receipt_digest() != item.source_receipt_digest
        || source.admission_id() != item.admission_id
        || source.admission_digest() != item.admission_digest
        || source.adapter_id() != item.adapter_id
        || source.release_version() != item.release_version
        || source.artifact_sha256() != item.archive_sha256
        || source.artifact_size_bytes() != item.archive_size_bytes
        || !release_projection_is_exact(conn, &stored)?
    {
        bail!("Adapter registry release failed exact historical readback audit");
    }
    Ok(stored)
}

pub(super) fn audit_binding(
    conn: &Connection,
    stored: StoredRegistryProviderBinding,
) -> Result<StoredRegistryProviderBinding> {
    validate_registry_provider_binding_receipt(&stored.receipt)?;
    let (json, digest) =
        canonical_registry_provider_binding_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.binding;
    let release = release_by_id_on(conn, &item.registry_release_id)?
        .ok_or_else(|| anyhow::anyhow!("Adapter registry binding lost neutral release root"))?;
    let installation = external_pool_adapter_installation_receipt_authority_on(
        conn,
        &item.installation_receipt_id,
        &item.installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter registry binding lost V247 installation root"))?;
    let adoption = external_pool_adapter_adoption_receipt_authority_on(
        conn,
        &item.adoption_receipt_id,
        &item.adoption_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter registry binding lost V244 adoption root"))?;
    let install_receipt = installation.receipt();
    let install = &install_receipt.installation.binding;
    let adoption_receipt = adoption.receipt();
    let adopted = &adoption_receipt.adoption.binding;
    let expected_projection_id = route_adapter_projection_id(
        &item.provider_id,
        item.provider_policy_revision,
        &item.provider_digest,
        &item.registry_release_id,
        &item.registry_release_digest,
        &item.installation_receipt_id,
        &item.installation_receipt_digest,
    )?;
    if json != stored.receipt_json
        || digest != stored.receipt.provider_binding_digest
        || item.route_adapter_projection_id != expected_projection_id
        || release.receipt.registry_release_digest != item.registry_release_digest
        || release.receipt.release.installation_content_digest != item.installation_content_digest
        || release.receipt.release.admission_id != item.admission_id
        || release.receipt.release.admission_digest != item.admission_digest
        || release.receipt.release.package_receipt_id != item.package_receipt_id
        || release.receipt.release.package_receipt_digest != item.package_receipt_digest
        || release.receipt.release.package_material_digest != item.package_material_digest
        || release.receipt.release.source_receipt_id != item.source_receipt_id
        || release.receipt.release.source_receipt_digest != item.source_receipt_digest
        || release.receipt.release.adapter_id != item.adapter_id
        || release.receipt.release.release_version != item.release_version
        || install_receipt.installation_material_digest != item.installation_material_digest
        || install.installation_content_digest != item.installation_content_digest
        || install.application_id != item.application_id
        || install.application_digest != item.application_digest
        || install.adoption_receipt_id != item.adoption_receipt_id
        || install.adoption_receipt_digest != item.adoption_receipt_digest
        || install.adoption_material_digest != item.adoption_material_digest
        || install.provider_id != item.provider_id
        || install.provider_owner_account_id != item.provider_owner_account_id
        || install.provider_policy_revision != item.provider_policy_revision
        || install.provider_digest != item.provider_digest
        || install.adapter_id != item.adapter_id
        || install.adapter_release_version != item.release_version
        || install.adapter_config_revision != item.adapter_config_revision
        || install.adapter_config_digest != item.adapter_config_digest
        || install.admission_id != item.admission_id
        || install.admission_digest != item.admission_digest
        || install.package_receipt_id != item.package_receipt_id
        || install.package_receipt_digest != item.package_receipt_digest
        || install.package_material_digest != item.package_material_digest
        || install.source_receipt_id != item.source_receipt_id
        || install.source_receipt_digest != item.source_receipt_digest
        || install.credential_locator_commitment != item.credential_locator_commitment
        || adoption_receipt.adoption_material_digest != item.adoption_material_digest
        || adopted.application_id != item.application_id
        || adopted.application_digest != item.application_digest
        || adopted.provider_id != item.provider_id
        || adopted.provider_owner_account_id != item.provider_owner_account_id
        || adopted.provider_policy_revision != item.provider_policy_revision
        || adopted.provider_digest != item.provider_digest
        || adopted.admission_id != item.admission_id
        || adopted.admission_digest != item.admission_digest
        || adopted.adapter_id != item.adapter_id
        || adopted.adapter_release_version != item.release_version
        || adopted.adapter_config_revision != item.adapter_config_revision
        || adopted.adapter_config_digest != item.adapter_config_digest
        || adopted.sandbox_conformance_receipt_id != item.sandbox_conformance_receipt_id
        || adopted.sandbox_conformance_receipt_digest != item.sandbox_conformance_receipt_digest
        || adopted.credential_verification_receipt_id != item.credential_verification_receipt_id
        || adopted.credential_verification_receipt_digest
            != item.credential_verification_receipt_digest
        || adopted.credential_locator_commitment != item.credential_locator_commitment
        || !binding_projection_is_exact(conn, &stored)?
    {
        bail!("Adapter registry binding failed exact historical readback audit");
    }
    Ok(stored)
}

fn release_projection_is_exact(conn: &Connection, stored: &StoredRegistryRelease) -> Result<bool> {
    let receipt = &stored.receipt;
    let item = &receipt.release;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_registry_releases
          WHERE registry_release_id=?1 AND registry_release_schema=?2
            AND registry_release_digest=?3 AND receipt_json=?4
            AND registry_release_material_digest=?5 AND canonicalization=?6
            AND digest_algorithm=?7 AND admission_id=?8 AND admission_digest=?9
            AND package_receipt_id=?10 AND package_receipt_digest=?11
            AND package_material_digest=?12 AND source_receipt_id=?13
            AND source_receipt_digest=?14 AND installation_content_digest=?15
            AND adapter_id=?16 AND release_version=?17 AND route_kind=?18
            AND supported_provider_kinds_json=?19 AND implementation_digest=?20
            AND declared_implementation_sha256=?21 AND supported_capabilities_json=?22
            AND capability_set_digest=?23 AND credential_verifier_json=?24
            AND credential_verifier_digest=?25 AND archive_sha256=?26
            AND archive_size_bytes=?27 AND manifest_canonical_json=?28
            AND manifest_digest=?29 AND entry_inventory_digest=?30 AND entry_count=?31
            AND total_uncompressed_bytes=?32 AND registered_at=?33 AND recorded_at=?34
            AND registry_effect=?35 AND provider_effect=?36 AND credential_effect=?37
            AND route_effect=?38 AND execution_effect=?39 AND settlement_effect=?40",
            params![
                receipt.registry_release_id,
                receipt.schema,
                receipt.registry_release_digest,
                stored.receipt_json,
                receipt.registry_release_material_digest,
                receipt.canonicalization,
                receipt.digest_algorithm,
                item.admission_id,
                item.admission_digest,
                item.package_receipt_id,
                item.package_receipt_digest,
                item.package_material_digest,
                item.source_receipt_id,
                item.source_receipt_digest,
                item.installation_content_digest,
                item.adapter_id,
                item.release_version,
                item.route_kind,
                canonical_json(&item.supported_provider_kinds)?,
                item.implementation_digest,
                item.declared_implementation_sha256,
                canonical_json(&item.supported_capabilities)?,
                item.capability_set_digest,
                canonical_json(&item.credential_verifier)?,
                item.credential_verifier_digest,
                item.archive_sha256,
                i64::try_from(item.archive_size_bytes)?,
                canonical_json(&item.manifest)?,
                item.manifest_digest,
                item.entry_inventory_digest,
                i64::try_from(item.entry_count)?,
                i64::try_from(item.total_uncompressed_bytes)?,
                item.registered_at,
                item.recorded_at,
                item.registry_effect,
                item.provider_effect,
                item.credential_effect,
                item.route_effect,
                item.execution_effect,
                item.settlement_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn canonical_json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            value,
            1024 * 1024,
        )?
        .0,
    )
}

fn binding_projection_is_exact(
    conn: &Connection,
    stored: &StoredRegistryProviderBinding,
) -> Result<bool> {
    super::audit_binding_projection::binding_projection_is_exact(conn, stored)
}
