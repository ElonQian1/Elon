DROP VIEW IF EXISTS compute_external_pool_adapter_credential_reattestation_current;

CREATE VIEW compute_external_pool_adapter_credential_reattestation_current AS
WITH heads AS (
  SELECT receipt.*
    FROM compute_external_pool_adapter_credential_reattestation_receipts receipt
   WHERE NOT EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts successor
      WHERE successor.predecessor_receipt_id=receipt.reattestation_receipt_id)
), bound AS (
  SELECT head.*,
         binding.provider_binding_id AS exact_binding_id,
         release.current_status AS release_current_status,
         verifier.current_status AS verifier_current_status,
         provider.provider_kind AS live_provider_kind,
         provider.status AS live_provider_status,
         provider.current_policy_revision AS live_provider_policy_revision,
         provider.current_provider_digest AS live_provider_digest,
         provider.owner_account_id AS live_provider_owner_account_id,
         version.provider_json AS live_provider_json,
         json_extract(onboarding.target_provider_jcs,'$.created_at') AS source_created_at,
         CASE WHEN EXISTS (
           SELECT 1
             FROM compute_external_pool_adapter_atomic_activation_receipts witness
             JOIN compute_external_pool_adapter_provider_active_successor_receipts genesis
               ON genesis.activation_witness_id=witness.activation_receipt_id
              AND genesis.activation_witness_digest=witness.activation_receipt_digest
              AND genesis.activation_root_digest=witness.activation_root_digest
              AND genesis.provider_binding_id=witness.provider_binding_id
              AND genesis.provider_binding_digest=witness.provider_binding_digest
              AND genesis.successor_sequence=1
              AND genesis.source_registering_provider_id=witness.source_registering_provider_id
              AND genesis.source_registering_provider_policy_revision=witness.source_registering_provider_policy_revision
              AND genesis.source_registering_provider_digest=witness.source_registering_provider_digest
              AND genesis.initial_active_provider_id=witness.target_active_provider_id
              AND genesis.initial_active_provider_policy_revision=witness.target_active_provider_policy_revision
              AND genesis.initial_active_provider_digest=witness.target_active_provider_digest
              AND genesis.route_adapter_projection_id=witness.route_adapter_projection_id
            WHERE witness.provider_binding_id=head.provider_binding_id
              AND witness.provider_binding_digest=head.provider_binding_digest
              AND witness.source_registering_provider_id=head.provider_id
              AND witness.source_registering_provider_policy_revision=binding.provider_policy_revision
              AND witness.source_registering_provider_digest=binding.provider_digest
              AND witness.target_active_provider_id=provider.provider_id
              AND witness.target_active_provider_policy_revision<=provider.current_policy_revision
              AND witness.route_adapter_projection_id=head.route_adapter_projection_id
         ) THEN 1 ELSE 0 END AS activation_witness_exact
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
), display AS (
  SELECT bound.*,
         CASE
           WHEN live_provider_status IS NULL THEN 'drifted'
           WHEN live_provider_kind IS NOT 'external_pool'
             OR json_extract(live_provider_json,'$.provider_id') IS NOT provider_id
             OR json_extract(live_provider_json,'$.provider_kind') IS NOT 'external_pool'
             OR live_provider_owner_account_id IS NOT provider_owner_account_id
             OR json_extract(live_provider_json,'$.owner_account_id') IS NOT provider_owner_account_id
             OR json_extract(live_provider_json,'$.created_at') IS NOT source_created_at
             OR json_extract(live_provider_json,'$.adapter.adapter_version') IS NOT release_version
             OR json_extract(live_provider_json,'$.adapter.config_revision') IS NOT adapter_config_revision
             OR json_extract(live_provider_json,'$.adapter.config_digest') IS NOT adapter_config_digest
             OR NOT (
               (live_provider_status='registering'
                AND json_extract(live_provider_json,'$.adapter.adapter_id')=adapter_id)
               OR
               (live_provider_status='active'
                AND activation_witness_exact=1
                AND json_extract(live_provider_json,'$.adapter.adapter_id')=route_adapter_projection_id)
             ) THEN 'drifted'
           ELSE 'subject_exact'
         END AS subject_status,
         CASE
           WHEN live_provider_status='registering'
             AND observed_provider_status='registering'
             AND live_provider_policy_revision=observed_provider_policy_revision
             AND live_provider_digest=observed_provider_digest
             THEN 'exact_registering'
           WHEN live_provider_status='active'
             AND activation_witness_exact=1
             AND observed_provider_status='active'
             AND live_provider_policy_revision=observed_provider_policy_revision
             AND live_provider_digest=observed_provider_digest
             THEN 'witnessed_projected_active'
           ELSE 'historical_only'
         END AS revision_status
    FROM bound
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
                  AND display.revision_status IN ('exact_registering','witnessed_projected_active')
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
