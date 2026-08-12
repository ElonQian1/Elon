use anyhow::Result;
use rusqlite::{params, Transaction};

use crate::compute_federation::external_pool_adapter_installation::ExternalPoolAdapterInstallationReceipt;

pub(super) fn insert_files(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterInstallationReceipt,
) -> Result<()> {
    let receipt_id = &receipt.installation_receipt_id;
    for (ordinal, file) in receipt
        .installation
        .binding
        .installed_files
        .iter()
        .enumerate()
    {
        tx.execute(
            "INSERT INTO compute_external_pool_adapter_installation_files(
                installation_receipt_id,ordinal,path,sha256,size_bytes,role
             ) VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                receipt_id,
                i64::try_from(ordinal)?,
                file.path,
                file.sha256,
                i64::try_from(file.size_bytes)?,
                file.role,
            ],
        )?;
    }
    Ok(())
}

pub(super) fn insert_receipt(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterInstallationReceipt,
    receipt_json: &str,
) -> Result<()> {
    let item = &receipt.installation;
    let binding = &item.binding;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_installation_receipts(
            installation_receipt_id,installation_receipt_digest,
            installation_receipt_schema,receipt_json,installation_material_digest,
            canonicalization,digest_algorithm,adoption_receipt_id,adoption_receipt_digest,
            adoption_material_digest,application_id,application_digest,provider_id,
            provider_owner_account_id,provider_policy_revision,provider_digest,
            admission_id,admission_digest,adapter_id,adapter_release_version,
            adapter_config_revision,adapter_config_digest,declared_implementation_sha256,
            capability_set_digest,credential_locator_commitment,package_receipt_id,
            package_receipt_digest,package_material_digest,source_receipt_id,
            source_receipt_digest,archive_sha256,archive_size_bytes,manifest_digest,
            entry_inventory_digest,entry_count,total_uncompressed_bytes,runtime_kind,
            entrypoint_path,entrypoint_sha256,entrypoint_size_bytes,
            installation_content_digest,storage_namespace,installed_by_admin_user_id,
            confirmation,idempotency_scope,idempotency_key,installed_at,recorded_at,
            installation_effect,credential_effect,provider_effect,route_effect,
            execution_effect,settlement_effect
        ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,
            ?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,
            ?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,?50,
            ?51,?52,?53,?54
        )",
        params![
            receipt.installation_receipt_id,
            receipt.installation_receipt_digest,
            receipt.schema,
            receipt_json,
            receipt.installation_material_digest,
            receipt.canonicalization,
            receipt.digest_algorithm,
            binding.adoption_receipt_id,
            binding.adoption_receipt_digest,
            binding.adoption_material_digest,
            binding.application_id,
            binding.application_digest,
            binding.provider_id,
            binding.provider_owner_account_id,
            binding.provider_policy_revision,
            binding.provider_digest,
            binding.admission_id,
            binding.admission_digest,
            binding.adapter_id,
            binding.adapter_release_version,
            binding.adapter_config_revision,
            binding.adapter_config_digest,
            binding.declared_implementation_sha256,
            binding.capability_set_digest,
            binding.credential_locator_commitment,
            binding.package_receipt_id,
            binding.package_receipt_digest,
            binding.package_material_digest,
            binding.source_receipt_id,
            binding.source_receipt_digest,
            binding.archive_sha256,
            i64::try_from(binding.archive_size_bytes)?,
            binding.manifest_digest,
            binding.entry_inventory_digest,
            i64::try_from(binding.entry_count)?,
            i64::try_from(binding.total_uncompressed_bytes)?,
            binding.runtime_kind,
            binding.entrypoint_path,
            binding.entrypoint_sha256,
            i64::try_from(binding.entrypoint_size_bytes)?,
            binding.installation_content_digest,
            binding.storage_namespace,
            item.installed_by_admin_user_id,
            item.confirmation,
            item.idempotency_scope,
            item.idempotency_key,
            item.installed_at,
            item.recorded_at,
            item.installation_effect,
            item.credential_effect,
            item.provider_effect,
            item.route_effect,
            item.execution_effect,
            item.settlement_effect,
        ],
    )?;
    Ok(())
}
