DROP TRIGGER IF EXISTS external_pool_adapter_credential_reattestation_challenge_exact_roots;
CREATE TRIGGER external_pool_adapter_credential_reattestation_challenge_exact_roots
BEFORE INSERT ON compute_external_pool_adapter_credential_reattestation_challenges
WHEN NOT EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_registry_provider_bindings binding
  JOIN compute_external_pool_adapter_registry_release_current release
    ON release.registry_release_id=binding.registry_release_id
   AND release.registry_release_digest=binding.registry_release_digest
   AND release.current_status='release_current'
  JOIN compute_external_pool_adapter_registry_releases release_root
    ON release_root.registry_release_id=release.registry_release_id
   AND release_root.registry_release_digest=release.registry_release_digest
  JOIN compute_external_pool_adapter_credential_verification_receipts legacy
    ON legacy.credential_verification_receipt_id=binding.credential_verification_receipt_id
   AND legacy.credential_verification_receipt_digest=binding.credential_verification_receipt_digest
  JOIN compute_external_pool_adapter_credential_verifier_key_current verifier
    ON verifier.key_record_id=NEW.credential_verifier_key_record_id
   AND verifier.key_record_digest=NEW.credential_verifier_key_record_digest
   AND verifier.key_id=NEW.credential_verifier_key_id
   AND verifier.current_status='active'
  JOIN compute_providers provider ON provider.provider_id=binding.provider_id
  JOIN compute_provider_versions version
    ON version.provider_id=provider.provider_id
   AND version.policy_revision=provider.current_policy_revision
   AND version.provider_digest=provider.current_provider_digest
  JOIN compute_external_pool_onboarding_applications onboarding
    ON onboarding.application_id=binding.application_id
   AND onboarding.application_digest=binding.application_digest
 WHERE binding.provider_binding_id=NEW.provider_binding_id
   AND binding.provider_binding_digest=NEW.provider_binding_digest
   AND binding.provider_binding_material_digest=NEW.provider_binding_material_digest
   AND binding.registry_release_id=NEW.registry_release_id
   AND binding.registry_release_digest=NEW.registry_release_digest
   AND release.registry_release_id=NEW.registry_release_id
   AND release_root.registry_release_material_digest=NEW.registry_release_material_digest
   AND release_root.credential_verifier_digest=verifier.verifier_digest
   AND json_extract(release_root.credential_verifier_json,'$.verification_kind')=verifier.verification_kind
   AND json_extract(release_root.credential_verifier_json,'$.verifier_id')=verifier.verifier_id
   AND json_extract(release_root.credential_verifier_json,'$.verifier_revision')=verifier.verifier_revision
   AND json_extract(release_root.credential_verifier_json,'$.verifier_digest')=verifier.verifier_digest
   AND json_extract(NEW.challenge_json,'$.binding.route_adapter_projection_id')=binding.route_adapter_projection_id
   AND json_extract(NEW.challenge_json,'$.binding.installation_receipt_id')=binding.installation_receipt_id
   AND json_extract(NEW.challenge_json,'$.binding.installation_receipt_digest')=binding.installation_receipt_digest
   AND json_extract(NEW.challenge_json,'$.binding.installation_content_digest')=binding.installation_content_digest
   AND json_extract(NEW.challenge_json,'$.binding.application_id')=binding.application_id
   AND json_extract(NEW.challenge_json,'$.binding.application_digest')=binding.application_digest
   AND legacy.application_id=binding.application_id
   AND legacy.application_digest=binding.application_digest
   AND json_extract(NEW.challenge_json,'$.binding.adoption_receipt_id')=binding.adoption_receipt_id
   AND json_extract(NEW.challenge_json,'$.binding.adoption_receipt_digest')=binding.adoption_receipt_digest
   AND json_extract(NEW.challenge_json,'$.binding.provider_id')=binding.provider_id
   AND legacy.provider_id=binding.provider_id
   AND legacy.provider_policy_revision=binding.provider_policy_revision
   AND legacy.provider_digest=binding.provider_digest
   AND json_extract(NEW.challenge_json,'$.binding.provider_kind')='external_pool'
   AND json_extract(NEW.challenge_json,'$.binding.provider_owner_account_id')=binding.provider_owner_account_id
   AND json_extract(NEW.challenge_json,'$.binding.observed_settlement_account_id')=provider.settlement_account_id
   AND json_extract(NEW.challenge_json,'$.binding.adapter_id')=binding.adapter_id
   AND json_extract(NEW.challenge_json,'$.binding.release_version')=binding.release_version
   AND json_extract(NEW.challenge_json,'$.binding.adapter_config_revision')=binding.adapter_config_revision
   AND json_extract(NEW.challenge_json,'$.binding.adapter_config_digest')=binding.adapter_config_digest
   AND legacy.adapter_id=binding.adapter_id
   AND legacy.adapter_release_version=binding.release_version
   AND legacy.adapter_config_revision=binding.adapter_config_revision
   AND legacy.adapter_config_digest=binding.adapter_config_digest
   AND json_extract(NEW.challenge_json,'$.binding.admission_id')=binding.admission_id
   AND json_extract(NEW.challenge_json,'$.binding.admission_digest')=binding.admission_digest
   AND legacy.admission_id=binding.admission_id
   AND legacy.admission_digest=binding.admission_digest
   AND json_extract(NEW.challenge_json,'$.binding.legacy_credential_verification_receipt_id')=legacy.credential_verification_receipt_id
   AND json_extract(NEW.challenge_json,'$.binding.legacy_credential_verification_receipt_digest')=legacy.credential_verification_receipt_digest
   AND json_extract(NEW.challenge_json,'$.binding.credential_ref_scheme')=legacy.credential_ref_scheme
   AND json_extract(NEW.challenge_json,'$.binding.credential_locator_commitment')=binding.credential_locator_commitment
   AND legacy.credential_locator_commitment=binding.credential_locator_commitment
   AND json(json_extract(NEW.challenge_json,'$.binding.expected_credential_verifier'))=json(release_root.credential_verifier_json)
   AND json_extract(NEW.challenge_json,'$.binding.credential_verifier_digest')=release_root.credential_verifier_digest
   AND json_extract(NEW.challenge_json,'$.binding.credential_verifier_record_id')=verifier.verifier_record_id
   AND json_extract(NEW.challenge_json,'$.binding.credential_verifier_record_digest')=verifier.verifier_record_digest
   AND provider.provider_kind='external_pool'
   AND provider.owner_account_id=binding.provider_owner_account_id
   AND provider.current_policy_revision=NEW.observed_provider_policy_revision
   AND provider.current_provider_digest=NEW.observed_provider_digest
   AND provider.status=NEW.observed_provider_status
   AND json_extract(version.provider_json,'$.provider_id')=binding.provider_id
   AND json_extract(version.provider_json,'$.provider_kind')='external_pool'
   AND json_extract(version.provider_json,'$.owner_account_id')=binding.provider_owner_account_id
   AND json_extract(version.provider_json,'$.created_at')=json_extract(onboarding.target_provider_jcs,'$.created_at')
   AND json_extract(version.provider_json,'$.adapter.adapter_version')=binding.release_version
   AND json_extract(version.provider_json,'$.adapter.config_revision')=binding.adapter_config_revision
   AND json_extract(version.provider_json,'$.adapter.config_digest')=binding.adapter_config_digest
   AND (
     (provider.status='registering'
      AND provider.current_policy_revision=binding.provider_policy_revision
      AND provider.current_provider_digest=binding.provider_digest
      AND json_extract(version.provider_json,'$.adapter.adapter_id')=binding.adapter_id)
     OR
     (provider.status='active'
      AND provider.current_policy_revision>binding.provider_policy_revision
      AND json_extract(version.provider_json,'$.adapter.adapter_id')=binding.route_adapter_projection_id
      AND EXISTS (
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
         WHERE witness.provider_binding_id=binding.provider_binding_id
           AND witness.provider_binding_digest=binding.provider_binding_digest
           AND witness.source_registering_provider_id=binding.provider_id
           AND witness.source_registering_provider_policy_revision=binding.provider_policy_revision
           AND witness.source_registering_provider_digest=binding.provider_digest
           AND witness.target_active_provider_id=provider.provider_id
           AND witness.target_active_provider_policy_revision<=provider.current_policy_revision
           AND witness.route_adapter_projection_id=binding.route_adapter_projection_id
      ))
   ))
  OR EXISTS (
    SELECT 1 FROM compute_external_pool_adapter_registry_provider_bindings binding
    JOIN compute_external_pool_adapter_installation_terminal_receipts terminal
      ON terminal.installation_receipt_id=binding.installation_receipt_id
     AND terminal.installation_receipt_digest=binding.installation_receipt_digest
   WHERE binding.provider_binding_id=NEW.provider_binding_id)
  OR EXISTS (
    SELECT 1 FROM compute_external_pool_adapter_registry_provider_bindings binding
    JOIN compute_external_pool_adapter_adoption_terminal_receipts terminal
      ON terminal.adoption_receipt_id=binding.adoption_receipt_id
     AND terminal.adoption_receipt_digest=binding.adoption_receipt_digest
   WHERE binding.provider_binding_id=NEW.provider_binding_id)
BEGIN SELECT RAISE(ABORT,'V253 challenge lacks registering or V277-witnessed projected-active roots'); END;
