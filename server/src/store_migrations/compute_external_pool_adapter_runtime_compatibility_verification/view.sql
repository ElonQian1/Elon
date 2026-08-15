DROP VIEW IF EXISTS compute_external_pool_adapter_runtime_compatibility_verification_current;
CREATE VIEW compute_external_pool_adapter_runtime_compatibility_verification_current AS
SELECT verification.verification_receipt_id,
       verification.verification_receipt_digest,
       verification.registry_release_id,
       verification.registry_release_digest,
       verification.run_observation_id,
       verification.run_observation_digest,
       verification.profile_id,
       verification.profile_revision,
       verification.sequence,
       verification.verified_at,
       verification.expires_at,
       revocation.revoked_at,
       release.adapter_id,
       release.release_version,
       CASE WHEN verification.sequence=(SELECT MAX(head.sequence) FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts head WHERE head.registry_release_id=verification.registry_release_id)
                  AND revocation.revocation_receipt_id IS NULL
                  AND julianday(verification.expires_at)>julianday('now')
                  AND release.current_status='release_current'
                  AND key.current_status='active'
                  AND verification.profile_digest=__PROFILE_DIGEST_SQL__
                  AND verification.runner_policy_digest=__RUNNER_POLICY_DIGEST_SQL__
                  AND verification.fixture_catalog_digest=__FIXTURE_CATALOG_DIGEST_SQL__
                  AND elon_v268_runtime_compatibility_verification_is_exact(
                    challenge.challenge_json,observation.run_observation_json,
                    verification.verification_receipt_json,key_root.public_key_pem)=1
            THEN 'current_signed_verifier_assertion'
            ELSE 'historical_signed_verifier_assertion' END AS currentness_status,
       CASE WHEN verification.sequence=(SELECT MAX(head.sequence) FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts head WHERE head.registry_release_id=verification.registry_release_id) THEN 'head' ELSE 'superseded' END AS lineage_status,
       CASE WHEN revocation.revocation_receipt_id IS NULL THEN 'unrevoked' ELSE 'revoked' END AS revocation_status,
       CASE WHEN julianday(verification.expires_at)>julianday('now') THEN 'within_validity_window' ELSE 'expired' END AS validity_status,
       COALESCE(release.current_status,'historical_only') AS release_status,
       COALESCE(key.current_status,'not_active') AS verifier_key_status,
       CASE WHEN verification.profile_digest=__PROFILE_DIGEST_SQL__
                  AND verification.runner_policy_digest=__RUNNER_POLICY_DIGEST_SQL__
                  AND verification.fixture_catalog_digest=__FIXTURE_CATALOG_DIGEST_SQL__
            THEN 'current' ELSE 'historical' END AS policy_status,
       elon_v268_runtime_compatibility_verification_is_exact(
         challenge.challenge_json,observation.run_observation_json,
         verification.verification_receipt_json,key_root.public_key_pem) AS signature_integrity
  FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts verification
  JOIN compute_external_pool_adapter_runtime_compatibility_verification_challenges challenge
    ON challenge.challenge_id=verification.challenge_id AND challenge.challenge_digest=verification.challenge_digest
  JOIN compute_external_pool_adapter_runtime_compatibility_verification_run_observations observation
    ON observation.run_observation_id=verification.run_observation_id AND observation.run_observation_digest=verification.run_observation_digest
  JOIN compute_external_pool_adapter_sandbox_verifier_keys key_root
    ON key_root.key_record_id=verification.sandbox_verifier_key_record_id
   AND key_root.key_record_digest=verification.sandbox_verifier_key_record_digest
   AND key_root.key_id=verification.sandbox_verifier_key_id
  LEFT JOIN compute_external_pool_adapter_sandbox_verifier_key_current key
    ON key.key_record_id=verification.sandbox_verifier_key_record_id
   AND key.key_record_digest=verification.sandbox_verifier_key_record_digest
   AND key.key_id=verification.sandbox_verifier_key_id
  LEFT JOIN compute_external_pool_adapter_registry_release_current release
    ON release.registry_release_id=verification.registry_release_id
   AND release.registry_release_digest=verification.registry_release_digest
  LEFT JOIN compute_external_pool_adapter_runtime_compatibility_verification_revocations revocation
    ON revocation.verification_receipt_id=verification.verification_receipt_id
   AND revocation.verification_receipt_digest=verification.verification_receipt_digest;
