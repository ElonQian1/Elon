use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    install_guard(
        conn,
        "external_pool_adapter_registry_release_json_projection",
        "compute_external_pool_adapter_registry_releases",
        "Provider-neutral registry release JSON projection mismatch",
        &release_projections(),
    )?;
    install_guard(
        conn,
        "external_pool_adapter_registry_provider_binding_json_projection",
        "compute_external_pool_adapter_registry_provider_bindings",
        "Registry Provider binding JSON projection mismatch",
        &binding_projections(),
    )?;
    Ok(())
}

fn install_guard(
    conn: &Connection,
    name: &str,
    table: &str,
    message: &str,
    projections: &[(&str, &str)],
) -> Result<()> {
    let mismatch = projections
        .iter()
        .map(|(path, column)| {
            format!("json_extract(NEW.receipt_json,'{path}') IS NOT NEW.{column}")
        })
        .collect::<Vec<_>>()
        .join("\n          OR ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS {name}\n         BEFORE INSERT ON {table}\n         WHEN {mismatch}\n         BEGIN SELECT RAISE(ABORT,'{message}'); END;"
    ))?;
    Ok(())
}

fn release_projections() -> Vec<(&'static str, &'static str)> {
    vec![
        ("$.schema", "registry_release_schema"),
        ("$.registry_release_id", "registry_release_id"),
        ("$.registry_release_digest", "registry_release_digest"),
        (
            "$.registry_release_material_digest",
            "registry_release_material_digest",
        ),
        ("$.canonicalization", "canonicalization"),
        ("$.digest_algorithm", "digest_algorithm"),
        ("$.release.admission_id", "admission_id"),
        ("$.release.admission_digest", "admission_digest"),
        ("$.release.package_receipt_id", "package_receipt_id"),
        ("$.release.package_receipt_digest", "package_receipt_digest"),
        (
            "$.release.package_material_digest",
            "package_material_digest",
        ),
        ("$.release.source_receipt_id", "source_receipt_id"),
        ("$.release.source_receipt_digest", "source_receipt_digest"),
        (
            "$.release.installation_content_digest",
            "installation_content_digest",
        ),
        ("$.release.adapter_id", "adapter_id"),
        ("$.release.release_version", "release_version"),
        ("$.release.route_kind", "route_kind"),
        (
            "$.release.supported_provider_kinds",
            "supported_provider_kinds_json",
        ),
        ("$.release.implementation_digest", "implementation_digest"),
        (
            "$.release.declared_implementation_sha256",
            "declared_implementation_sha256",
        ),
        (
            "$.release.supported_capabilities",
            "supported_capabilities_json",
        ),
        ("$.release.capability_set_digest", "capability_set_digest"),
        ("$.release.credential_verifier", "credential_verifier_json"),
        (
            "$.release.credential_verifier_digest",
            "credential_verifier_digest",
        ),
        ("$.release.archive_sha256", "archive_sha256"),
        ("$.release.archive_size_bytes", "archive_size_bytes"),
        ("$.release.manifest", "manifest_canonical_json"),
        ("$.release.manifest_digest", "manifest_digest"),
        ("$.release.entry_inventory_digest", "entry_inventory_digest"),
        ("$.release.entry_count", "entry_count"),
        (
            "$.release.total_uncompressed_bytes",
            "total_uncompressed_bytes",
        ),
        ("$.release.registered_at", "registered_at"),
        ("$.release.recorded_at", "recorded_at"),
        ("$.release.registry_effect", "registry_effect"),
        ("$.release.provider_effect", "provider_effect"),
        ("$.release.credential_effect", "credential_effect"),
        ("$.release.route_effect", "route_effect"),
        ("$.release.execution_effect", "execution_effect"),
        ("$.release.settlement_effect", "settlement_effect"),
    ]
}

fn binding_projections() -> Vec<(&'static str, &'static str)> {
    vec![
        ("$.schema", "provider_binding_schema"),
        ("$.provider_binding_id", "provider_binding_id"),
        ("$.provider_binding_digest", "provider_binding_digest"),
        (
            "$.provider_binding_material_digest",
            "provider_binding_material_digest",
        ),
        ("$.canonicalization", "canonicalization"),
        ("$.digest_algorithm", "digest_algorithm"),
        ("$.binding.registry_release_id", "registry_release_id"),
        (
            "$.binding.registry_release_digest",
            "registry_release_digest",
        ),
        (
            "$.binding.route_adapter_projection_id",
            "route_adapter_projection_id",
        ),
        (
            "$.binding.installation_receipt_id",
            "installation_receipt_id",
        ),
        (
            "$.binding.installation_receipt_digest",
            "installation_receipt_digest",
        ),
        (
            "$.binding.installation_material_digest",
            "installation_material_digest",
        ),
        (
            "$.binding.installation_content_digest",
            "installation_content_digest",
        ),
        ("$.binding.application_id", "application_id"),
        ("$.binding.application_digest", "application_digest"),
        ("$.binding.adoption_receipt_id", "adoption_receipt_id"),
        (
            "$.binding.adoption_receipt_digest",
            "adoption_receipt_digest",
        ),
        (
            "$.binding.adoption_material_digest",
            "adoption_material_digest",
        ),
        ("$.binding.provider_id", "provider_id"),
        (
            "$.binding.provider_owner_account_id",
            "provider_owner_account_id",
        ),
        (
            "$.binding.provider_policy_revision",
            "provider_policy_revision",
        ),
        ("$.binding.provider_digest", "provider_digest"),
        ("$.binding.adapter_id", "adapter_id"),
        ("$.binding.release_version", "release_version"),
        (
            "$.binding.adapter_config_revision",
            "adapter_config_revision",
        ),
        ("$.binding.adapter_config_digest", "adapter_config_digest"),
        ("$.binding.admission_id", "admission_id"),
        ("$.binding.admission_digest", "admission_digest"),
        ("$.binding.package_receipt_id", "package_receipt_id"),
        ("$.binding.package_receipt_digest", "package_receipt_digest"),
        (
            "$.binding.package_material_digest",
            "package_material_digest",
        ),
        ("$.binding.source_receipt_id", "source_receipt_id"),
        ("$.binding.source_receipt_digest", "source_receipt_digest"),
        (
            "$.binding.sandbox_conformance_receipt_id",
            "sandbox_conformance_receipt_id",
        ),
        (
            "$.binding.sandbox_conformance_receipt_digest",
            "sandbox_conformance_receipt_digest",
        ),
        (
            "$.binding.credential_verification_receipt_id",
            "credential_verification_receipt_id",
        ),
        (
            "$.binding.credential_verification_receipt_digest",
            "credential_verification_receipt_digest",
        ),
        (
            "$.binding.credential_locator_commitment",
            "credential_locator_commitment",
        ),
        ("$.binding.bound_by_admin_user_id", "bound_by_admin_user_id"),
        ("$.binding.confirmation", "confirmation"),
        ("$.binding.checked_at", "checked_at"),
        ("$.binding.bound_at", "bound_at"),
        ("$.binding.recorded_at", "recorded_at"),
        ("$.binding.idempotency_scope", "idempotency_scope"),
        ("$.binding.idempotency_key", "idempotency_key"),
        ("$.binding.registry_effect", "registry_effect"),
        ("$.binding.provider_effect", "provider_effect"),
        ("$.binding.credential_effect", "credential_effect"),
        ("$.binding.route_effect", "route_effect"),
        ("$.binding.execution_effect", "execution_effect"),
        ("$.binding.settlement_effect", "settlement_effect"),
    ]
}
