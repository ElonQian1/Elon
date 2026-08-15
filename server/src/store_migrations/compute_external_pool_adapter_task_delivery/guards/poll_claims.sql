CREATE TRIGGER IF NOT EXISTS v273_task_reconcile_poll_initial_claim
BEFORE INSERT ON compute_external_pool_adapter_task_reconcile_polls
WHEN NEW.claim_status<>'pending' OR NEW.claim_revision<>1 OR NEW.claim_generation<>0 OR NEW.claim_owner_id IS NOT NULL OR NEW.claim_token_digest IS NOT NULL OR NEW.claim_expires_at IS NOT NULL
BEGIN SELECT RAISE(ABORT,'V273 reconcile poll claim projection must start pending'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_event_poll_initial_claim
BEFORE INSERT ON compute_external_pool_adapter_task_event_polls
WHEN NEW.claim_status<>'pending' OR NEW.claim_revision<>1 OR NEW.claim_generation<>0 OR NEW.claim_owner_id IS NOT NULL OR NEW.claim_token_digest IS NOT NULL OR NEW.claim_expires_at IS NOT NULL
BEGIN SELECT RAISE(ABORT,'V273 event poll claim projection must start pending'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_reconcile_poll_claim_cas
BEFORE UPDATE ON compute_external_pool_adapter_task_reconcile_polls
WHEN NEW.claim_revision<>OLD.claim_revision+1 OR NOT (
  (OLD.claim_status='pending' AND NEW.claim_status='claimed' AND NEW.claim_generation=OLD.claim_generation+1 AND NEW.claim_owner_id IS NOT NULL AND NEW.claim_token_digest IS NOT NULL AND NEW.claim_expires_at IS NOT NULL)
  OR (OLD.claim_status='claimed' AND OLD.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now') AND NEW.claim_status='pending' AND NEW.claim_generation=OLD.claim_generation AND NEW.claim_owner_id IS NULL AND NEW.claim_token_digest IS NULL AND NEW.claim_expires_at IS NULL AND NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt WHERE attempt.source_kind='reconcile_poll' AND attempt.source_id=OLD.reconcile_poll_id AND attempt.source_digest=OLD.reconcile_poll_digest))
  OR (OLD.claim_status='claimed' AND NEW.claim_status='in_flight_unknown' AND NEW.claim_generation=OLD.claim_generation AND NEW.claim_owner_id IS NULL AND NEW.claim_token_digest IS NULL AND NEW.claim_expires_at IS NULL AND EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt WHERE attempt.source_kind='reconcile_poll' AND attempt.source_id=OLD.reconcile_poll_id AND attempt.source_digest=OLD.reconcile_poll_digest AND NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest)))
  OR (OLD.claim_status='claimed' AND NEW.claim_status='quarantined' AND NEW.claim_generation=OLD.claim_generation AND NEW.claim_owner_id IS NULL AND NEW.claim_token_digest IS NULL AND NEW.claim_expires_at IS NULL)
  OR (OLD.claim_status='claimed' AND NEW.claim_status='delivery_observed' AND NEW.claim_generation=OLD.claim_generation AND NEW.claim_owner_id IS NULL AND NEW.claim_token_digest IS NULL AND NEW.claim_expires_at IS NULL AND EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt WHERE receipt.source_kind='reconcile_poll' AND receipt.source_id=OLD.reconcile_poll_id AND receipt.source_digest=OLD.reconcile_poll_digest))
)
BEGIN SELECT RAISE(ABORT,'V273 reconcile poll claim update is not exact CAS'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_event_poll_claim_cas
BEFORE UPDATE ON compute_external_pool_adapter_task_event_polls
WHEN NEW.claim_revision<>OLD.claim_revision+1 OR NOT (
  (OLD.claim_status='pending' AND NEW.claim_status='claimed' AND NEW.claim_generation=OLD.claim_generation+1 AND NEW.claim_owner_id IS NOT NULL AND NEW.claim_token_digest IS NOT NULL AND NEW.claim_expires_at IS NOT NULL)
  OR (OLD.claim_status='claimed' AND OLD.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now') AND NEW.claim_status='pending' AND NEW.claim_generation=OLD.claim_generation AND NEW.claim_owner_id IS NULL AND NEW.claim_token_digest IS NULL AND NEW.claim_expires_at IS NULL AND NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt WHERE attempt.source_kind='event_poll' AND attempt.source_id=OLD.event_poll_id AND attempt.source_digest=OLD.event_poll_digest))
  OR (OLD.claim_status='claimed' AND NEW.claim_status='in_flight_unknown' AND NEW.claim_generation=OLD.claim_generation AND NEW.claim_owner_id IS NULL AND NEW.claim_token_digest IS NULL AND NEW.claim_expires_at IS NULL AND EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt WHERE attempt.source_kind='event_poll' AND attempt.source_id=OLD.event_poll_id AND attempt.source_digest=OLD.event_poll_digest AND NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest)))
  OR (OLD.claim_status='claimed' AND NEW.claim_status='quarantined' AND NEW.claim_generation=OLD.claim_generation AND NEW.claim_owner_id IS NULL AND NEW.claim_token_digest IS NULL AND NEW.claim_expires_at IS NULL)
  OR (OLD.claim_status='claimed' AND NEW.claim_status='delivery_observed' AND NEW.claim_generation=OLD.claim_generation AND NEW.claim_owner_id IS NULL AND NEW.claim_token_digest IS NULL AND NEW.claim_expires_at IS NULL AND EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_event_batches batch WHERE batch.event_poll_id=OLD.event_poll_id AND batch.event_poll_digest=OLD.event_poll_digest AND (SELECT count(*) FROM compute_external_pool_adapter_task_events event WHERE event.event_batch_id=batch.event_batch_id)=batch.event_count AND COALESCE((SELECT max(event.event_ordinal) FROM compute_external_pool_adapter_task_events event WHERE event.event_batch_id=batch.event_batch_id),0)=batch.event_count))
)
BEGIN SELECT RAISE(ABORT,'V273 event poll claim update is not exact CAS'); END;
