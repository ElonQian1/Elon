CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_task_protocol_roots
BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_receipts
WHEN NOT EXISTS (
  SELECT 1
    FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts run
   WHERE run.run_receipt_id=NEW.task_protocol_conformance_run_receipt_id
     AND run.run_receipt_digest=NEW.task_protocol_conformance_run_receipt_digest
     AND run.registry_release_id=NEW.registry_release_id
     AND run.registry_release_digest=NEW.registry_release_digest
     AND run.task_protocol_profile_digest=NEW.task_protocol_profile_digest
     AND run.launch_image_sha256=NEW.launch_image_sha256
     AND run.expires_at=NEW.task_protocol_conformance_expires_at
     AND run.post_cleanup_checked_at<=NEW.evidence_checked_at
     AND NEW.evidence_checked_at<run.expires_at
     AND NEW.observation_expires_at<=run.expires_at
     AND NOT EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts successor
        WHERE successor.predecessor_run_receipt_id=run.run_receipt_id)
     AND NOT EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_revocations revoked
        WHERE revoked.run_receipt_id=run.run_receipt_id
          AND revoked.run_receipt_digest=run.run_receipt_digest))
BEGIN SELECT RAISE(ABORT,'V274 active successor lacks exact fresh V272 evidence'); END;

CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_revocation_time
BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_revocations
WHEN NOT EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_provider_active_successor_receipts target
   WHERE target.active_successor_receipt_id=NEW.target_active_successor_receipt_id
     AND target.receipt_digest=NEW.target_active_successor_receipt_digest
     AND NEW.revoked_at>=target.created_at
     AND NEW.revoked_at<target.observation_expires_at
     AND julianday(NEW.revoked_at)<=julianday('now')
     AND (julianday('now')-julianday(NEW.revoked_at))*86400.0<=15.000001)
BEGIN SELECT RAISE(ABORT,'V274 active successor revocation insertion window is invalid'); END;
