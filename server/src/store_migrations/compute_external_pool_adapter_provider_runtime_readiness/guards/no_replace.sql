CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_receipt_no_replace
BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_receipts
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_receipts old
   WHERE old.readiness_receipt_id=NEW.readiness_receipt_id
      OR old.readiness_receipt_digest=NEW.readiness_receipt_digest
      OR old.probe_execution_id=NEW.probe_execution_id
      OR (old.provider_binding_id=NEW.provider_binding_id AND old.sequence=NEW.sequence)
      OR (NEW.predecessor_readiness_receipt_id IS NOT NULL
          AND old.predecessor_readiness_receipt_id=NEW.predecessor_readiness_receipt_id)
      OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)
)
BEGIN SELECT RAISE(ABORT,'V270 readiness receipt replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_revocation_no_replace
BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_revocations
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_revocations old
   WHERE old.revocation_receipt_id=NEW.revocation_receipt_id
      OR old.revocation_receipt_digest=NEW.revocation_receipt_digest
      OR old.readiness_receipt_id=NEW.readiness_receipt_id
      OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)
)
BEGIN SELECT RAISE(ABORT,'V270 readiness revocation replacement is forbidden'); END;
