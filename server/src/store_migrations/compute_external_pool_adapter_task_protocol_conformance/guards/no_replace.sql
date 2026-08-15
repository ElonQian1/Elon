CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_run_no_replace
BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_run_receipts
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts old
   WHERE old.run_receipt_id=NEW.run_receipt_id
      OR old.run_receipt_digest=NEW.run_receipt_digest
      OR old.run_material_digest=NEW.run_material_digest
      OR old.run_nonce_digest=NEW.run_nonce_digest
      OR old.process_hmac_seal=NEW.process_hmac_seal
      OR old.receipt_integrity_digest=NEW.receipt_integrity_digest
      OR (old.registry_release_id=NEW.registry_release_id AND old.sequence=NEW.sequence)
      OR (NEW.predecessor_run_receipt_id IS NOT NULL
          AND old.predecessor_run_receipt_id=NEW.predecessor_run_receipt_id)
      OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)
)
BEGIN SELECT RAISE(ABORT,'V272 task protocol conformance run replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_revocation_no_replace
BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_revocations
WHEN EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_revocations old
   WHERE old.revocation_receipt_id=NEW.revocation_receipt_id
      OR old.revocation_receipt_digest=NEW.revocation_receipt_digest
      OR old.revocation_material_digest=NEW.revocation_material_digest
      OR old.run_receipt_id=NEW.run_receipt_id
      OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)
)
BEGIN SELECT RAISE(ABORT,'V272 task protocol conformance revocation replacement is forbidden'); END;
