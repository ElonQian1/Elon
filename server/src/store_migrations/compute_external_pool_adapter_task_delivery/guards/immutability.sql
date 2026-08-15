CREATE TRIGGER IF NOT EXISTS v273_task_exchange_attempts_no_update
BEFORE UPDATE ON compute_external_pool_adapter_task_exchange_attempts
BEGIN SELECT RAISE(ABORT,'V273 exchange attempts are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v273_task_exchange_attempts_no_delete
BEFORE DELETE ON compute_external_pool_adapter_task_exchange_attempts
BEGIN SELECT RAISE(ABORT,'V273 exchange attempts are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v273_task_exchange_receipts_no_update
BEFORE UPDATE ON compute_external_pool_adapter_task_exchange_receipts
BEGIN SELECT RAISE(ABORT,'V273 exchange receipts are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v273_task_exchange_receipts_no_delete
BEFORE DELETE ON compute_external_pool_adapter_task_exchange_receipts
BEGIN SELECT RAISE(ABORT,'V273 exchange receipts are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v273_task_event_batches_no_update
BEFORE UPDATE ON compute_external_pool_adapter_task_event_batches
BEGIN SELECT RAISE(ABORT,'V273 event batches are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v273_task_event_batches_no_delete
BEFORE DELETE ON compute_external_pool_adapter_task_event_batches
BEGIN SELECT RAISE(ABORT,'V273 event batches are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v273_task_events_no_update
BEFORE UPDATE ON compute_external_pool_adapter_task_events
BEGIN SELECT RAISE(ABORT,'V273 events are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v273_task_events_no_delete
BEFORE DELETE ON compute_external_pool_adapter_task_events
BEGIN SELECT RAISE(ABORT,'V273 events are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v273_task_reconcile_polls_no_delete
BEFORE DELETE ON compute_external_pool_adapter_task_reconcile_polls
BEGIN SELECT RAISE(ABORT,'V273 reconcile polls cannot be deleted'); END;
CREATE TRIGGER IF NOT EXISTS v273_task_event_polls_no_delete
BEFORE DELETE ON compute_external_pool_adapter_task_event_polls
BEGIN SELECT RAISE(ABORT,'V273 event polls cannot be deleted'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_reconcile_poll_intent_immutable
BEFORE UPDATE ON compute_external_pool_adapter_task_reconcile_polls
WHEN NEW.reconcile_poll_id IS NOT OLD.reconcile_poll_id OR NEW.reconcile_poll_schema IS NOT OLD.reconcile_poll_schema OR NEW.reconcile_poll_digest IS NOT OLD.reconcile_poll_digest OR NEW.reconcile_poll_json IS NOT OLD.reconcile_poll_json OR NEW.canonicalization IS NOT OLD.canonicalization OR NEW.digest_algorithm IS NOT OLD.digest_algorithm OR NEW.predecessor_reconcile_poll_id IS NOT OLD.predecessor_reconcile_poll_id OR NEW.predecessor_reconcile_poll_digest IS NOT OLD.predecessor_reconcile_poll_digest OR NEW.poll_ordinal IS NOT OLD.poll_ordinal OR NEW.uncertain_exchange_attempt_id IS NOT OLD.uncertain_exchange_attempt_id OR NEW.uncertain_exchange_attempt_digest IS NOT OLD.uncertain_exchange_attempt_digest OR NEW.command_id IS NOT OLD.command_id OR NEW.command_digest IS NOT OLD.command_digest OR NEW.outbox_id IS NOT OLD.outbox_id OR NEW.outbox_digest IS NOT OLD.outbox_digest OR NEW.send_attempt_id IS NOT OLD.send_attempt_id OR NEW.send_attempt_digest IS NOT OLD.send_attempt_digest OR NEW.route_authorization_id IS NOT OLD.route_authorization_id OR NEW.route_authorization_digest IS NOT OLD.route_authorization_digest OR NEW.executor_binding_digest IS NOT OLD.executor_binding_digest OR NEW.fencing_generation IS NOT OLD.fencing_generation OR NEW.fence_digest IS NOT OLD.fence_digest OR NEW.remote_execution_id IS NOT OLD.remote_execution_id OR NEW.remote_identity_digest IS NOT OLD.remote_identity_digest OR NEW.remote_execution_state IS NOT OLD.remote_execution_state OR NEW.authenticated_subject_sha256 IS NOT OLD.authenticated_subject_sha256 OR NEW.request_digest IS NOT OLD.request_digest OR NEW.not_before IS NOT OLD.not_before OR NEW.not_after IS NOT OLD.not_after OR NEW.created_at IS NOT OLD.created_at OR NEW.authority_status IS NOT OLD.authority_status OR NEW.effects_json IS NOT OLD.effects_json OR NEW.readiness_json IS NOT OLD.readiness_json
BEGIN SELECT RAISE(ABORT,'V273 reconcile poll intent is immutable'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_event_poll_intent_immutable
BEFORE UPDATE ON compute_external_pool_adapter_task_event_polls
WHEN NEW.event_poll_id IS NOT OLD.event_poll_id OR NEW.event_poll_schema IS NOT OLD.event_poll_schema OR NEW.event_poll_digest IS NOT OLD.event_poll_digest OR NEW.event_poll_json IS NOT OLD.event_poll_json OR NEW.canonicalization IS NOT OLD.canonicalization OR NEW.digest_algorithm IS NOT OLD.digest_algorithm OR NEW.predecessor_event_poll_id IS NOT OLD.predecessor_event_poll_id OR NEW.predecessor_event_poll_digest IS NOT OLD.predecessor_event_poll_digest OR NEW.poll_ordinal IS NOT OLD.poll_ordinal OR NEW.source_exchange_receipt_id IS NOT OLD.source_exchange_receipt_id OR NEW.source_exchange_receipt_digest IS NOT OLD.source_exchange_receipt_digest OR NEW.command_id IS NOT OLD.command_id OR NEW.command_digest IS NOT OLD.command_digest OR NEW.outbox_id IS NOT OLD.outbox_id OR NEW.outbox_digest IS NOT OLD.outbox_digest OR NEW.send_attempt_id IS NOT OLD.send_attempt_id OR NEW.send_attempt_digest IS NOT OLD.send_attempt_digest OR NEW.route_authorization_id IS NOT OLD.route_authorization_id OR NEW.route_authorization_digest IS NOT OLD.route_authorization_digest OR NEW.executor_binding_digest IS NOT OLD.executor_binding_digest OR NEW.fencing_generation IS NOT OLD.fencing_generation OR NEW.fence_digest IS NOT OLD.fence_digest OR NEW.remote_execution_id IS NOT OLD.remote_execution_id OR NEW.remote_identity_digest IS NOT OLD.remote_identity_digest OR NEW.remote_execution_state IS NOT OLD.remote_execution_state OR NEW.authenticated_subject_sha256 IS NOT OLD.authenticated_subject_sha256 OR NEW.requested_remote_sequence IS NOT OLD.requested_remote_sequence OR NEW.requested_previous_event_root IS NOT OLD.requested_previous_event_root OR NEW.requested_cursor_digest IS NOT OLD.requested_cursor_digest OR NEW.request_digest IS NOT OLD.request_digest OR NEW.not_before IS NOT OLD.not_before OR NEW.not_after IS NOT OLD.not_after OR NEW.created_at IS NOT OLD.created_at OR NEW.authority_status IS NOT OLD.authority_status OR NEW.effects_json IS NOT OLD.effects_json OR NEW.readiness_json IS NOT OLD.readiness_json
BEGIN SELECT RAISE(ABORT,'V273 event poll intent is immutable'); END;
