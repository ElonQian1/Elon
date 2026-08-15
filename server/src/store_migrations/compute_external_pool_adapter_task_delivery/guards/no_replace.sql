CREATE TRIGGER IF NOT EXISTS v273_task_exchange_attempt_no_replace
BEFORE INSERT ON compute_external_pool_adapter_task_exchange_attempts
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts old
   WHERE old.exchange_attempt_id=NEW.exchange_attempt_id
      OR old.exchange_attempt_digest=NEW.exchange_attempt_digest
      OR (old.source_kind=NEW.source_kind AND old.source_id=NEW.source_id)
      OR old.delivery_attempt_digest=NEW.delivery_attempt_digest)
BEGIN SELECT RAISE(ABORT,'V273 exchange attempt replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_exchange_receipt_no_replace
BEFORE INSERT ON compute_external_pool_adapter_task_exchange_receipts
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts old
   WHERE old.exchange_receipt_id=NEW.exchange_receipt_id
      OR old.exchange_receipt_digest=NEW.exchange_receipt_digest
      OR old.exchange_attempt_id=NEW.exchange_attempt_id
      OR old.exchange_root=NEW.exchange_root)
BEGIN SELECT RAISE(ABORT,'V273 exchange receipt replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_reconcile_poll_no_replace
BEFORE INSERT ON compute_external_pool_adapter_task_reconcile_polls
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_reconcile_polls old
   WHERE old.reconcile_poll_id=NEW.reconcile_poll_id
      OR old.reconcile_poll_digest=NEW.reconcile_poll_digest
      OR (old.uncertain_exchange_attempt_id=NEW.uncertain_exchange_attempt_id
          AND old.poll_ordinal=NEW.poll_ordinal)
      OR old.predecessor_reconcile_poll_id=NEW.predecessor_reconcile_poll_id)
BEGIN SELECT RAISE(ABORT,'V273 reconcile poll replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_event_poll_no_replace
BEFORE INSERT ON compute_external_pool_adapter_task_event_polls
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_event_polls old
   WHERE old.event_poll_id=NEW.event_poll_id
      OR old.event_poll_digest=NEW.event_poll_digest
      OR (old.remote_identity_digest=NEW.remote_identity_digest
          AND old.requested_cursor_digest=NEW.requested_cursor_digest
          AND old.poll_ordinal=NEW.poll_ordinal)
      OR old.predecessor_event_poll_id=NEW.predecessor_event_poll_id)
BEGIN SELECT RAISE(ABORT,'V273 event poll replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_event_batch_no_replace
BEFORE INSERT ON compute_external_pool_adapter_task_event_batches
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_event_batches old
   WHERE old.event_batch_id=NEW.event_batch_id
      OR old.event_batch_digest=NEW.event_batch_digest
      OR old.event_poll_id=NEW.event_poll_id
      OR old.batch_root=NEW.batch_root
      OR old.predecessor_event_batch_id=NEW.predecessor_event_batch_id)
BEGIN SELECT RAISE(ABORT,'V273 event batch replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_event_no_replace
BEFORE INSERT ON compute_external_pool_adapter_task_events
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_events old
   WHERE old.event_id=NEW.event_id
      OR old.event_digest=NEW.event_digest
      OR old.event_root=NEW.event_root
      OR (old.event_batch_id=NEW.event_batch_id AND old.event_ordinal=NEW.event_ordinal)
      OR (old.event_batch_id=NEW.event_batch_id AND old.remote_event_id=NEW.remote_event_id)
      OR (old.event_batch_id=NEW.event_batch_id AND old.remote_sequence=NEW.remote_sequence))
BEGIN SELECT RAISE(ABORT,'V273 event replacement is forbidden'); END;
