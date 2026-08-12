use anyhow::Result;
use rusqlite::{params, Transaction};

use crate::compute_federation::external_pool_adapter_registry::{
    canonical_registry_provider_binding_receipt_json_and_digest,
    canonical_registry_release_receipt_json_and_digest,
    ExternalPoolAdapterRegistryProviderBindingReceipt, ExternalPoolAdapterRegistryReleaseReceipt,
};

pub(super) fn insert_release(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRegistryReleaseReceipt,
) -> Result<()> {
    let item = &receipt.release;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_registry_releases(
            registry_release_id,registry_release_schema,registry_release_digest,receipt_json,
            registry_release_material_digest,canonicalization,digest_algorithm,admission_id,
            admission_digest,package_receipt_id,package_receipt_digest,package_material_digest,
            source_receipt_id,source_receipt_digest,installation_content_digest,adapter_id,
            release_version,route_kind,supported_provider_kinds_json,implementation_digest,
            declared_implementation_sha256,supported_capabilities_json,capability_set_digest,
            credential_verifier_json,credential_verifier_digest,archive_sha256,archive_size_bytes,
            manifest_canonical_json,manifest_digest,entry_inventory_digest,entry_count,
            total_uncompressed_bytes,registered_at,recorded_at,registry_effect,provider_effect,
            credential_effect,route_effect,execution_effect,settlement_effect
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,
            ?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,
            ?39,?40
         )",
        params![
            receipt.registry_release_id,
            receipt.schema,
            receipt.registry_release_digest,
            canonical_registry_release_receipt_json_and_digest(receipt)?.0,
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
    )?;
    Ok(())
}

pub(super) fn insert_binding(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRegistryProviderBindingReceipt,
) -> Result<()> {
    let item = &receipt.binding;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_registry_provider_bindings(
            provider_binding_id,provider_binding_schema,provider_binding_digest,receipt_json,
            provider_binding_material_digest,canonicalization,digest_algorithm,registry_release_id,
            registry_release_digest,route_adapter_projection_id,installation_receipt_id,
            installation_receipt_digest,installation_material_digest,installation_content_digest,
            application_id,application_digest,adoption_receipt_id,adoption_receipt_digest,
            adoption_material_digest,provider_id,provider_owner_account_id,provider_policy_revision,
            provider_digest,adapter_id,release_version,adapter_config_revision,adapter_config_digest,
            admission_id,admission_digest,package_receipt_id,package_receipt_digest,
            package_material_digest,source_receipt_id,source_receipt_digest,
            sandbox_conformance_receipt_id,sandbox_conformance_receipt_digest,
            credential_verification_receipt_id,credential_verification_receipt_digest,
            credential_locator_commitment,bound_by_admin_user_id,confirmation,checked_at,bound_at,
            recorded_at,idempotency_scope,idempotency_key,registry_effect,provider_effect,
            credential_effect,route_effect,execution_effect,settlement_effect
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,
            ?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,
            ?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,?50,?51,?52
         )",
        params![
            receipt.provider_binding_id, receipt.schema, receipt.provider_binding_digest,
            canonical_registry_provider_binding_receipt_json_and_digest(receipt)?.0,
            receipt.provider_binding_material_digest, receipt.canonicalization,
            receipt.digest_algorithm, item.registry_release_id, item.registry_release_digest,
            item.route_adapter_projection_id, item.installation_receipt_id,
            item.installation_receipt_digest, item.installation_material_digest,
            item.installation_content_digest, item.application_id, item.application_digest,
            item.adoption_receipt_id, item.adoption_receipt_digest, item.adoption_material_digest,
            item.provider_id, item.provider_owner_account_id, item.provider_policy_revision,
            item.provider_digest, item.adapter_id, item.release_version,
            item.adapter_config_revision, item.adapter_config_digest, item.admission_id,
            item.admission_digest, item.package_receipt_id, item.package_receipt_digest,
            item.package_material_digest, item.source_receipt_id, item.source_receipt_digest,
            item.sandbox_conformance_receipt_id, item.sandbox_conformance_receipt_digest,
            item.credential_verification_receipt_id, item.credential_verification_receipt_digest,
            item.credential_locator_commitment, item.bound_by_admin_user_id, item.confirmation,
            item.checked_at, item.bound_at, item.recorded_at, item.idempotency_scope,
            item.idempotency_key, item.registry_effect, item.provider_effect,
            item.credential_effect, item.route_effect, item.execution_effect, item.settlement_effect,
        ],
    )?;
    Ok(())
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
