use std::collections::BTreeSet;

use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::external_pool_adapter_artifact_package::{
    ARTIFACT_PACKAGE_ENTRYPOINT_ROLE, ARTIFACT_PACKAGE_RESOURCE_ROLE, MAX_ARTIFACT_PACKAGE_ENTRIES,
    MAX_ARTIFACT_PACKAGE_ENTRY_BYTES, MAX_ARTIFACT_PACKAGE_UNCOMPRESSED_BYTES,
};

use super::{canonical::*, types::*};

pub(crate) fn validate_external_pool_adapter_installation_receipt(
    receipt: &ExternalPoolAdapterInstallationReceipt,
) -> Result<()> {
    if receipt.schema != INSTALLATION_RECEIPT_SCHEMA
        || receipt.canonicalization != INSTALLATION_CANONICALIZATION
        || receipt.digest_algorithm != INSTALLATION_DIGEST_ALGORITHM
    {
        bail!("Adapter installation receipt metadata is unsupported");
    }
    identifier(&receipt.installation_receipt_id, 200)?;
    digest(&receipt.installation_receipt_digest)?;
    digest(&receipt.installation_material_digest)?;
    let item = &receipt.installation;
    validate_external_pool_adapter_installation_binding(&item.binding)?;
    identifier(&item.installed_by_admin_user_id, 200)?;
    identifier(&item.idempotency_scope, 240)?;
    identifier(&item.idempotency_key, 240)?;
    canonical_nanos(&item.installed_at)?;
    canonical_nanos(&item.recorded_at)?;
    if item.installed_at != item.recorded_at
        || item.confirmation != INSTALLATION_CONFIRMATION
        || item.installation_effect != INSTALLATION_EFFECT
        || item.credential_effect != INSTALLATION_NO_EFFECT
        || item.provider_effect != INSTALLATION_NO_EFFECT
        || item.route_effect != INSTALLATION_NO_EFFECT
        || item.execution_effect != INSTALLATION_NO_EFFECT
        || item.settlement_effect != INSTALLATION_NO_EFFECT
        || installation_material_digest(item)? != receipt.installation_material_digest
        || canonical_external_pool_adapter_installation_receipt_json_and_digest(receipt)?.1
            != receipt.installation_receipt_digest
    {
        bail!("Adapter installation receipt material or effects are not exact");
    }
    Ok(())
}

pub(crate) fn validate_external_pool_adapter_installation_binding(
    binding: &ExternalPoolAdapterInstallationBinding,
) -> Result<()> {
    for value in [
        &binding.application_id,
        &binding.provider_id,
        &binding.provider_owner_account_id,
        &binding.admission_id,
        &binding.adapter_id,
        &binding.adapter_release_version,
        &binding.adoption_receipt_id,
        &binding.package_receipt_id,
        &binding.source_receipt_id,
        &binding.runtime_kind,
    ] {
        identifier(value, 200)?;
    }
    for value in [
        &binding.application_digest,
        &binding.provider_digest,
        &binding.admission_digest,
        &binding.declared_implementation_sha256,
        &binding.capability_set_digest,
        &binding.credential_locator_commitment,
        &binding.adoption_receipt_digest,
        &binding.adoption_material_digest,
        &binding.package_receipt_digest,
        &binding.package_material_digest,
        &binding.source_receipt_digest,
        &binding.archive_sha256,
        &binding.manifest_digest,
        &binding.entry_inventory_digest,
        &binding.entrypoint_sha256,
        &binding.installation_content_digest,
    ] {
        digest(value)?;
    }
    identifier(&binding.adapter_config_digest, 512)?;
    relative_path(&binding.entrypoint_path)?;
    if binding.provider_policy_revision < 1
        || binding.adapter_config_revision < 1
        || binding.archive_size_bytes == 0
        || binding.entrypoint_size_bytes == 0
        || binding.entry_count == 0
        || binding.entry_count as usize != binding.installed_files.len()
        || binding.installed_files.len() > MAX_ARTIFACT_PACKAGE_ENTRIES
        || binding.total_uncompressed_bytes == 0
        || binding.total_uncompressed_bytes > MAX_ARTIFACT_PACKAGE_UNCOMPRESSED_BYTES
        || binding.storage_namespace != INSTALLATION_STORAGE_NAMESPACE
        || binding.declared_implementation_sha256 != binding.archive_sha256
    {
        bail!("Adapter installation binding bounds or namespace are invalid");
    }

    let mut paths = BTreeSet::new();
    let mut previous: Option<&str> = None;
    let mut entrypoint_count = 0;
    let mut installed_bytes = 0_u64;
    for file in &binding.installed_files {
        relative_path(&file.path)?;
        digest(&file.sha256)?;
        if file.size_bytes == 0
            || file.size_bytes > MAX_ARTIFACT_PACKAGE_ENTRY_BYTES
            || !matches!(
                file.role.as_str(),
                ARTIFACT_PACKAGE_ENTRYPOINT_ROLE | ARTIFACT_PACKAGE_RESOURCE_ROLE
            )
            || previous.is_some_and(|value| value >= file.path.as_str())
            || !paths.insert(file.path.to_ascii_lowercase())
        {
            bail!("Adapter installation file inventory is not exact and canonical");
        }
        if file.role == ARTIFACT_PACKAGE_ENTRYPOINT_ROLE {
            entrypoint_count += 1;
            if file.path != binding.entrypoint_path
                || file.sha256 != binding.entrypoint_sha256
                || file.size_bytes != binding.entrypoint_size_bytes
            {
                bail!("Adapter installation entrypoint does not match inventory");
            }
        }
        installed_bytes = installed_bytes
            .checked_add(file.size_bytes)
            .ok_or_else(|| anyhow::anyhow!("Adapter installation byte total overflow"))?;
        previous = Some(&file.path);
    }
    if entrypoint_count != 1
        || installed_bytes >= binding.total_uncompressed_bytes
        || binding_content_digest(binding)? != binding.installation_content_digest
    {
        bail!("Adapter installation content commitment is invalid");
    }
    Ok(())
}

pub(super) fn identifier(value: &str, maximum: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        bail!("Adapter installation identifier is invalid");
    }
    Ok(())
}

pub(super) fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("Adapter installation digest is invalid");
    }
    Ok(())
}

pub(super) fn relative_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 160
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        bail!("Adapter installation path is not canonical relative form");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("Adapter installation timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
