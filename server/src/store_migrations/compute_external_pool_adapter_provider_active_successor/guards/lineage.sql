CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_lineage
BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_receipts
WHEN (NEW.successor_sequence=1 AND EXISTS (
        SELECT 1 FROM compute_external_pool_adapter_provider_active_successor_receipts old
         WHERE old.provider_binding_id=NEW.provider_binding_id
           AND old.activation_root_digest=NEW.activation_root_digest))
 OR (NEW.successor_sequence>1 AND NOT EXISTS (
        SELECT 1 FROM compute_external_pool_adapter_provider_active_successor_receipts predecessor
         WHERE predecessor.active_successor_receipt_id=NEW.predecessor_active_successor_receipt_id
           AND predecessor.receipt_digest=NEW.predecessor_active_successor_receipt_digest
           AND predecessor.provider_binding_id=NEW.provider_binding_id
           AND predecessor.activation_root_digest=NEW.activation_root_digest
           AND predecessor.successor_sequence+1=NEW.successor_sequence
           AND predecessor.evidence_provider_policy_revision<=NEW.evidence_provider_policy_revision
           AND predecessor.checked_at<NEW.checked_at
           AND NOT EXISTS (
             SELECT 1 FROM compute_external_pool_adapter_provider_active_successor_receipts successor
              WHERE successor.predecessor_active_successor_receipt_id=predecessor.active_successor_receipt_id)))
BEGIN SELECT RAISE(ABORT,'V274 active successor lineage is not the exact structural head'); END;

CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_revocation_lineage
BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_revocations
WHEN NOT EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_provider_active_successor_receipts target
        WHERE target.active_successor_receipt_id=NEW.target_active_successor_receipt_id
          AND target.receipt_digest=NEW.target_active_successor_receipt_digest
          AND target.provider_binding_id=NEW.provider_binding_id
          AND target.activation_root_digest=NEW.activation_root_digest
          AND NEW.revoked_at>=target.created_at
          AND NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_provider_active_successor_receipts successor
             WHERE successor.predecessor_active_successor_receipt_id=target.active_successor_receipt_id))
BEGIN SELECT RAISE(ABORT,'V274 active successor revocation target is not the exact head'); END;
