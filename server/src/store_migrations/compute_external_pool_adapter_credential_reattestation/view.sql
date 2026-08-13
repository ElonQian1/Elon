DROP VIEW IF EXISTS compute_external_pool_adapter_credential_reattestation_current;

CREATE VIEW compute_external_pool_adapter_credential_reattestation_current AS
WITH heads AS (
  SELECT receipt.*
    FROM compute_external_pool_adapter_credential_reattestation_receipts receipt
   WHERE NOT EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts successor
      WHERE successor.predecessor_receipt_id=receipt.reattestation_receipt_id)
), display AS (
  SELECT head.*,
         binding.provider_binding_id AS exact_binding_id,
         release.current_status AS release_current_status,
         verifier.current_status AS verifier_current_status,
         provider.status AS live_provider_status,
         provider.current_policy_revision AS live_provider_policy_revision,
         provider.current_provider_digest AS live_provider_digest,
         provider.owner_account_id AS live_provider_owner_account_id,
         version.provider_json AS live_provider_json,
         CASE
           WHEN provider.provider_id IS NULL THEN 'drifted'
           WHEN provider.provider_kind IS NOT 'external_pool'
             OR provider.owner_account_id IS NOT head.provider_owner_account_id
             OR json_extract(version.provider_json,'$.provider_id') IS NOT head.provider_id
             OR json_extract(version.provider_json,'$.provider_kind') IS NOT 'external_pool'
             OR json_extract(version.provider_json,'$.owner_account_id') IS NOT head.provider_owner_account_id
             OR json_extract(version.provider_json,'$.created_at') IS NOT json_extract(onboarding.target_provider_jcs,'$.created_at')
             OR json_extract(version.provider_json,'$.adapter.adapter_id') IS NOT head.adapter_id
             OR json_extract(version.provider_json,'$.adapter.adapter_version') IS NOT head.release_version
             OR json_extract(version.provider_json,'$.adapter.config_revision') IS NOT head.adapter_config_revision
             OR json_extract(version.provider_json,'$.adapter.config_digest') IS NOT head.adapter_config_digest
             THEN 'drifted'
           ELSE 'subject_exact'
         END AS subject_status,
         CASE
           WHEN provider.provider_id IS NULL THEN 'historical_only'
           WHEN head.observed_provider_status='registering'
             AND provider.status='registering'
             AND provider.current_policy_revision=head.observed_provider_policy_revision
             AND provider.current_provider_digest=head.observed_provider_digest
             THEN 'exact_registering'
           WHEN head.observed_provider_status='registering'
             AND provider.status='active'
             AND provider.current_policy_revision=head.observed_provider_policy_revision+1
             THEN 'adjacent_active'
           WHEN head.observed_provider_status='active'
             AND provider.status='active'
             AND provider.current_policy_revision=head.observed_provider_policy_revision
             AND provider.current_provider_digest=head.observed_provider_digest
             THEN 'exact_active'
           ELSE 'historical_only'
         END AS revision_status
    FROM heads head
    LEFT JOIN compute_external_pool_adapter_registry_provider_bindings binding
      ON binding.provider_binding_id=head.provider_binding_id
     AND binding.provider_binding_digest=head.provider_binding_digest
     AND binding.provider_binding_material_digest=head.provider_binding_material_digest
     AND binding.registry_release_id=head.registry_release_id
     AND binding.registry_release_digest=head.registry_release_digest
     AND binding.route_adapter_projection_id=head.route_adapter_projection_id
     AND binding.installation_receipt_id=head.installation_receipt_id
     AND binding.installation_receipt_digest=head.installation_receipt_digest
     AND binding.installation_content_digest=head.installation_content_digest
     AND binding.application_id=head.application_id
     AND binding.application_digest=head.application_digest
     AND binding.adoption_receipt_id=head.adoption_receipt_id
     AND binding.adoption_receipt_digest=head.adoption_receipt_digest
     AND binding.provider_id=head.provider_id
     AND binding.provider_owner_account_id=head.provider_owner_account_id
     AND binding.adapter_id=head.adapter_id
     AND binding.release_version=head.release_version
     AND binding.adapter_config_revision=head.adapter_config_revision
     AND binding.adapter_config_digest=head.adapter_config_digest
     AND binding.admission_id=head.admission_id
     AND binding.admission_digest=head.admission_digest
     AND binding.credential_verification_receipt_id=head.legacy_credential_verification_receipt_id
     AND binding.credential_verification_receipt_digest=head.legacy_credential_verification_receipt_digest
     AND binding.credential_locator_commitment=head.credential_locator_commitment
    LEFT JOIN compute_external_pool_adapter_registry_release_current release
      ON release.registry_release_id=head.registry_release_id
     AND release.registry_release_digest=head.registry_release_digest
    LEFT JOIN compute_external_pool_adapter_credential_verifier_key_current verifier
      ON verifier.key_record_id=head.credential_verifier_key_record_id
     AND verifier.key_record_digest=head.credential_verifier_key_record_digest
     AND verifier.verifier_record_id=head.credential_verifier_record_id
     AND verifier.verifier_record_digest=head.credential_verifier_record_digest
     AND verifier.key_id=head.credential_verifier_key_id
    LEFT JOIN compute_providers provider ON provider.provider_id=head.provider_id
    LEFT JOIN compute_external_pool_onboarding_applications onboarding
      ON onboarding.application_id=head.application_id
     AND onboarding.application_digest=head.application_digest
     AND onboarding.provider_id=head.provider_id
    LEFT JOIN compute_provider_versions version
      ON version.provider_id=provider.provider_id
     AND version.policy_revision=provider.current_policy_revision
     AND version.provider_digest=provider.current_provider_digest
)
SELECT 'compute_federation.external_pool_adapter_credential_reattestation_currentness.v1'
         AS currentness_schema,
       display.reattestation_receipt_id,
       display.reattestation_receipt_digest,
       display.provider_binding_id,
       display.provider_binding_digest,
       display.registry_release_id,
       display.registry_release_digest,
       display.provider_id,
       display.observed_provider_policy_revision,
       display.observed_provider_digest,
       display.observed_provider_status,
       display.sequence,
       display.verified_at,
       display.report_expires_at,
       CASE WHEN display.exact_binding_id IS NOT NULL
                  AND display.release_current_status='release_current'
                  AND installation_terminal.terminal_receipt_id IS NULL
                  AND adoption_terminal.terminal_receipt_id IS NULL
                  AND display.subject_status='subject_exact'
                  AND display.revision_status IN ('exact_registering','adjacent_active','exact_active')
                  AND display.verifier_current_status='active'
                  AND julianday(display.verified_at)<=julianday('now')
                  AND julianday(display.report_expires_at)>julianday('now')
                  AND revocation.revocation_receipt_id IS NULL
            THEN 'verified_current' ELSE 'historical_only' END AS current_status,
       'head' AS head_status,
       CASE WHEN display.exact_binding_id IS NULL THEN 'historical_exact' ELSE 'binding_exact' END
         AS provider_binding_status,
       CASE WHEN display.release_current_status='release_current' THEN 'release_current'
            ELSE 'historical_only' END AS registry_release_status,
       display.subject_status AS provider_subject_status,
       display.revision_status AS provider_revision_status,
       CASE WHEN display.verifier_current_status='active' THEN 'active' ELSE 'revoked' END
         AS credential_verifier_key_status,
       CASE WHEN julianday(display.verified_at)<=julianday('now')
                  AND julianday(display.report_expires_at)>julianday('now')
            THEN 'current' ELSE 'expired' END AS report_validity_status,
       CASE WHEN revocation.revocation_receipt_id IS NULL THEN 'none' ELSE 'revoked' END
         AS revocation_status
  FROM display
  LEFT JOIN compute_external_pool_adapter_installation_terminal_receipts installation_terminal
    ON installation_terminal.installation_receipt_id=display.installation_receipt_id
   AND installation_terminal.installation_receipt_digest=display.installation_receipt_digest
  LEFT JOIN compute_external_pool_adapter_adoption_terminal_receipts adoption_terminal
    ON adoption_terminal.adoption_receipt_id=display.adoption_receipt_id
   AND adoption_terminal.adoption_receipt_digest=display.adoption_receipt_digest
  LEFT JOIN compute_external_pool_adapter_credential_reattestation_revocations revocation
    ON revocation.reattestation_receipt_id=display.reattestation_receipt_id
   AND revocation.reattestation_receipt_digest=display.reattestation_receipt_digest;
