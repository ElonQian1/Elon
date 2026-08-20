CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_provider_credential_roots
BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_receipts
WHEN NOT EXISTS (
  SELECT 1
    FROM compute_providers provider
    JOIN compute_provider_versions evidence_version
      ON evidence_version.provider_id=provider.provider_id
     AND evidence_version.policy_revision=provider.current_policy_revision
     AND evidence_version.provider_digest=provider.current_provider_digest
    JOIN compute_provider_versions credential_version
      ON credential_version.provider_id=NEW.credential_observed_provider_id
     AND credential_version.policy_revision=NEW.credential_observed_provider_policy_revision
     AND credential_version.provider_digest=NEW.credential_observed_provider_digest
    JOIN compute_external_pool_adapter_credential_reattestation_receipts reattestation
      ON reattestation.reattestation_receipt_id=NEW.reattestation_receipt_id
     AND reattestation.reattestation_receipt_digest=NEW.reattestation_receipt_digest
    JOIN compute_external_pool_adapter_credential_verifier_key_current verifier
      ON verifier.key_record_id=reattestation.credential_verifier_key_record_id
     AND verifier.key_record_digest=reattestation.credential_verifier_key_record_digest
     AND verifier.key_id=reattestation.credential_verifier_key_id
     AND verifier.current_status='active'
   WHERE provider.provider_id=NEW.evidence_provider_id
     AND provider.provider_id=NEW.provider_id
     AND provider.provider_kind='external_pool'
     AND provider.owner_account_id=NEW.provider_owner_account_id
     AND provider.status='active'
     AND provider.current_policy_revision=NEW.evidence_provider_policy_revision
     AND provider.current_provider_digest=NEW.evidence_provider_digest
     AND evidence_version.provider_json=NEW.evidence_provider_json
     AND json_extract(evidence_version.provider_json,'$.provider_id')=NEW.provider_id
     AND json_extract(evidence_version.provider_json,'$.provider_kind')='external_pool'
     AND json_extract(evidence_version.provider_json,'$.owner_account_id')=NEW.provider_owner_account_id
     AND json_extract(evidence_version.provider_json,'$.status')='active'
     AND json_extract(evidence_version.provider_json,'$.adapter.adapter_id')=NEW.route_adapter_projection_id
     AND json_extract(evidence_version.provider_json,'$.updated_at')<=NEW.evidence_checked_at
     AND (NEW.successor_sequence>1 OR json_extract(evidence_version.provider_json,'$.updated_at')=NEW.activation_target_updated_at)
     AND NEW.runtime_observed_provider_id=NEW.evidence_provider_id
     AND NEW.runtime_observed_provider_policy_revision=NEW.evidence_provider_policy_revision
     AND NEW.runtime_observed_provider_json=NEW.evidence_provider_json
     AND NEW.runtime_observed_provider_digest=NEW.evidence_provider_digest
     AND credential_version.provider_json=NEW.credential_observed_provider_json
     AND reattestation.provider_binding_id=NEW.provider_binding_id
     AND reattestation.provider_binding_digest=NEW.provider_binding_digest
     AND reattestation.registry_release_id=NEW.registry_release_id
     AND reattestation.registry_release_digest=NEW.registry_release_digest
     AND reattestation.registry_release_material_digest=NEW.registry_release_material_digest
     AND reattestation.route_adapter_projection_id=NEW.route_adapter_projection_id
     AND reattestation.installation_receipt_id=NEW.installation_receipt_id
     AND reattestation.installation_receipt_digest=NEW.installation_receipt_digest
     AND reattestation.installation_content_digest=NEW.installation_content_digest
     AND reattestation.provider_id=NEW.provider_id
     AND reattestation.provider_owner_account_id=NEW.provider_owner_account_id
     AND reattestation.adapter_id=NEW.logical_adapter_id
     AND reattestation.observed_provider_policy_revision=NEW.credential_observed_provider_policy_revision
     AND reattestation.observed_provider_digest=NEW.credential_observed_provider_digest
     AND reattestation.observed_provider_status=json_extract(credential_version.provider_json,'$.status')
     AND reattestation.verified_at<=NEW.evidence_checked_at
     AND NEW.observation_expires_at<=reattestation.report_expires_at
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts successor WHERE successor.predecessor_receipt_id=reattestation.reattestation_receipt_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_revocations revoked WHERE revoked.reattestation_receipt_id=reattestation.reattestation_receipt_id)
     AND ((NEW.successor_sequence=1
           AND reattestation.observed_provider_status='registering'
           AND json_extract(credential_version.provider_json,'$.adapter.adapter_id')=NEW.logical_adapter_id
           AND NEW.credential_observed_provider_id=NEW.source_registering_provider_id
           AND NEW.credential_observed_provider_policy_revision=NEW.source_registering_provider_policy_revision
           AND NEW.credential_observed_provider_json=NEW.source_registering_provider_json
           AND NEW.credential_observed_provider_digest=NEW.source_registering_provider_digest)
          OR (NEW.successor_sequence>1
              AND reattestation.observed_provider_status='active'
              AND json_extract(credential_version.provider_json,'$.adapter.adapter_id')=NEW.route_adapter_projection_id
              AND NEW.credential_observed_provider_id=NEW.evidence_provider_id
              AND NEW.credential_observed_provider_policy_revision=NEW.evidence_provider_policy_revision
              AND NEW.credential_observed_provider_json=NEW.evidence_provider_json
              AND NEW.credential_observed_provider_digest=NEW.evidence_provider_digest)))
BEGIN SELECT RAISE(ABORT,'V274 active successor lacks exact live Provider/V253 evidence'); END;
