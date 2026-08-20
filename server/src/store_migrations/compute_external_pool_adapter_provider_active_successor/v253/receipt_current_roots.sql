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
   AND provider.status='registering'
   AND NEW.observed_provider_status='registering'
   AND provider.current_policy_revision=NEW.observed_provider_policy_revision
   AND provider.current_provider_digest=NEW.observed_provider_digest
   AND json_extract(version.provider_json,'$.provider_id')=NEW.provider_id
   AND json_extract(version.provider_json,'$.provider_kind')='external_pool'
   AND json_extract(version.provider_json,'$.owner_account_id')=NEW.provider_owner_account_id
   AND json_extract(version.provider_json,'$.created_at')=json_extract(onboarding.target_provider_jcs,'$.created_at')
   AND json_extract(version.provider_json,'$.adapter.adapter_id')=NEW.adapter_id
   AND json_extract(version.provider_json,'$.adapter.adapter_version')=NEW.release_version
   AND json_extract(version.provider_json,'$.adapter.config_revision')=NEW.adapter_config_revision
   AND json_extract(version.provider_json,'$.adapter.config_digest')=NEW.adapter_config_digest)
 OR EXISTS (
    SELECT 1 FROM compute_external_pool_adapter_installation_terminal_receipts terminal
     WHERE terminal.installation_receipt_id=NEW.installation_receipt_id
       AND terminal.installation_receipt_digest=NEW.installation_receipt_digest)
 OR EXISTS (
    SELECT 1 FROM compute_external_pool_adapter_adoption_terminal_receipts terminal
     WHERE terminal.adoption_receipt_id=NEW.adoption_receipt_id
       AND terminal.adoption_receipt_digest=NEW.adoption_receipt_digest)
BEGIN SELECT RAISE(ABORT,'V253 receipt is registering-only until V277 activation witness'); END;
