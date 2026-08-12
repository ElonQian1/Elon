use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_REGISTERING},
    },
    store::{
        compute_external_pool_adapter_adoption::current_external_pool_adapter_adoption_authority_on,
        compute_external_pool_adapter_artifact_package::current_artifact_package_authority_on,
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        compute_provider_registry::current_registered_provider_on,
    },
};

use super::{
    read::receipt_by_id_on,
    terminal::terminal_by_installation_on,
    types::{
        CurrentExternalPoolAdapterInstallationAuthority,
        HistoricalExternalPoolAdapterInstallationAuthority,
    },
};

pub(in crate::store) fn current_external_pool_adapter_installation_authority_on(
    conn: &Connection,
    receipt_id: &str,
    expected_receipt_digest: &str,
    checked_at: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
) -> Result<Option<CurrentExternalPoolAdapterInstallationAuthority>> {
    validate_checked_at(checked_at)?;
    let Some(stored) = receipt_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    let receipt = &stored.receipt;
    let binding = &receipt.installation.binding;
    if receipt.installation_receipt_digest != expected_receipt_digest
        || prepared.binding() != binding
        || checked_at < receipt.installation.installed_at.as_str()
        || terminal_by_installation_on(conn, receipt_id)?.is_some()
    {
        bail!("Adapter installation authority is not current and exact");
    }

    let adoption = current_external_pool_adapter_adoption_authority_on(
        conn,
        &binding.adoption_receipt_id,
        &binding.adoption_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter installation lost current adoption authority"))?;
    let package = current_artifact_package_authority_on(
        conn,
        &binding.admission_id,
        &binding.package_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter installation lost current package authority"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        conn,
        &binding.admission_id,
        &binding.admission_digest,
        &binding.source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter installation lost exact source authority"))?;
    let provider = current_registered_provider_on(conn, &binding.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("Adapter installation lost Provider authority"))?;
    let provider_item = &provider.provider;
    let adapter = provider_item.adapter.as_ref();
    if adoption.checked_at() != checked_at
        || adoption.receipt().adoption_receipt_id != binding.adoption_receipt_id
        || adoption.receipt().adoption_receipt_digest != binding.adoption_receipt_digest
        || adoption.receipt().adoption_material_digest != binding.adoption_material_digest
        || package.receipt().package_receipt_id != binding.package_receipt_id
        || package.receipt().package_receipt_digest != binding.package_receipt_digest
        || package.receipt().package_material_digest != binding.package_material_digest
        || source.source_receipt_id() != binding.source_receipt_id
        || source.source_receipt_digest() != binding.source_receipt_digest
        || source.admission_id() != binding.admission_id
        || source.admission_digest() != binding.admission_digest
        || source.artifact_sha256() != binding.archive_sha256
        || source.artifact_size_bytes() != binding.archive_size_bytes
        || provider_item.provider_id != binding.provider_id
        || provider_item.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || provider_item.owner_account_id != binding.provider_owner_account_id
        || provider_item.policy_revision != binding.provider_policy_revision
        || provider_item.status != PROVIDER_STATUS_REGISTERING
        || provider.provider_digest != binding.provider_digest
        || adapter.map(|value| value.adapter_id.as_str()) != Some(binding.adapter_id.as_str())
        || adapter.map(|value| value.adapter_version.as_str())
            != Some(binding.adapter_release_version.as_str())
        || adapter.map(|value| value.config_revision) != Some(binding.adapter_config_revision)
        || adapter.map(|value| value.config_digest.as_str())
            != Some(binding.adapter_config_digest.as_str())
    {
        bail!("Adapter installation current authority roots drifted");
    }

    Ok(Some(CurrentExternalPoolAdapterInstallationAuthority::new(
        stored.receipt,
        prepared,
        checked_at.to_string(),
    )))
}

pub(in crate::store) fn external_pool_adapter_installation_receipt_authority_on(
    conn: &Connection,
    receipt_id: &str,
    expected_receipt_digest: &str,
) -> Result<Option<HistoricalExternalPoolAdapterInstallationAuthority>> {
    let Some(stored) = receipt_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    if stored.receipt.installation_receipt_digest != expected_receipt_digest {
        bail!("Adapter installation receipt authority is not exact");
    }
    Ok(Some(
        HistoricalExternalPoolAdapterInstallationAuthority::new(stored.receipt),
    ))
}

fn validate_checked_at(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("Adapter installation checked_at is not canonical UTC nanoseconds");
    }
    Ok(())
}
