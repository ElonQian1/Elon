DROP TRIGGER IF EXISTS external_pool_adapter_credential_reattestation_receipt_current_roots;
CREATE TRIGGER external_pool_adapter_credential_reattestation_receipt_current_roots
BEFORE INSERT ON compute_external_pool_adapter_credential_reattestation_receipts
WHEN NOT EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_registry_release_current release
  JOIN compute_external_pool_adapter_credential_verifier_key_current verifier
    ON verifier.key_record_id=NEW.credential_verifier_key_record_id
   AND verifier.key_record_digest=NEW.credential_verifier_key_record_digest
   AND verifier.verifier_record_id=NEW.credential_verifier_record_id
   AND verifier.verifier_record_digest=NEW.credential_verifier_record_digest
   AND verifier.key_id=NEW.credential_verifier_key_id
   AND verifier.current_status='active'
  JOIN compute_providers provider ON provider.provider_id=NEW.provider_id
  JOIN compute_provider_versions version
    ON version.provider_id=provider.provider_id
   AND version.policy_revision=provider.current_policy_revision
   AND version.provider_digest=provider.current_provider_digest
  JOIN compute_external_pool_onboarding_applications onboarding
    ON onboarding.application_id=NEW.application_id
   AND onboarding.application_digest=NEW.application_digest
 WHERE release.registry_release_id=NEW.registry_release_id
   AND release.registry_release_digest=NEW.registry_release_digest
   AND release.current_status='release_current'
   AND provider.provider_kind='external_pool'
   AND provider.owner_account_id=NEW.provider_owner_account_id
   AND provider.status=NEW.observed_provider_status
   AND provider.current_policy_revision=NEW.observed_provider_policy_revision
   AND provider.current_provider_digest=NEW.observed_provider_digest
   AND json_extract(version.provider_json,'$.provider_id')=NEW.provider_id
   AND json_extract(version.provider_json,'$.provider_kind')='external_pool'
   AND json_extract(version.provider_json,'$.owner_account_id')=NEW.provider_owner_account_id
   AND json_extract(version.provider_json,'$.created_at')=json_extract(onboarding.target_provider_jcs,'$.created_at')
   AND json_extract(version.provider_json,'$.adapter.adapter_version')=NEW.release_version
   AND json_extract(version.provider_json,'$.adapter.config_revision')=NEW.adapter_config_revision
   AND json_extract(version.provider_json,'$.adapter.config_digest')=NEW.adapter_config_digest
   AND (
     (provider.status='registering'
      AND json_extract(version.provider_json,'$.adapter.adapter_id')=NEW.adapter_id)
     OR
     (provider.status='active'
      AND json_extract(version.provider_json,'$.adapter.adapter_id')=NEW.route_adapter_projection_id
      AND EXISTS (
        SELECT 1
          FROM compute_external_pool_adapter_registry_provider_bindings binding
          JOIN compute_external_pool_adapter_atomic_activation_receipts witness
            ON witness.provider_binding_id=binding.provider_binding_id
           AND witness.provider_binding_digest=binding.provider_binding_digest
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
         WHERE binding.provider_binding_id=NEW.provider_binding_id
           AND binding.provider_binding_digest=NEW.provider_binding_digest
           AND binding.provider_id=NEW.provider_id
           AND binding.provider_owner_account_id=NEW.provider_owner_account_id
           AND binding.route_adapter_projection_id=NEW.route_adapter_projection_id
           AND witness.provider_binding_digest=NEW.provider_binding_digest
           AND witness.source_registering_provider_id=NEW.provider_id
           AND witness.source_registering_provider_policy_revision=binding.provider_policy_revision
           AND witness.source_registering_provider_digest=binding.provider_digest
           AND witness.target_active_provider_id=provider.provider_id
           AND witness.target_active_provider_policy_revision<=provider.current_policy_revision
           AND witness.route_adapter_projection_id=NEW.route_adapter_projection_id
      ))
   ))
  OR EXISTS (
    SELECT 1 FROM compute_external_pool_adapter_installation_terminal_receipts terminal
     WHERE terminal.installation_receipt_id=NEW.installation_receipt_id
       AND terminal.installation_receipt_digest=NEW.installation_receipt_digest)
  OR EXISTS (
    SELECT 1 FROM compute_external_pool_adapter_adoption_terminal_receipts terminal
     WHERE terminal.adoption_receipt_id=NEW.adoption_receipt_id
       AND terminal.adoption_receipt_digest=NEW.adoption_receipt_digest)
BEGIN SELECT RAISE(ABORT,'V253 receipt lacks registering or V277-witnessed projected-active roots'); END;
