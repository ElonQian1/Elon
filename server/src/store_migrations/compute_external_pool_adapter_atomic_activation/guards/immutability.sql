CREATE TRIGGER IF NOT EXISTS v277_atomic_activation_receipt_no_replace
BEFORE INSERT ON compute_external_pool_adapter_atomic_activation_receipts
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_atomic_activation_receipts old
   WHERE old.activation_receipt_id=NEW.activation_receipt_id
      OR old.activation_receipt_digest=NEW.activation_receipt_digest
      OR (old.provider_binding_id=NEW.provider_binding_id
          AND old.activation_root_digest=NEW.activation_root_digest)
      OR old.executor_id=NEW.executor_id
      OR old.stable_executor_binding_digest=NEW.stable_executor_binding_digest
      OR old.idempotency_digest=NEW.idempotency_digest
)
BEGIN SELECT RAISE(ABORT,'V277 atomic activation INSERT OR REPLACE is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v277_atomic_activation_receipt_no_update
BEFORE UPDATE ON compute_external_pool_adapter_atomic_activation_receipts
BEGIN SELECT RAISE(ABORT,'V277 atomic activation receipts are immutable'); END;

CREATE TRIGGER IF NOT EXISTS v277_atomic_activation_receipt_no_delete
BEFORE DELETE ON compute_external_pool_adapter_atomic_activation_receipts
BEGIN SELECT RAISE(ABORT,'V277 atomic activation receipts are immutable'); END;
