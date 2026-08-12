use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use super::types::StoredRegistryProviderBinding;

pub(super) fn binding_projection_is_exact(
    conn: &Connection,
    stored: &StoredRegistryProviderBinding,
) -> Result<bool> {
    let receipt = &stored.receipt;
    let item = &receipt.binding;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_registry_provider_bindings
          WHERE provider_binding_id=?1 AND provider_binding_schema=?2
            AND provider_binding_digest=?3 AND receipt_json=?4
            AND provider_binding_material_digest=?5 AND canonicalization=?6
            AND digest_algorithm=?7 AND registry_release_id=?8
            AND registry_release_digest=?9 AND route_adapter_projection_id=?10
            AND installation_receipt_id=?11 AND installation_receipt_digest=?12
            AND installation_material_digest=?13 AND installation_content_digest=?14
            AND application_id=?15 AND application_digest=?16 AND adoption_receipt_id=?17
            AND adoption_receipt_digest=?18 AND adoption_material_digest=?19
            AND provider_id=?20 AND provider_owner_account_id=?21
            AND provider_policy_revision=?22 AND provider_digest=?23 AND adapter_id=?24
            AND release_version=?25 AND adapter_config_revision=?26
            AND adapter_config_digest=?27 AND admission_id=?28 AND admission_digest=?29
            AND package_receipt_id=?30 AND package_receipt_digest=?31
            AND package_material_digest=?32 AND source_receipt_id=?33
            AND source_receipt_digest=?34 AND sandbox_conformance_receipt_id=?35
            AND sandbox_conformance_receipt_digest=?36
            AND credential_verification_receipt_id=?37
            AND credential_verification_receipt_digest=?38
            AND credential_locator_commitment=?39 AND bound_by_admin_user_id=?40
            AND confirmation=?41 AND checked_at=?42 AND bound_at=?43 AND recorded_at=?44
            AND idempotency_scope=?45 AND idempotency_key=?46 AND registry_effect=?47
            AND provider_effect=?48 AND credential_effect=?49 AND route_effect=?50
            AND execution_effect=?51 AND settlement_effect=?52",
            params![
                receipt.provider_binding_id,
                receipt.schema,
                receipt.provider_binding_digest,
                stored.receipt_json,
                receipt.provider_binding_material_digest,
                receipt.canonicalization,
                receipt.digest_algorithm,
                item.registry_release_id,
                item.registry_release_digest,
                item.route_adapter_projection_id,
                item.installation_receipt_id,
                item.installation_receipt_digest,
                item.installation_material_digest,
                item.installation_content_digest,
                item.application_id,
                item.application_digest,
                item.adoption_receipt_id,
                item.adoption_receipt_digest,
                item.adoption_material_digest,
                item.provider_id,
                item.provider_owner_account_id,
                item.provider_policy_revision,
                item.provider_digest,
                item.adapter_id,
                item.release_version,
                item.adapter_config_revision,
                item.adapter_config_digest,
                item.admission_id,
                item.admission_digest,
                item.package_receipt_id,
                item.package_receipt_digest,
                item.package_material_digest,
                item.source_receipt_id,
                item.source_receipt_digest,
                item.sandbox_conformance_receipt_id,
                item.sandbox_conformance_receipt_digest,
                item.credential_verification_receipt_id,
                item.credential_verification_receipt_digest,
                item.credential_locator_commitment,
                item.bound_by_admin_user_id,
                item.confirmation,
                item.checked_at,
                item.bound_at,
                item.recorded_at,
                item.idempotency_scope,
                item.idempotency_key,
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
