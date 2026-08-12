DROP VIEW IF EXISTS compute_external_pool_adapter_sandbox_reattestation_current;

CREATE VIEW compute_external_pool_adapter_sandbox_reattestation_current AS
SELECT 'compute_federation.external_pool_adapter_sandbox_reattestation_currentness.v1'
         AS currentness_schema,
       receipt.reattestation_receipt_id,
       receipt.reattestation_receipt_digest,
       receipt.registry_release_id,
       receipt.registry_release_digest,
       receipt.sequence,
       receipt.predecessor_receipt_id,
       receipt.predecessor_receipt_digest,
       CASE WHEN successor.reattestation_receipt_id IS NULL THEN 'head'
            ELSE 'superseded' END AS head_status,
       CASE WHEN registry.current_status='release_current' THEN 'release_current'
            ELSE 'historical_only' END AS registry_release_status,
       CASE WHEN vulnerability.current_status='verified_current' THEN 'verified_current'
            ELSE 'historical_only' END AS vulnerability_reattestation_status,
       CASE WHEN verifier.current_status='active' THEN 'active' ELSE 'revoked' END
         AS sandbox_verifier_key_status,
       CASE WHEN julianday(receipt.verified_at)<=julianday('now')
                  AND julianday(receipt.report_expires_at)>julianday('now') THEN 'current'
            ELSE 'expired' END AS report_validity_status,
       CASE WHEN revocation.revocation_receipt_id IS NULL THEN 'none' ELSE 'revoked' END
         AS revocation_status,
       CASE WHEN successor.reattestation_receipt_id IS NULL
                  AND registry.current_status='release_current'
                  AND vulnerability.current_status='verified_current'
                  AND verifier.current_status='active'
                  AND julianday(receipt.verified_at)<=julianday('now')
                  AND julianday(receipt.report_expires_at)>julianday('now')
                  AND revocation.revocation_receipt_id IS NULL
            THEN 'verified_current' ELSE 'historical_only' END AS current_status,
       receipt.report_expires_at,
       receipt.verified_at
  FROM compute_external_pool_adapter_sandbox_reattestation_receipts receipt
  LEFT JOIN compute_external_pool_adapter_sandbox_reattestation_receipts successor
    ON successor.predecessor_receipt_id=receipt.reattestation_receipt_id
   AND successor.predecessor_receipt_digest=receipt.reattestation_receipt_digest
  LEFT JOIN compute_external_pool_adapter_registry_release_current registry
    ON registry.registry_release_id=receipt.registry_release_id
   AND registry.registry_release_digest=receipt.registry_release_digest
  LEFT JOIN compute_external_pool_adapter_vulnerability_reattestation_current vulnerability
    ON vulnerability.reattestation_receipt_id=receipt.vulnerability_reattestation_receipt_id
   AND vulnerability.reattestation_receipt_digest=receipt.vulnerability_reattestation_receipt_digest
   AND vulnerability.registry_release_id=receipt.registry_release_id
   AND vulnerability.registry_release_digest=receipt.registry_release_digest
  LEFT JOIN compute_external_pool_adapter_sandbox_verifier_key_current verifier
    ON verifier.key_record_id=receipt.sandbox_verifier_key_record_id
   AND verifier.key_record_digest=receipt.sandbox_verifier_key_record_digest
   AND verifier.key_id=receipt.sandbox_verifier_key_id
  LEFT JOIN compute_external_pool_adapter_sandbox_reattestation_revocations revocation
    ON revocation.reattestation_receipt_id=receipt.reattestation_receipt_id
   AND revocation.reattestation_receipt_digest=receipt.reattestation_receipt_digest;
