use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_registry_release_exact_roots
        BEFORE INSERT ON compute_external_pool_adapter_registry_releases
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_adapter_release_admission_current current_admission
            JOIN compute_external_pool_adapter_release_admissions admission
              ON admission.admission_id=current_admission.admission_id
             AND admission.admission_digest=current_admission.admission_digest
            JOIN compute_external_pool_adapter_artifact_package_current current_package
              ON current_package.admission_id=admission.admission_id
             AND current_package.admission_digest=admission.admission_digest
            JOIN compute_external_pool_adapter_artifact_package_receipts package
              ON package.package_receipt_id=current_package.package_receipt_id
             AND package.package_receipt_digest=current_package.package_receipt_digest
            JOIN compute_external_pool_adapter_artifact_source_receipts source
              ON source.source_receipt_digest=package.source_receipt_digest
           WHERE current_admission.current_status='staged'
             AND current_package.current_status='verified_current'
             AND admission.admission_id=NEW.admission_id
             AND admission.admission_digest=NEW.admission_digest
             AND admission.adapter_id=NEW.adapter_id
             AND admission.release_version=NEW.release_version
             AND admission.route_kind=NEW.route_kind
             AND admission.supported_provider_kinds_json=NEW.supported_provider_kinds_json
             AND admission.declared_implementation_sha256=NEW.declared_implementation_sha256
             AND admission.capabilities_json=NEW.supported_capabilities_json
             AND admission.capability_set_digest=NEW.capability_set_digest
             AND admission.verifier_digest=NEW.credential_verifier_digest
             AND admission.verifier_verification_kind=json_extract(NEW.credential_verifier_json,'$.verification_kind')
             AND admission.verifier_id=json_extract(NEW.credential_verifier_json,'$.verifier_id')
             AND admission.verifier_revision=json_extract(NEW.credential_verifier_json,'$.verifier_revision')
             AND package.package_receipt_id=NEW.package_receipt_id
             AND package.package_receipt_digest=NEW.package_receipt_digest
             AND package.package_material_digest=NEW.package_material_digest
             AND package.admission_id=NEW.admission_id
             AND package.admission_digest=NEW.admission_digest
             AND package.archive_sha256=NEW.archive_sha256
             AND package.archive_size_bytes=NEW.archive_size_bytes
             AND package.manifest_canonical_json=NEW.manifest_canonical_json
             AND package.manifest_digest=NEW.manifest_digest
             AND package.entry_inventory_digest=NEW.entry_inventory_digest
             AND package.entry_count=NEW.entry_count
             AND package.total_uncompressed_bytes=NEW.total_uncompressed_bytes
             AND package.adapter_id=NEW.adapter_id
             AND package.release_version=NEW.release_version
             AND package.runtime_kind='server_sidecar_v1'
             AND package.supported_capabilities_json=NEW.supported_capabilities_json
             AND package.capability_set_digest=NEW.capability_set_digest
             AND package.credential_verifier_json=NEW.credential_verifier_json
             AND package.credential_verifier_digest=NEW.credential_verifier_digest
             AND source.source_receipt_id=NEW.source_receipt_id
             AND source.source_receipt_digest=NEW.source_receipt_digest
             AND source.admission_id=NEW.admission_id
             AND source.admission_digest=NEW.admission_digest
             AND source.adapter_id=NEW.adapter_id
             AND source.release_version=NEW.release_version
             AND source.declared_implementation_sha256=NEW.declared_implementation_sha256
             AND source.reopened_sha256=NEW.archive_sha256
             AND source.artifact_size_bytes=NEW.archive_size_bytes
             AND admission.applied_at<=NEW.registered_at
             AND package.inspected_at<=NEW.registered_at
             AND source.recorded_at<=NEW.registered_at
        )
        BEGIN SELECT RAISE(ABORT,'Provider-neutral registry release lacks exact current V222/V232/V227 roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_registry_provider_binding_exact_roots
        BEFORE INSERT ON compute_external_pool_adapter_registry_provider_bindings
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_adapter_registry_release_current current_release
            JOIN compute_external_pool_adapter_registry_releases release
              ON release.registry_release_id=current_release.registry_release_id
             AND release.registry_release_digest=current_release.registry_release_digest
            JOIN compute_external_pool_adapter_installation_current current_installation
              ON current_installation.installation_receipt_id=NEW.installation_receipt_id
             AND current_installation.installation_receipt_digest=NEW.installation_receipt_digest
            JOIN compute_external_pool_adapter_installation_receipts installation
              ON installation.installation_receipt_id=current_installation.installation_receipt_id
             AND installation.installation_receipt_digest=current_installation.installation_receipt_digest
            JOIN compute_external_pool_adapter_adoption_current current_adoption
              ON current_adoption.adoption_receipt_id=installation.adoption_receipt_id
             AND current_adoption.adoption_receipt_digest=installation.adoption_receipt_digest
            JOIN compute_external_pool_adapter_adoption_receipts adoption
              ON adoption.adoption_receipt_id=current_adoption.adoption_receipt_id
             AND adoption.adoption_receipt_digest=current_adoption.adoption_receipt_digest
            JOIN compute_providers provider ON provider.provider_id=installation.provider_id
            JOIN compute_provider_versions provider_version
              ON provider_version.provider_id=provider.provider_id
             AND provider_version.policy_revision=provider.current_policy_revision
             AND provider_version.provider_digest=provider.current_provider_digest
           WHERE current_release.current_status='release_current'
             AND current_installation.current_status='installed_upstreams_current'
             AND current_adoption.current_status='adopted_current'
             AND release.registry_release_id=NEW.registry_release_id
             AND release.registry_release_digest=NEW.registry_release_digest
             AND release.adapter_id=NEW.adapter_id
             AND release.release_version=NEW.release_version
             AND release.installation_content_digest=NEW.installation_content_digest
             AND release.admission_id=NEW.admission_id
             AND release.admission_digest=NEW.admission_digest
             AND release.package_receipt_id=NEW.package_receipt_id
             AND release.package_receipt_digest=NEW.package_receipt_digest
             AND release.package_material_digest=NEW.package_material_digest
             AND release.source_receipt_id=NEW.source_receipt_id
             AND release.source_receipt_digest=NEW.source_receipt_digest
             AND installation.installation_material_digest=NEW.installation_material_digest
             AND installation.installation_content_digest=NEW.installation_content_digest
             AND installation.application_id=NEW.application_id
             AND installation.application_digest=NEW.application_digest
             AND installation.adoption_receipt_id=NEW.adoption_receipt_id
             AND installation.adoption_receipt_digest=NEW.adoption_receipt_digest
             AND installation.adoption_material_digest=NEW.adoption_material_digest
             AND installation.provider_id=NEW.provider_id
             AND installation.provider_owner_account_id=NEW.provider_owner_account_id
             AND installation.provider_policy_revision=NEW.provider_policy_revision
             AND installation.provider_digest=NEW.provider_digest
             AND installation.adapter_id=NEW.adapter_id
             AND installation.adapter_release_version=NEW.release_version
             AND installation.adapter_config_revision=NEW.adapter_config_revision
             AND installation.adapter_config_digest=NEW.adapter_config_digest
             AND installation.admission_id=NEW.admission_id
             AND installation.admission_digest=NEW.admission_digest
             AND installation.package_receipt_id=NEW.package_receipt_id
             AND installation.package_receipt_digest=NEW.package_receipt_digest
             AND installation.package_material_digest=NEW.package_material_digest
             AND installation.source_receipt_id=NEW.source_receipt_id
             AND installation.source_receipt_digest=NEW.source_receipt_digest
             AND installation.installation_content_digest=release.installation_content_digest
             AND installation.installed_at<=NEW.checked_at
             AND adoption.application_id=NEW.application_id
             AND adoption.application_digest=NEW.application_digest
             AND adoption.adoption_material_digest=NEW.adoption_material_digest
             AND adoption.provider_id=NEW.provider_id
             AND adoption.provider_owner_account_id=NEW.provider_owner_account_id
             AND adoption.provider_policy_revision=NEW.provider_policy_revision
             AND adoption.provider_digest=NEW.provider_digest
             AND adoption.admission_id=NEW.admission_id
             AND adoption.admission_digest=NEW.admission_digest
             AND adoption.adapter_id=NEW.adapter_id
             AND adoption.adapter_release_version=NEW.release_version
             AND adoption.adapter_config_revision=NEW.adapter_config_revision
             AND adoption.adapter_config_digest=NEW.adapter_config_digest
             AND adoption.sandbox_conformance_receipt_id=NEW.sandbox_conformance_receipt_id
             AND adoption.sandbox_conformance_receipt_digest=NEW.sandbox_conformance_receipt_digest
             AND adoption.credential_verification_receipt_id=NEW.credential_verification_receipt_id
             AND adoption.credential_verification_receipt_digest=NEW.credential_verification_receipt_digest
             AND adoption.credential_locator_commitment=NEW.credential_locator_commitment
             AND provider.provider_kind='external_pool'
             AND provider.owner_account_id=NEW.provider_owner_account_id
             AND provider.status='registering'
             AND provider.current_policy_revision=NEW.provider_policy_revision
             AND provider.current_provider_digest=NEW.provider_digest
        )
        BEGIN SELECT RAISE(ABORT,'Registry Provider binding lacks exact current V247/V244/Provider roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_registry_provider_binding_projection_reserved
        BEFORE INSERT ON compute_external_pool_adapter_registry_provider_bindings
        WHEN EXISTS (
          SELECT 1 FROM compute_route_adapters route_adapter
           WHERE route_adapter.adapter_id=NEW.route_adapter_projection_id
        )
        BEGIN SELECT RAISE(ABORT,'Reserved route Adapter projection identity already exists'); END;
        "#,
    )?;
    Ok(())
}
