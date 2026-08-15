CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_receipt_lineage
BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_receipts
WHEN NOT (
  (NEW.sequence=1
   AND NEW.predecessor_readiness_receipt_id IS NULL
   AND NEW.predecessor_readiness_receipt_digest IS NULL
   AND NOT EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_receipts existing
      WHERE existing.provider_binding_id=NEW.provider_binding_id))
  OR
  (NEW.sequence>1
   AND EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_receipts predecessor
      WHERE predecessor.readiness_receipt_id=NEW.predecessor_readiness_receipt_id
        AND predecessor.readiness_receipt_digest=NEW.predecessor_readiness_receipt_digest
        AND predecessor.provider_binding_id=NEW.provider_binding_id
        AND predecessor.sequence=NEW.sequence-1
        AND predecessor.recorded_at<=NEW.recorded_at
        AND NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_receipts successor
           WHERE successor.predecessor_readiness_receipt_id=predecessor.readiness_receipt_id)))
)
BEGIN SELECT RAISE(ABORT,'V270 readiness requires the exact structural binding head'); END;

CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_receipt_actor_and_window
BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_receipts
WHEN NEW.idempotency_scope<>(
       'v270:provider-runtime-readiness:create:'||NEW.recorded_by_actor_user_id)
 OR NOT EXISTS (
      SELECT 1 FROM users actor
       WHERE actor.id=NEW.recorded_by_actor_user_id
         AND actor.role IN ('admin','owner')
         AND actor.status='active')
 OR julianday(NEW.checked_at)>julianday('now','+1 second')
 OR julianday(NEW.expires_at)<=julianday('now')
BEGIN SELECT RAISE(ABORT,'V270 readiness actor or short observation window is invalid'); END;

CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_revocation_lineage
BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_revocations
WHEN NOT EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_receipts receipt
   WHERE receipt.readiness_receipt_id=NEW.readiness_receipt_id
     AND receipt.readiness_receipt_digest=NEW.readiness_receipt_digest
     AND receipt.provider_binding_id=NEW.provider_binding_id
     AND receipt.provider_binding_digest=NEW.provider_binding_digest
     AND receipt.candidate_id=NEW.candidate_id
     AND receipt.candidate_digest=NEW.candidate_digest
     AND receipt.profile_id=NEW.profile_id
     AND receipt.profile_digest=NEW.profile_digest
     AND receipt.target_id=NEW.target_id
     AND receipt.target_digest=NEW.target_digest
     AND receipt.companion_id=NEW.companion_id
     AND receipt.companion_digest=NEW.companion_digest
     AND receipt.provider_id=NEW.provider_id
     AND receipt.recorded_at<=NEW.revoked_at
     AND NOT EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_receipts successor
        WHERE successor.predecessor_readiness_receipt_id=receipt.readiness_receipt_id)
     AND NOT EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_revocations prior
        WHERE prior.readiness_receipt_id=receipt.readiness_receipt_id)
)
BEGIN SELECT RAISE(ABORT,'V270 revocation requires the exact unrevoked structural binding head'); END;

CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_revocation_actor
BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_revocations
WHEN NEW.idempotency_scope<>(
       'v270:provider-runtime-readiness:revoke:'||NEW.revoked_by_actor_kind||':'||NEW.revoked_by_actor_user_id)
 OR abs((julianday(NEW.revoked_at)-julianday('now'))*86400.0)>60.0
 OR NOT EXISTS (
      SELECT 1
        FROM compute_external_pool_adapter_provider_runtime_readiness_receipts receipt
        JOIN compute_external_pool_adapter_supervisor_session_policy_companions companion
          ON companion.companion_id=receipt.companion_id
         AND companion.companion_digest=receipt.companion_digest
       WHERE receipt.readiness_receipt_id=NEW.readiness_receipt_id
         AND receipt.readiness_receipt_digest=NEW.readiness_receipt_digest
          AND ((NEW.revoked_by_actor_kind='provider_owner'
                AND NEW.revoked_by_actor_user_id=companion.provider_owner_account_id
                AND EXISTS (
                  SELECT 1 FROM users actor
                   WHERE actor.id=NEW.revoked_by_actor_user_id
                     AND actor.status='active'))
              OR
              (NEW.revoked_by_actor_kind='platform_admin'
               AND EXISTS (
                 SELECT 1 FROM users actor
                  WHERE actor.id=NEW.revoked_by_actor_user_id
                    AND actor.role IN ('admin','owner')
                    AND actor.status='active'))))
BEGIN SELECT RAISE(ABORT,'V270 readiness revocation actor is unauthorized'); END;
