use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_registry::REGISTRY_PROVIDER_BINDING_CURRENTNESS_SCHEMA,
        provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_REGISTERING},
    },
    store::{
        compute_external_pool_adapter_adoption::{
            external_pool_adapter_adoption_is_revoked_on,
            external_pool_adapter_adoption_receipt_authority_on,
        },
        compute_external_pool_adapter_artifact_package::{
            artifact_package_is_current_exact_on, current_artifact_package_authority_on,
        },
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        compute_external_pool_adapter_installation::{
            external_pool_adapter_installation_is_revoked_on,
            external_pool_adapter_installation_receipt_authority_on,
        },
        compute_external_pool_adapter_release_lifecycle::{
            current_external_pool_adapter_release_admission_authority_on,
            external_pool_adapter_release_admission_is_current_exact_on,
        },
        compute_provider_registry::current_registered_provider_on,
        Store,
    },
};

use super::{read::*, types::*};

pub(in crate::store) fn current_external_pool_adapter_registry_release_authority_on(
    conn: &Connection,
    registry_release_id: &str,
    expected_registry_release_digest: &str,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterRegistryReleaseAuthority>> {
    let Some(release) = release_by_id_on(conn, registry_release_id)? else {
        return Ok(None);
    };
    let item = &release.receipt.release;
    validate_release_checked_at(checked_at, &item.registered_at)?;
    let admission = current_external_pool_adapter_release_admission_authority_on(
        conn,
        &item.admission_id,
        &item.admission_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry release lost current admission"))?;
    let package = current_artifact_package_authority_on(
        conn,
        &item.admission_id,
        &item.package_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry release lost current package"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        conn,
        &item.admission_id,
        &item.admission_digest,
        &item.source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry release lost exact source"))?;
    let package_receipt = package.receipt();
    let package_item = &package_receipt.package;
    if release.receipt.registry_release_digest != expected_registry_release_digest
        || admission.admission_id() != item.admission_id
        || admission.admission_digest() != item.admission_digest
        || admission.adapter_id() != item.adapter_id
        || admission.release_version() != item.release_version
        || package_receipt.package_receipt_id != item.package_receipt_id
        || package_receipt.package_receipt_digest != item.package_receipt_digest
        || package_item.source_receipt_digest != item.source_receipt_digest
        || package_item.archive_sha256 != item.archive_sha256
        || package_item.manifest_digest != item.manifest_digest
        || source.source_receipt_id() != item.source_receipt_id
        || source.source_receipt_digest() != item.source_receipt_digest
        || source.artifact_sha256() != item.archive_sha256
    {
        bail!("Adapter registry release is not current and exact");
    }
    Ok(Some(
        CurrentExternalPoolAdapterRegistryReleaseAuthority::new(
            release.receipt,
            checked_at.to_string(),
        ),
    ))
}

pub(in crate::store) fn external_pool_adapter_registry_release_is_current_exact_on(
    conn: &Connection,
    registry_release_id: &str,
    expected_registry_release_digest: &str,
    checked_at: &str,
) -> Result<bool> {
    let Some(release) = release_by_id_on(conn, registry_release_id)? else {
        return Ok(false);
    };
    let item = &release.receipt.release;
    validate_release_checked_at(checked_at, &item.registered_at)?;
    if release.receipt.registry_release_digest != expected_registry_release_digest
        || !external_pool_adapter_release_admission_is_current_exact_on(
            conn,
            &item.admission_id,
            &item.admission_digest,
        )?
        || !artifact_package_is_current_exact_on(
            conn,
            &item.admission_id,
            &item.package_receipt_digest,
        )?
    {
        return Ok(false);
    }
    let Some(source) = external_pool_adapter_artifact_source_authority_on(
        conn,
        &item.admission_id,
        &item.admission_digest,
        &item.source_receipt_digest,
    )?
    else {
        return Ok(false);
    };
    Ok(source.source_receipt_id() == item.source_receipt_id
        && source.source_receipt_digest() == item.source_receipt_digest
        && source.artifact_sha256() == item.archive_sha256)
}

fn validate_release_checked_at(checked_at: &str, registered_at: &str) -> Result<()> {
    let checked = DateTime::parse_from_rfc3339(checked_at)?;
    let registered = DateTime::parse_from_rfc3339(registered_at)?;
    if checked.offset().local_minus_utc() != 0
        || checked.to_rfc3339_opts(SecondsFormat::Nanos, true) != checked_at
        || registered.offset().local_minus_utc() != 0
        || registered.to_rfc3339_opts(SecondsFormat::Nanos, true) != registered_at
        || checked < registered
        || checked > Utc::now() + chrono::Duration::minutes(5)
    {
        bail!("Adapter registry release checked_at is not a current canonical observation");
    }
    Ok(())
}

pub(in crate::store) fn current_external_pool_adapter_registry_provider_binding_authority_on(
    conn: &Connection,
    provider_binding_id: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterRegistryProviderBindingAuthority>> {
    let Some(binding) = binding_by_id_on(conn, provider_binding_id)? else {
        return Ok(None);
    };
    let item = &binding.receipt.binding;
    validate_checked_at(checked_at, &item.checked_at, &item.bound_at)?;
    let release = release_by_id_on(conn, &item.registry_release_id)?
        .ok_or_else(|| anyhow::anyhow!("registry binding lost release root"))?;
    let installation = external_pool_adapter_installation_receipt_authority_on(
        conn,
        &item.installation_receipt_id,
        &item.installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry binding lost installation root"))?;
    let installation_receipt = installation.receipt();
    let installation_binding = &installation_receipt.installation.binding;
    if prepared.binding() != installation_binding
        || prepared.installation_content_digest() != item.installation_content_digest
        || external_pool_adapter_installation_is_revoked_on(conn, &item.installation_receipt_id)?
        || external_pool_adapter_adoption_is_revoked_on(conn, &item.adoption_receipt_id)?
    {
        bail!("registry Provider binding is terminal or filesystem-inexact");
    }
    let adoption = external_pool_adapter_adoption_receipt_authority_on(
        conn,
        &item.adoption_receipt_id,
        &item.adoption_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry binding lost adoption history"))?;
    let admission = current_external_pool_adapter_release_admission_authority_on(
        conn,
        &item.admission_id,
        &item.admission_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry binding lost current admission"))?;
    let package = current_artifact_package_authority_on(
        conn,
        &item.admission_id,
        &item.package_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry binding lost current package"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        conn,
        &item.admission_id,
        &item.admission_digest,
        &item.source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry binding lost exact source"))?;
    let provider = current_registered_provider_on(conn, &item.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("registry binding lost Provider"))?;
    let provider_item = &provider.provider;
    let adapter = provider_item.adapter.as_ref();
    let release_item = &release.receipt.release;
    let adoption_receipt = adoption.receipt();
    let adoption_item = &adoption_receipt.adoption.binding;
    let package_receipt = package.receipt();
    let package_item = &package_receipt.package;
    let route_collision: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM compute_route_adapters WHERE adapter_id=?1",
            params![item.route_adapter_projection_id],
            |r| r.get(0),
        )
        .optional()?;
    if route_collision.is_some()
        || release.receipt.registry_release_id != item.registry_release_id
        || release.receipt.registry_release_digest != item.registry_release_digest
        || release_item.installation_content_digest != item.installation_content_digest
        || release_item.admission_id != item.admission_id
        || release_item.admission_digest != item.admission_digest
        || release_item.package_receipt_id != item.package_receipt_id
        || release_item.package_receipt_digest != item.package_receipt_digest
        || release_item.package_material_digest != item.package_material_digest
        || release_item.source_receipt_id != item.source_receipt_id
        || release_item.source_receipt_digest != item.source_receipt_digest
        || release_item.adapter_id != item.adapter_id
        || release_item.release_version != item.release_version
        || release_item.implementation_digest != installation_binding.archive_sha256
        || release_item.declared_implementation_sha256
            != installation_binding.declared_implementation_sha256
        || release_item.capability_set_digest != installation_binding.capability_set_digest
        || release_item.archive_sha256 != installation_binding.archive_sha256
        || release_item.archive_size_bytes != installation_binding.archive_size_bytes
        || release_item.manifest_digest != installation_binding.manifest_digest
        || release_item.entry_inventory_digest != installation_binding.entry_inventory_digest
        || release_item.entry_count != installation_binding.entry_count
        || release_item.total_uncompressed_bytes != installation_binding.total_uncompressed_bytes
        || installation_receipt.installation_receipt_id != item.installation_receipt_id
        || installation_receipt.installation_receipt_digest != item.installation_receipt_digest
        || installation_receipt.installation_material_digest != item.installation_material_digest
        || installation_binding.installation_content_digest != item.installation_content_digest
        || installation_binding.application_id != item.application_id
        || installation_binding.application_digest != item.application_digest
        || installation_binding.provider_id != item.provider_id
        || installation_binding.provider_owner_account_id != item.provider_owner_account_id
        || installation_binding.provider_policy_revision != item.provider_policy_revision
        || installation_binding.provider_digest != item.provider_digest
        || installation_binding.adoption_receipt_id != item.adoption_receipt_id
        || installation_binding.adoption_receipt_digest != item.adoption_receipt_digest
        || installation_binding.adoption_material_digest != item.adoption_material_digest
        || installation_binding.adapter_id != item.adapter_id
        || installation_binding.adapter_release_version != item.release_version
        || installation_binding.adapter_config_revision != item.adapter_config_revision
        || installation_binding.adapter_config_digest != item.adapter_config_digest
        || installation_binding.admission_id != item.admission_id
        || installation_binding.admission_digest != item.admission_digest
        || installation_binding.package_receipt_id != item.package_receipt_id
        || installation_binding.package_receipt_digest != item.package_receipt_digest
        || installation_binding.package_material_digest != item.package_material_digest
        || installation_binding.source_receipt_id != item.source_receipt_id
        || installation_binding.source_receipt_digest != item.source_receipt_digest
        || installation_binding.credential_locator_commitment != item.credential_locator_commitment
        || adoption_receipt.adoption_receipt_id != item.adoption_receipt_id
        || adoption_receipt.adoption_receipt_digest != item.adoption_receipt_digest
        || adoption_receipt.adoption_material_digest != item.adoption_material_digest
        || adoption_item.application_id != item.application_id
        || adoption_item.application_digest != item.application_digest
        || adoption_item.provider_id != item.provider_id
        || adoption_item.provider_owner_account_id != item.provider_owner_account_id
        || adoption_item.provider_policy_revision != item.provider_policy_revision
        || adoption_item.provider_digest != item.provider_digest
        || adoption_item.admission_id != item.admission_id
        || adoption_item.admission_digest != item.admission_digest
        || adoption_item.adapter_id != item.adapter_id
        || adoption_item.adapter_release_version != item.release_version
        || adoption_item.adapter_config_revision != item.adapter_config_revision
        || adoption_item.adapter_config_digest != item.adapter_config_digest
        || adoption_item.declared_implementation_sha256 != release_item.implementation_digest
        || adoption_item.capability_set_digest != release_item.capability_set_digest
        || adoption_item.sandbox_conformance_receipt_id != item.sandbox_conformance_receipt_id
        || adoption_item.sandbox_conformance_receipt_digest
            != item.sandbox_conformance_receipt_digest
        || adoption_item.credential_verification_receipt_id
            != item.credential_verification_receipt_id
        || adoption_item.credential_verification_receipt_digest
            != item.credential_verification_receipt_digest
        || adoption_item.credential_locator_commitment != item.credential_locator_commitment
        || admission.admission_id() != item.admission_id
        || admission.admission_digest() != item.admission_digest
        || admission.adapter_id() != item.adapter_id
        || admission.release_version() != item.release_version
        || admission.declared_implementation_sha256() != release_item.implementation_digest
        || admission.supported_capabilities() != release_item.supported_capabilities.as_slice()
        || admission.capability_set_digest() != release_item.capability_set_digest
        || admission.expected_credential_verifier() != &release_item.credential_verifier
        || package_receipt.package_receipt_id != item.package_receipt_id
        || package_receipt.package_receipt_digest != item.package_receipt_digest
        || package_receipt.package_material_digest != item.package_material_digest
        || package_item.admission_id != item.admission_id
        || package_item.admission_digest != item.admission_digest
        || package_item.source_receipt_digest != item.source_receipt_digest
        || package_item.archive_sha256 != release_item.archive_sha256
        || package_item.archive_size_bytes != release_item.archive_size_bytes
        || package_item.manifest != release_item.manifest
        || package_item.manifest_digest != release_item.manifest_digest
        || package_item.entry_inventory_digest != release_item.entry_inventory_digest
        || package_item.entry_count != release_item.entry_count
        || package_item.total_uncompressed_bytes != release_item.total_uncompressed_bytes
        || source.source_receipt_id() != item.source_receipt_id
        || source.source_receipt_digest() != item.source_receipt_digest
        || source.admission_id() != item.admission_id
        || source.admission_digest() != item.admission_digest
        || source.adapter_id() != item.adapter_id
        || source.release_version() != item.release_version
        || source.artifact_sha256() != release_item.archive_sha256
        || source.artifact_size_bytes() != release_item.archive_size_bytes
        || provider_item.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || provider_item.status != PROVIDER_STATUS_REGISTERING
        || provider_item.owner_account_id != item.provider_owner_account_id
        || provider_item.policy_revision != item.provider_policy_revision
        || provider.provider_digest != item.provider_digest
        || adapter.map(|x| x.adapter_id.as_str()) != Some(item.adapter_id.as_str())
        || adapter.map(|x| x.adapter_version.as_str()) != Some(item.release_version.as_str())
        || adapter.map(|x| x.config_revision) != Some(item.adapter_config_revision)
        || adapter.map(|x| x.config_digest.as_str()) != Some(item.adapter_config_digest.as_str())
    {
        bail!("registry Provider binding current roots drifted");
    }
    Ok(Some(
        CurrentExternalPoolAdapterRegistryProviderBindingAuthority::new(
            release.receipt,
            binding.receipt,
            prepared,
            checked_at.to_string(),
        ),
    ))
}

fn validate_checked_at(
    checked_at: &str,
    registered_checked_at: &str,
    bound_at: &str,
) -> Result<()> {
    let checked = DateTime::parse_from_rfc3339(checked_at)?;
    let registered = DateTime::parse_from_rfc3339(registered_checked_at)?;
    let bound = DateTime::parse_from_rfc3339(bound_at)?;
    if checked.offset().local_minus_utc() != 0
        || checked.to_rfc3339_opts(SecondsFormat::Nanos, true) != checked_at
        || registered.offset().local_minus_utc() != 0
        || registered.to_rfc3339_opts(SecondsFormat::Nanos, true) != registered_checked_at
        || bound.offset().local_minus_utc() != 0
        || bound.to_rfc3339_opts(SecondsFormat::Nanos, true) != bound_at
        || checked < registered
        || checked < bound
    {
        bail!("registry Provider binding checked_at is not a current canonical observation");
    }
    Ok(())
}

impl Store {
    pub(crate) fn external_pool_adapter_registry_provider_binding_currentness(
        &self,
        provider_binding_id: &str,
        prepared: PreparedExternalPoolAdapterInstallation,
    ) -> Result<Option<ExternalPoolAdapterRegistryProviderBindingCurrentness>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let Some(authority) = current_external_pool_adapter_registry_provider_binding_authority_on(
            &tx,
            provider_binding_id,
            prepared,
            &checked_at,
        )?
        else {
            tx.commit()?;
            return Ok(None);
        };
        let release = StoredRegistryRelease {
            receipt: authority.release().clone(),
            receipt_json: String::new(),
        }
        .summary();
        let binding = StoredRegistryProviderBinding {
            receipt: authority.binding().clone(),
            receipt_json: String::new(),
        }
        .summary();
        let currentness = ExternalPoolAdapterRegistryProviderBindingCurrentness {
            schema: REGISTRY_PROVIDER_BINDING_CURRENTNESS_SCHEMA,
            release,
            binding,
            current_status: "binding_current".into(),
            release_status: "release_current".into(),
            admission_status: "staged".into(),
            package_status: "verified_current".into(),
            source_status: "exact".into(),
            adoption_terminal_status: "none".into(),
            installation_terminal_status: "none".into(),
            provider_status: "exact_registering".into(),
            file_inventory_status: "reopened_rehashed_exact".into(),
            route_projection_status: "reserved".into(),
        };
        tx.commit()?;
        Ok(Some(currentness))
    }
}
