DROP VIEW IF EXISTS compute_external_pool_adapter_task_protocol_conformance_current;

CREATE VIEW compute_external_pool_adapter_task_protocol_conformance_current AS
SELECT 'compute_federation.external_pool_adapter_task_protocol_conformance_currentness.v1'
         AS currentness_schema,
       receipt.run_receipt_id,
       receipt.run_receipt_digest,
       receipt.run_material_digest,
       receipt.registry_release_id,
       receipt.registry_release_digest,
       receipt.vulnerability_reattestation_receipt_id,
       receipt.vulnerability_reattestation_receipt_digest,
       receipt.sandbox_reattestation_receipt_id,
       receipt.sandbox_reattestation_receipt_digest,
       receipt.sandbox_verifier_key_record_id,
       receipt.sandbox_verifier_key_record_digest,
       receipt.sandbox_verifier_key_id,
       receipt.runtime_compatibility_verification_receipt_id,
       receipt.runtime_compatibility_verification_receipt_digest,
       receipt.runtime_compatibility_run_observation_id,
       receipt.runtime_compatibility_run_observation_digest,
       receipt.task_protocol_profile_id,
       receipt.task_protocol_profile_revision,
       receipt.task_protocol_profile_digest,
       receipt.fixture_catalog_id,
       receipt.fixture_catalog_revision,
       receipt.fixture_catalog_digest,
       receipt.runtime_compatibility_public_fixture_delivery_root,
       receipt.public_fixture_delivery_root,
       receipt.session_roots_digest,
       receipt.session_transcript_digest,
       receipt.delivery_inventory_digest,
       receipt.exchange_inventory_digest,
       receipt.task_observation_root,
       receipt.sequence,
       receipt.predecessor_run_receipt_id,
       receipt.predecessor_run_receipt_digest,
       receipt.post_cleanup_checked_at,
       receipt.expires_at,
       receipt.recorded_at,
       receipt.evidence_scope,
       receipt.receipt_status,
       receipt.non_production_authority_status,
       receipt.effects_json,
       receipt.readiness_json,
       CASE WHEN successor.run_receipt_id IS NULL THEN 'head'
            ELSE 'superseded' END AS head_status,
       CASE WHEN revocation.revocation_receipt_id IS NULL THEN 'unrevoked'
            ELSE 'revoked' END AS revocation_status,
       CASE WHEN receipt.expires_at>strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
            THEN 'within_short_ttl' ELSE 'expired' END AS ttl_status,
       CASE WHEN current_release.current_status='release_current'
            THEN 'release_current' ELSE 'historical_only' END AS registry_release_status,
       CASE WHEN current_vulnerability.current_status='verified_current'
            THEN 'verified_current' ELSE 'historical_only' END
         AS vulnerability_reattestation_status,
       CASE WHEN current_sandbox.current_status='verified_current'
            THEN 'verified_current' ELSE 'historical_only' END
         AS sandbox_reattestation_status,
       CASE WHEN current_verifier.current_status='active'
            THEN 'active' ELSE 'historical_only' END AS sandbox_verifier_key_status,
       CASE WHEN current_compatibility.currentness_status='current_signed_verifier_assertion'
            THEN 'current_signed_verifier_assertion' ELSE 'historical_only' END
         AS runtime_compatibility_verification_status,
       CASE WHEN receipt.task_protocol_profile_digest=__TASK_PROTOCOL_PROFILE_DIGEST_SQL__
            THEN 'server_profile_current' ELSE 'historical_only' END
         AS task_protocol_profile_status,
       CASE WHEN receipt.fixture_catalog_digest=__FIXTURE_CATALOG_DIGEST_SQL__
            THEN 'server_fixture_catalog_current' ELSE 'historical_only' END
         AS fixture_catalog_status,
       CASE WHEN elon_v272_task_protocol_conformance_run_receipt_is_exact(
                        receipt.run_receipt_json)=1
            THEN 'exact' ELSE 'invalid' END AS canonical_receipt_integrity_status,
       CASE WHEN elon_v272_task_protocol_conformance_receipt_integrity_is_exact(
                        receipt.run_receipt_digest,receipt.runtime_custody_epoch_digest,
                        receipt.process_hmac_seal,receipt.receipt_integrity_digest)=1
            THEN 'exact' ELSE 'invalid' END AS receipt_integrity_status,
       'requires_same_process_committed_seal_reproof' AS process_custody_status,
       'requires_fresh_prepared_execution_carrier_reproof' AS prepared_reproof_status,
       CASE WHEN successor.run_receipt_id IS NULL
                  AND revocation.revocation_receipt_id IS NULL
                  AND receipt.expires_at>strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
                  AND current_release.current_status='release_current'
                  AND current_vulnerability.current_status='verified_current'
                  AND current_sandbox.current_status='verified_current'
                  AND current_verifier.current_status='active'
                  AND current_compatibility.currentness_status='current_signed_verifier_assertion'
                  AND receipt.task_protocol_profile_digest=__TASK_PROTOCOL_PROFILE_DIGEST_SQL__
                  AND receipt.fixture_catalog_digest=__FIXTURE_CATALOG_DIGEST_SQL__
                  AND elon_v272_task_protocol_conformance_run_receipt_is_exact(
                        receipt.run_receipt_json)=1
                  AND elon_v272_task_protocol_conformance_receipt_integrity_is_exact(
                        receipt.run_receipt_digest,receipt.runtime_custody_epoch_digest,
                        receipt.process_hmac_seal,receipt.receipt_integrity_digest)=1
            THEN 'relationally_current_requires_process_custody_and_prepared_reproof'
            ELSE 'historical_only' END AS current_status
  FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts receipt
  LEFT JOIN compute_external_pool_adapter_task_protocol_conformance_run_receipts successor
    ON successor.predecessor_run_receipt_id=receipt.run_receipt_id
   AND successor.predecessor_run_receipt_digest=receipt.run_receipt_digest
  LEFT JOIN compute_external_pool_adapter_task_protocol_conformance_revocations revocation
    ON revocation.run_receipt_id=receipt.run_receipt_id
   AND revocation.run_receipt_digest=receipt.run_receipt_digest
  LEFT JOIN compute_external_pool_adapter_registry_release_current current_release
    ON current_release.registry_release_id=receipt.registry_release_id
   AND current_release.registry_release_digest=receipt.registry_release_digest
  LEFT JOIN compute_external_pool_adapter_vulnerability_reattestation_current current_vulnerability
    ON current_vulnerability.reattestation_receipt_id=receipt.vulnerability_reattestation_receipt_id
   AND current_vulnerability.reattestation_receipt_digest=receipt.vulnerability_reattestation_receipt_digest
  LEFT JOIN compute_external_pool_adapter_sandbox_reattestation_current current_sandbox
    ON current_sandbox.reattestation_receipt_id=receipt.sandbox_reattestation_receipt_id
   AND current_sandbox.reattestation_receipt_digest=receipt.sandbox_reattestation_receipt_digest
  LEFT JOIN compute_external_pool_adapter_sandbox_verifier_key_current current_verifier
    ON current_verifier.key_record_id=receipt.sandbox_verifier_key_record_id
   AND current_verifier.key_record_digest=receipt.sandbox_verifier_key_record_digest
   AND current_verifier.key_id=receipt.sandbox_verifier_key_id
   AND current_verifier.verifier_operator=receipt.sandbox_verifier_operator
   AND current_verifier.verifier_product=receipt.sandbox_verifier_product
  LEFT JOIN compute_external_pool_adapter_runtime_compatibility_verification_current current_compatibility
    ON current_compatibility.verification_receipt_id=receipt.runtime_compatibility_verification_receipt_id
   AND current_compatibility.verification_receipt_digest=receipt.runtime_compatibility_verification_receipt_digest;
