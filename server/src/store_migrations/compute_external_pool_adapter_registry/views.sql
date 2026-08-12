DROP VIEW IF EXISTS compute_external_pool_adapter_registry_provider_binding_current;
DROP VIEW IF EXISTS compute_external_pool_adapter_registry_release_current;

CREATE VIEW compute_external_pool_adapter_registry_release_current AS
SELECT release.registry_release_id,
       release.registry_release_digest,
       release.adapter_id,
       release.release_version,
       CASE WHEN admission.current_status='staged'
                  AND package.current_status='verified_current'
                  AND source.source_receipt_id IS NOT NULL
            THEN 'release_current' ELSE 'historical_only' END AS current_status,
       COALESCE(admission.current_status,'not_current') AS admission_status,
       COALESCE(package.current_status,'not_current') AS package_status,
       CASE WHEN source.source_receipt_id IS NULL THEN 'not_exact' ELSE 'exact' END AS source_status,
       release.registered_at
  FROM compute_external_pool_adapter_registry_releases release
  LEFT JOIN compute_external_pool_adapter_release_admission_current admission
    ON admission.admission_id=release.admission_id
   AND admission.admission_digest=release.admission_digest
  LEFT JOIN compute_external_pool_adapter_artifact_package_current package
    ON package.package_receipt_id=release.package_receipt_id
   AND package.package_receipt_digest=release.package_receipt_digest
  LEFT JOIN compute_external_pool_adapter_artifact_source_receipts source
    ON source.source_receipt_id=release.source_receipt_id
   AND source.source_receipt_digest=release.source_receipt_digest;

CREATE VIEW compute_external_pool_adapter_registry_provider_binding_current AS
SELECT 'compute_federation.external_pool_adapter_registry_provider_binding_currentness.v1'
         AS currentness_schema,
       binding.provider_binding_id,
       binding.provider_binding_digest,
       binding.registry_release_id,
       binding.registry_release_digest,
       binding.route_adapter_projection_id,
       binding.provider_id,
       binding.installation_receipt_id,
       binding.installation_receipt_digest,
       CASE WHEN release.current_status='release_current'
                  AND installation.installation_receipt_id IS NOT NULL
                  AND adoption.adoption_receipt_id IS NOT NULL
                  AND installation_terminal.terminal_receipt_id IS NULL
                  AND adoption_terminal.terminal_receipt_id IS NULL
                  AND provider_version.provider_id IS NOT NULL
                  AND route_adapter.adapter_id IS NULL
            THEN 'binding_current' ELSE 'historical_only' END AS current_status,
       COALESCE(release.current_status,'not_current') AS release_status,
       CASE WHEN installation_terminal.terminal_receipt_id IS NULL THEN 'none' ELSE 'revoked' END
         AS installation_terminal_status,
       CASE WHEN adoption_terminal.terminal_receipt_id IS NULL THEN 'none' ELSE 'revoked' END
         AS adoption_terminal_status,
       CASE WHEN provider_version.provider_id IS NULL THEN 'not_current' ELSE 'exact_registering' END
         AS provider_status,
       CASE WHEN route_adapter.adapter_id IS NULL THEN 'reserved' ELSE 'collided' END
         AS projection_status,
       binding.bound_at
  FROM compute_external_pool_adapter_registry_provider_bindings binding
  LEFT JOIN compute_external_pool_adapter_registry_release_current release
   ON release.registry_release_id=binding.registry_release_id
   AND release.registry_release_digest=binding.registry_release_digest
  LEFT JOIN compute_external_pool_adapter_installation_receipts installation
    ON installation.installation_receipt_id=binding.installation_receipt_id
   AND installation.installation_receipt_digest=binding.installation_receipt_digest
   AND installation.installation_material_digest=binding.installation_material_digest
   AND installation.installation_content_digest=binding.installation_content_digest
   AND installation.adoption_receipt_id=binding.adoption_receipt_id
   AND installation.adoption_receipt_digest=binding.adoption_receipt_digest
   AND installation.provider_id=binding.provider_id
   AND installation.provider_digest=binding.provider_digest
   AND installation.adapter_id=binding.adapter_id
   AND installation.adapter_release_version=binding.release_version
  LEFT JOIN compute_external_pool_adapter_adoption_receipts adoption
    ON adoption.adoption_receipt_id=binding.adoption_receipt_id
   AND adoption.adoption_receipt_digest=binding.adoption_receipt_digest
   AND adoption.adoption_material_digest=binding.adoption_material_digest
   AND adoption.application_id=binding.application_id
   AND adoption.application_digest=binding.application_digest
   AND adoption.provider_id=binding.provider_id
   AND adoption.provider_digest=binding.provider_digest
   AND adoption.adapter_id=binding.adapter_id
   AND adoption.adapter_release_version=binding.release_version
  LEFT JOIN compute_external_pool_adapter_installation_terminal_receipts installation_terminal
    ON installation_terminal.installation_receipt_id=binding.installation_receipt_id
   AND installation_terminal.installation_receipt_digest=binding.installation_receipt_digest
  LEFT JOIN compute_external_pool_adapter_adoption_terminal_receipts adoption_terminal
    ON adoption_terminal.adoption_receipt_id=binding.adoption_receipt_id
   AND adoption_terminal.adoption_receipt_digest=binding.adoption_receipt_digest
  LEFT JOIN compute_providers provider
    ON provider.provider_id=binding.provider_id
   AND provider.provider_kind='external_pool'
   AND provider.owner_account_id=binding.provider_owner_account_id
   AND provider.status='registering'
   AND provider.current_policy_revision=binding.provider_policy_revision
   AND provider.current_provider_digest=binding.provider_digest
  LEFT JOIN compute_provider_versions provider_version
    ON provider_version.provider_id=provider.provider_id
   AND provider_version.policy_revision=binding.provider_policy_revision
   AND provider_version.provider_digest=binding.provider_digest
  LEFT JOIN compute_route_adapters route_adapter
    ON route_adapter.adapter_id=binding.route_adapter_projection_id;
