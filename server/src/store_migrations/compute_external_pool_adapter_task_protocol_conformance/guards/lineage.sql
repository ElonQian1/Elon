CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_run_lineage
BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_run_receipts
WHEN NOT (
  (NEW.sequence=1
   AND NEW.predecessor_run_receipt_id IS NULL
   AND NEW.predecessor_run_receipt_digest IS NULL
   AND NOT EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts existing
      WHERE existing.registry_release_id=NEW.registry_release_id))
  OR
  (NEW.sequence>1
   AND EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts predecessor
      WHERE predecessor.run_receipt_id=NEW.predecessor_run_receipt_id
        AND predecessor.run_receipt_digest=NEW.predecessor_run_receipt_digest
        AND predecessor.registry_release_id=NEW.registry_release_id
        AND predecessor.registry_release_digest=NEW.registry_release_digest
        AND predecessor.sequence=NEW.sequence-1
        AND predecessor.recorded_at<=NEW.recorded_at
        AND NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts successor
           WHERE successor.predecessor_run_receipt_id=predecessor.run_receipt_id)))
)
BEGIN SELECT RAISE(ABORT,'V272 task protocol run requires the exact structural release head'); END;

CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_run_actor_and_window
BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_run_receipts
WHEN NEW.idempotency_scope<>(
       'v272:task-protocol-conformance:create:'||NEW.recorded_by_admin_user_id)
 OR NOT EXISTS (
      SELECT 1 FROM users actor
       WHERE actor.id=NEW.recorded_by_admin_user_id
         AND actor.role IN ('admin','owner')
         AND actor.status='active')
 OR julianday(NEW.post_cleanup_checked_at)>julianday('now','+1 second')
 OR julianday(NEW.expires_at)<=julianday('now')
BEGIN SELECT RAISE(ABORT,'V272 task protocol run actor or short observation window is invalid'); END;

CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_revocation_lineage
BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_revocations
WHEN NOT EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts receipt
   WHERE receipt.run_receipt_id=NEW.run_receipt_id
     AND receipt.run_receipt_digest=NEW.run_receipt_digest
     AND receipt.registry_release_id=NEW.registry_release_id
     AND receipt.registry_release_digest=NEW.registry_release_digest
     AND receipt.recorded_at<=NEW.revoked_at
     AND NOT EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts successor
        WHERE successor.predecessor_run_receipt_id=receipt.run_receipt_id)
     AND NOT EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_revocations prior
        WHERE prior.run_receipt_id=receipt.run_receipt_id)
)
BEGIN SELECT RAISE(ABORT,'V272 task protocol revocation requires the exact unrevoked structural head'); END;

CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_revocation_actor
BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_revocations
WHEN NEW.idempotency_scope<>(
       'v272:task-protocol-conformance:revoke:'||NEW.revoked_by_admin_user_id)
 OR abs((julianday(NEW.revoked_at)-julianday('now'))*86400.0)>60.0
 OR NOT EXISTS (
      SELECT 1 FROM users actor
       WHERE actor.id=NEW.revoked_by_admin_user_id
         AND actor.role IN ('admin','owner')
         AND actor.status='active')
BEGIN SELECT RAISE(ABORT,'V272 task protocol revocation actor is unauthorized'); END;
