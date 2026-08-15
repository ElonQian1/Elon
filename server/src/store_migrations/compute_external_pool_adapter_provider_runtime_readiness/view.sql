DROP VIEW IF EXISTS compute_external_pool_adapter_provider_runtime_readiness_current;

CREATE VIEW compute_external_pool_adapter_provider_runtime_readiness_current AS
SELECT receipt.readiness_receipt_id,
       receipt.readiness_receipt_digest,
       receipt.readiness_material_digest,
       receipt.policy_id,
       receipt.policy_revision,
       receipt.policy_digest,
       receipt.provider_binding_id,
       receipt.provider_binding_digest,
       receipt.registry_release_id,
       receipt.registry_release_digest,
       receipt.registry_release_material_digest,
       receipt.installation_receipt_id,
       receipt.installation_receipt_digest,
       receipt.installation_content_digest,
       receipt.candidate_id,
       receipt.candidate_digest,
       receipt.delegation_id,
       receipt.delegation_digest,
       receipt.profile_id,
       receipt.profile_digest,
       receipt.target_id,
       receipt.target_digest,
       receipt.companion_id,
       receipt.companion_digest,
       receipt.provider_id,
       receipt.provider_policy_revision,
       receipt.provider_digest,
       receipt.provider_status,
       receipt.vulnerability_reattestation_receipt_id,
       receipt.vulnerability_reattestation_receipt_digest,
       receipt.sandbox_reattestation_receipt_id,
       receipt.sandbox_reattestation_receipt_digest,
       receipt.credential_reattestation_receipt_id,
       receipt.credential_reattestation_receipt_digest,
       receipt.runtime_compatibility_verification_receipt_id,
       receipt.runtime_compatibility_verification_receipt_digest,
       receipt.sequence,
       receipt.predecessor_readiness_receipt_id,
       receipt.predecessor_readiness_receipt_digest,
       receipt.probe_checked_at,
       receipt.cleanup_completed_at,
       receipt.checked_at,
       receipt.expires_at,
       receipt.evidence_scope,
       receipt.receipt_status,
       receipt.effects_json,
       receipt.observed_readiness_json,
       CASE WHEN NOT EXISTS (
                    SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_receipts successor
                     WHERE successor.predecessor_readiness_receipt_id=receipt.readiness_receipt_id)
            THEN 'head' ELSE 'superseded' END AS head_status,
       CASE WHEN revocation.revocation_receipt_id IS NULL THEN 'unrevoked'
            ELSE 'revoked' END AS revocation_status,
       CASE WHEN receipt.expires_at>strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
            THEN 'within_short_ttl' ELSE 'expired' END AS ttl_status,
       CASE WHEN current_profile.provider_binding_status='binding_current'
            THEN 'binding_current' ELSE 'historical_only' END AS provider_binding_status,
       CASE WHEN current_profile.provider_revision_status='exact_registering'
            THEN 'exact_registering' ELSE 'historical_only' END AS provider_status,
       CASE WHEN current_profile.candidate_status='candidate_current_not_activation_ready'
            THEN 'candidate_current_not_activation_ready' ELSE 'historical_only' END
         AS candidate_status,
       CASE WHEN current_companion.profile_status='launch_profile_current_inert'
            THEN 'launch_profile_current_inert' ELSE 'historical_only' END AS profile_status,
       CASE WHEN current_companion.target_status='upstream_transport_target_current_inert'
            THEN 'upstream_transport_target_current_inert' ELSE 'historical_only' END
         AS target_status,
       CASE WHEN current_companion.current_status='supervisor_session_policy_companion_current_inert'
            THEN 'current' ELSE 'historical_only' END AS companion_status,
       CASE WHEN current_vulnerability.current_status='verified_current'
            THEN 'verified_current' ELSE 'historical_only' END AS vulnerability_reattestation_status,
       CASE WHEN current_sandbox.current_status='verified_current'
            THEN 'verified_current' ELSE 'historical_only' END AS sandbox_reattestation_status,
       CASE WHEN current_credential.current_status='verified_current'
            THEN 'verified_current' ELSE 'historical_only' END AS credential_reattestation_status,
       CASE WHEN current_compatibility.currentness_status='current_signed_verifier_assertion'
            THEN 'current_signed_verifier_assertion' ELSE 'historical_only' END
         AS runtime_compatibility_verification_status,
       CASE WHEN elon_v270_provider_runtime_readiness_receipt_is_exact(receipt.readiness_receipt_json)=1
            THEN 'exact' ELSE 'invalid' END AS receipt_integrity_status,
       CASE WHEN NOT EXISTS (
                    SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_receipts successor
                     WHERE successor.predecessor_readiness_receipt_id=receipt.readiness_receipt_id)
                  AND revocation.revocation_receipt_id IS NULL
                  AND receipt.expires_at>strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
                  AND current_companion.current_status='supervisor_session_policy_companion_current_inert'
                  AND current_vulnerability.current_status='verified_current'
                  AND current_sandbox.current_status='verified_current'
                  AND current_credential.current_status='verified_current'
                  AND current_compatibility.currentness_status='current_signed_verifier_assertion'
                  AND elon_v270_provider_runtime_readiness_receipt_is_exact(receipt.readiness_receipt_json)=1
            THEN 'relationally_current_requires_process_custody_reproof'
            ELSE 'historical_only' END AS current_status
  FROM compute_external_pool_adapter_provider_runtime_readiness_receipts receipt
  LEFT JOIN compute_external_pool_adapter_provider_runtime_readiness_revocations revocation
    ON revocation.readiness_receipt_id=receipt.readiness_receipt_id
   AND revocation.readiness_receipt_digest=receipt.readiness_receipt_digest
  LEFT JOIN compute_external_pool_adapter_supervisor_session_policy_companion_current current_companion
    ON current_companion.companion_id=receipt.companion_id
   AND current_companion.companion_digest=receipt.companion_digest
  LEFT JOIN compute_external_pool_adapter_runtime_launch_profile_current current_profile
    ON current_profile.profile_id=receipt.profile_id
   AND current_profile.profile_digest=receipt.profile_digest
  LEFT JOIN compute_external_pool_adapter_vulnerability_reattestation_current current_vulnerability
    ON current_vulnerability.reattestation_receipt_id=receipt.vulnerability_reattestation_receipt_id
   AND current_vulnerability.reattestation_receipt_digest=receipt.vulnerability_reattestation_receipt_digest
  LEFT JOIN compute_external_pool_adapter_sandbox_reattestation_current current_sandbox
    ON current_sandbox.reattestation_receipt_id=receipt.sandbox_reattestation_receipt_id
   AND current_sandbox.reattestation_receipt_digest=receipt.sandbox_reattestation_receipt_digest
  LEFT JOIN compute_external_pool_adapter_credential_reattestation_current current_credential
    ON current_credential.reattestation_receipt_id=receipt.credential_reattestation_receipt_id
   AND current_credential.reattestation_receipt_digest=receipt.credential_reattestation_receipt_digest
  LEFT JOIN compute_external_pool_adapter_runtime_compatibility_verification_current current_compatibility
    ON current_compatibility.verification_receipt_id=receipt.runtime_compatibility_verification_receipt_id
   AND current_compatibility.verification_receipt_digest=receipt.runtime_compatibility_verification_receipt_digest;
