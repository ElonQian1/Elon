use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Transaction, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_installation::{
        canonical_external_pool_adapter_installation_receipt_json_and_digest,
        installation_material_digest, validate_external_pool_adapter_installation_receipt,
        ExternalPoolAdapterInstallationMaterial, ExternalPoolAdapterInstallationReceipt,
        INSTALLATION_CANONICALIZATION, INSTALLATION_CONFIRMATION, INSTALLATION_DIGEST_ALGORITHM,
        INSTALLATION_EFFECT, INSTALLATION_NO_EFFECT, INSTALLATION_RECEIPT_SCHEMA,
    },
    store::{
        compute_external_pool_adapter_adoption::current_external_pool_adapter_adoption_authority_on,
        compute_external_pool_adapter_artifact_package::current_artifact_package_authority_on,
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        new_id, Store,
    },
};

use super::{persistence::*, read::*, types::*};

impl Store {
    pub(crate) fn install_external_pool_adapter(
        &self,
        input: InstallExternalPoolAdapter,
    ) -> Result<ExternalPoolAdapterInstallationWriteReceipt> {
        validate_input(&input)?;
        // The service hands Store a non-Clone proof returned by the filesystem
        // preparer/auditor. Retained file and directory handles keep that proof
        // pinned while this transaction rechecks every database authority root.
        let InstallExternalPoolAdapter {
            prepared,
            expected_adoption_receipt_digest,
            expected_package_receipt_digest,
            expected_source_receipt_digest,
            installed_by_admin_user_id,
            confirmation,
            idempotency_scope,
            idempotency_key,
        } = input;
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = receipt_by_idempotency_on(&tx, &idempotency_scope, &idempotency_key)?
        {
            ensure_replay(
                &stored,
                prepared.binding(),
                &expected_adoption_receipt_digest,
                &expected_package_receipt_digest,
                &expected_source_receipt_digest,
                &installed_by_admin_user_id,
                &confirmation,
                &idempotency_scope,
                &idempotency_key,
            )?;
            let output = write_receipt(&stored, true);
            tx.commit()?;
            return Ok(output);
        }
        if receipt_by_adoption_on(&tx, &prepared.binding().adoption_receipt_id)?.is_some() {
            bail!("exact Adapter adoption already has an installation receipt");
        }

        let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        exact_current_roots(
            &tx,
            prepared.binding(),
            &expected_adoption_receipt_digest,
            &expected_package_receipt_digest,
            &expected_source_receipt_digest,
            &checked_at,
        )?;
        let material = ExternalPoolAdapterInstallationMaterial {
            binding: prepared.binding().clone(),
            installed_by_admin_user_id,
            confirmation,
            idempotency_scope,
            idempotency_key,
            installed_at: checked_at.clone(),
            recorded_at: checked_at,
            installation_effect: INSTALLATION_EFFECT.to_string(),
            credential_effect: INSTALLATION_NO_EFFECT.to_string(),
            provider_effect: INSTALLATION_NO_EFFECT.to_string(),
            route_effect: INSTALLATION_NO_EFFECT.to_string(),
            execution_effect: INSTALLATION_NO_EFFECT.to_string(),
            settlement_effect: INSTALLATION_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterInstallationReceipt {
            schema: INSTALLATION_RECEIPT_SCHEMA.to_string(),
            installation_receipt_id: new_id("external_pool_adapter_installation"),
            installation_receipt_digest: String::new(),
            installation_material_digest: installation_material_digest(&material)?,
            canonicalization: INSTALLATION_CANONICALIZATION.to_string(),
            digest_algorithm: INSTALLATION_DIGEST_ALGORITHM.to_string(),
            installation: material,
        };
        receipt.installation_receipt_digest =
            canonical_external_pool_adapter_installation_receipt_json_and_digest(&receipt)?.1;
        validate_external_pool_adapter_installation_receipt(&receipt)?;
        let (json, digest) =
            canonical_external_pool_adapter_installation_receipt_json_and_digest(&receipt)?;
        if digest != receipt.installation_receipt_digest {
            bail!("Adapter installation digest changed before persistence");
        }
        insert_files(&tx, &receipt)?;
        insert_receipt(&tx, &receipt, &json)?;
        let stored = receipt_by_id_on(&tx, &receipt.installation_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("Adapter installation disappeared after insert"))?;
        if stored.receipt != receipt
            || stored.receipt_json != json
            || stored.files != receipt.installation.binding.installed_files
        {
            bail!("Adapter installation changed during exact readback");
        }
        let output = write_receipt(&stored, false);
        tx.commit()?;
        Ok(output)
    }
}

fn exact_current_roots(
    tx: &Transaction<'_>,
    prepared: &crate::compute_federation::external_pool_adapter_installation::ExternalPoolAdapterInstallationBinding,
    expected_adoption_receipt_digest: &str,
    expected_package_receipt_digest: &str,
    expected_source_receipt_digest: &str,
    checked_at: &str,
) -> Result<()> {
    let adoption = current_external_pool_adapter_adoption_authority_on(
        tx,
        &prepared.adoption_receipt_id,
        expected_adoption_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current Adapter adoption was not found"))?;
    let package = current_artifact_package_authority_on(
        tx,
        &prepared.admission_id,
        expected_package_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current Adapter package was not found"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        tx,
        &prepared.admission_id,
        &prepared.admission_digest,
        expected_source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("exact Adapter source was not found"))?;
    let adoption_receipt = adoption.receipt();
    let adoption_binding = &adoption_receipt.adoption.binding;
    let package_receipt = package.receipt();
    let package_item = &package_receipt.package;
    let manifest = &package_item.manifest;
    let manifest_files_are_exact = prepared.installed_files.len() == manifest.files.len()
        && prepared
            .installed_files
            .iter()
            .zip(&manifest.files)
            .all(|(installed, declared)| {
                installed.path == declared.path
                    && installed.sha256 == declared.sha256
                    && installed.size_bytes == declared.size_bytes
                    && installed.role == declared.role
            });
    let entrypoint = manifest
        .files
        .iter()
        .find(|file| file.path == manifest.runtime.entrypoint);
    if adoption.checked_at() != checked_at
        || adoption_receipt.adoption_receipt_id != prepared.adoption_receipt_id
        || adoption_receipt.adoption_receipt_digest != prepared.adoption_receipt_digest
        || adoption_receipt.adoption_material_digest != prepared.adoption_material_digest
        || adoption_binding.application_id != prepared.application_id
        || adoption_binding.application_digest != prepared.application_digest
        || adoption_binding.provider_id != prepared.provider_id
        || adoption_binding.provider_owner_account_id != prepared.provider_owner_account_id
        || adoption_binding.provider_policy_revision != prepared.provider_policy_revision
        || adoption_binding.provider_digest != prepared.provider_digest
        || adoption_binding.admission_id != prepared.admission_id
        || adoption_binding.admission_digest != prepared.admission_digest
        || adoption_binding.adapter_id != prepared.adapter_id
        || adoption_binding.adapter_release_version != prepared.adapter_release_version
        || adoption_binding.adapter_config_revision != prepared.adapter_config_revision
        || adoption_binding.adapter_config_digest != prepared.adapter_config_digest
        || adoption_binding.declared_implementation_sha256
            != prepared.declared_implementation_sha256
        || adoption_binding.capability_set_digest != prepared.capability_set_digest
        || adoption_binding.credential_locator_commitment != prepared.credential_locator_commitment
        || package_receipt.package_receipt_id != prepared.package_receipt_id
        || package_receipt.package_receipt_digest != prepared.package_receipt_digest
        || package_receipt.package_material_digest != prepared.package_material_digest
        || package_item.admission_id != prepared.admission_id
        || package_item.admission_digest != prepared.admission_digest
        || package_item.source_receipt_digest != prepared.source_receipt_digest
        || package_item.archive_sha256 != prepared.archive_sha256
        || package_item.archive_size_bytes != prepared.archive_size_bytes
        || package_item.manifest_digest != prepared.manifest_digest
        || package_item.entry_inventory_digest != prepared.entry_inventory_digest
        || package_item.entry_count != prepared.entry_count
        || package_item.total_uncompressed_bytes != prepared.total_uncompressed_bytes
        || manifest.adapter_id != prepared.adapter_id
        || manifest.release_version != prepared.adapter_release_version
        || manifest.capability_set_digest != prepared.capability_set_digest
        || manifest.runtime.kind != prepared.runtime_kind
        || manifest.runtime.entrypoint != prepared.entrypoint_path
        || !manifest_files_are_exact
        || entrypoint.map(|file| (&file.sha256, file.size_bytes))
            != Some((&prepared.entrypoint_sha256, prepared.entrypoint_size_bytes))
        || source.source_receipt_id() != prepared.source_receipt_id
        || source.source_receipt_digest() != prepared.source_receipt_digest
        || source.admission_id() != prepared.admission_id
        || source.admission_digest() != prepared.admission_digest
        || source.adapter_id() != prepared.adapter_id
        || source.release_version() != prepared.adapter_release_version
        || source.artifact_sha256() != prepared.archive_sha256
        || source.artifact_size_bytes() != prepared.archive_size_bytes
    {
        bail!("Adapter installation prepared evidence drifted from current exact roots");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredExternalPoolAdapterInstallation,
    prepared: &crate::compute_federation::external_pool_adapter_installation::ExternalPoolAdapterInstallationBinding,
    expected_adoption_receipt_digest: &str,
    expected_package_receipt_digest: &str,
    expected_source_receipt_digest: &str,
    installed_by_admin_user_id: &str,
    confirmation: &str,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<()> {
    let item = &stored.receipt.installation;
    if &item.binding != prepared
        || item.binding.adoption_receipt_digest != expected_adoption_receipt_digest
        || item.binding.package_receipt_digest != expected_package_receipt_digest
        || item.binding.source_receipt_digest != expected_source_receipt_digest
        || item.installed_by_admin_user_id != installed_by_admin_user_id
        || item.confirmation != confirmation
        || item.idempotency_scope != idempotency_scope
        || item.idempotency_key != idempotency_key
    {
        bail!("Adapter installation idempotency replay conflicts with immutable receipt");
    }
    Ok(())
}

fn validate_input(input: &InstallExternalPoolAdapter) -> Result<()> {
    for (label, value) in [
        (
            "expected adoption digest",
            &input.expected_adoption_receipt_digest,
        ),
        (
            "expected package digest",
            &input.expected_package_receipt_digest,
        ),
        (
            "expected source digest",
            &input.expected_source_receipt_digest,
        ),
    ] {
        if !is_sha256(value) {
            bail!("Adapter installation {label} is invalid");
        }
    }
    for (label, value, max) in [
        ("installer", &input.installed_by_admin_user_id, 200),
        ("idempotency scope", &input.idempotency_scope, 240),
        ("idempotency key", &input.idempotency_key, 240),
    ] {
        if value.is_empty() || value != value.trim() || value.len() > max {
            bail!("Adapter installation {label} is invalid");
        }
    }
    if input.confirmation != INSTALLATION_CONFIRMATION {
        bail!("Adapter installation confirmation is invalid");
    }
    Ok(())
}

fn write_receipt(
    stored: &StoredExternalPoolAdapterInstallation,
    replayed: bool,
) -> ExternalPoolAdapterInstallationWriteReceipt {
    ExternalPoolAdapterInstallationWriteReceipt {
        installation: stored.summary(),
        replayed,
    }
}
