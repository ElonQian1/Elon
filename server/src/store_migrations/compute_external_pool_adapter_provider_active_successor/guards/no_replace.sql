CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_receipt_no_replace
BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_receipts
WHEN EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_provider_active_successor_receipts old
        WHERE old.active_successor_receipt_id=NEW.active_successor_receipt_id
           OR old.receipt_digest=NEW.receipt_digest
           OR (old.provider_binding_id=NEW.provider_binding_id
               AND old.activation_root_digest=NEW.activation_root_digest
               AND old.successor_sequence=NEW.successor_sequence)
           OR old.predecessor_active_successor_receipt_id=NEW.predecessor_active_successor_receipt_id
           OR old.runtime_observation_id=NEW.runtime_observation_id
           OR old.runtime_observation_digest=NEW.runtime_observation_digest
           OR old.process_custody_nonce_digest=NEW.process_custody_nonce_digest
           OR old.process_custody_seal_digest=NEW.process_custody_seal_digest
           OR old.receipt_integrity_digest=NEW.receipt_integrity_digest)
BEGIN SELECT RAISE(ABORT,'V274 active successor INSERT OR REPLACE is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_revocation_no_replace
BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_revocations
WHEN EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_provider_active_successor_revocations old
        WHERE old.active_successor_revocation_id=NEW.active_successor_revocation_id
           OR old.revocation_digest=NEW.revocation_digest
           OR old.target_active_successor_receipt_id=NEW.target_active_successor_receipt_id
           OR old.idempotency_digest=NEW.idempotency_digest
           OR old.process_custody_nonce_digest=NEW.process_custody_nonce_digest
           OR old.process_custody_seal_digest=NEW.process_custody_seal_digest
           OR old.receipt_integrity_digest=NEW.receipt_integrity_digest)
BEGIN SELECT RAISE(ABORT,'V274 active successor revocation INSERT OR REPLACE is forbidden'); END;
