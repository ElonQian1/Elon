DROP VIEW IF EXISTS compute_external_pool_adapter_provider_active_successor_current;

CREATE VIEW compute_external_pool_adapter_provider_active_successor_current AS
SELECT 'compute_federation.external_pool_adapter_provider_active_successor_currentness.v1'
         AS currentness_schema,
       receipt.active_successor_receipt_id,
       receipt.receipt_digest,
       receipt.provider_binding_id,
       receipt.activation_root_digest,
       receipt.successor_sequence,
       receipt.predecessor_active_successor_receipt_id,
       receipt.predecessor_active_successor_receipt_digest,
       receipt.evidence_provider_id,
       receipt.evidence_provider_policy_revision,
       receipt.evidence_provider_digest,
       receipt.checked_at,
       receipt.observation_expires_at,
       CASE WHEN successor.active_successor_receipt_id IS NULL THEN 'head'
            ELSE 'historical' END AS head_status,
       CASE WHEN revocation.active_successor_revocation_id IS NULL THEN 'unrevoked'
            ELSE 'revoked' END AS revocation_status,
       CASE WHEN provider.provider_id IS NOT NULL
                  AND provider.provider_kind='external_pool'
                  AND provider.status='active'
                  AND provider.current_policy_revision=receipt.evidence_provider_policy_revision
                  AND provider.current_provider_digest=receipt.evidence_provider_digest
                  AND version.provider_json=receipt.evidence_provider_json
                  AND json_extract(version.provider_json,'$.adapter.adapter_id')=receipt.route_adapter_projection_id
            THEN 'projected_active_exact' ELSE 'provider_drifted' END AS provider_status,
       CASE WHEN julianday(receipt.checked_at)<=julianday('now')
                  AND julianday(receipt.observation_expires_at)>julianday('now')
            THEN 'unexpired' ELSE 'expired' END AS expiry_status,
       CASE WHEN successor.active_successor_receipt_id IS NULL
                  AND revocation.active_successor_revocation_id IS NULL
                  AND provider.provider_id IS NOT NULL
                  AND provider.provider_kind='external_pool'
                  AND provider.status='active'
                  AND provider.current_policy_revision=receipt.evidence_provider_policy_revision
                  AND provider.current_provider_digest=receipt.evidence_provider_digest
                  AND version.provider_json=receipt.evidence_provider_json
                  AND json_extract(version.provider_json,'$.adapter.adapter_id')=receipt.route_adapter_projection_id
                  AND julianday(receipt.checked_at)<=julianday('now')
                  AND julianday(receipt.observation_expires_at)>julianday('now')
            THEN 'relationally_current_requires_process_custody_and_active_root_reproof'
            ELSE 'historical_only' END AS current_status
  FROM compute_external_pool_adapter_provider_active_successor_receipts receipt
  LEFT JOIN compute_external_pool_adapter_provider_active_successor_receipts successor
    ON successor.predecessor_active_successor_receipt_id=receipt.active_successor_receipt_id
   AND successor.predecessor_active_successor_receipt_digest=receipt.receipt_digest
  LEFT JOIN compute_external_pool_adapter_provider_active_successor_revocations revocation
    ON revocation.target_active_successor_receipt_id=receipt.active_successor_receipt_id
   AND revocation.target_active_successor_receipt_digest=receipt.receipt_digest
  LEFT JOIN compute_providers provider
    ON provider.provider_id=receipt.evidence_provider_id
  LEFT JOIN compute_provider_versions version
    ON version.provider_id=provider.provider_id
   AND version.policy_revision=provider.current_policy_revision
   AND version.provider_digest=provider.current_provider_digest;
