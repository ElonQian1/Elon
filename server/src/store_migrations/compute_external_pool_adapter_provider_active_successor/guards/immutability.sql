CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_receipt_no_update
BEFORE UPDATE ON compute_external_pool_adapter_provider_active_successor_receipts
BEGIN SELECT RAISE(ABORT,'V274 active successor receipts are immutable'); END;

CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_receipt_no_delete
BEFORE DELETE ON compute_external_pool_adapter_provider_active_successor_receipts
BEGIN SELECT RAISE(ABORT,'V274 active successor receipts are immutable'); END;

CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_revocation_no_update
BEFORE UPDATE ON compute_external_pool_adapter_provider_active_successor_revocations
BEGIN SELECT RAISE(ABORT,'V274 active successor revocations are immutable'); END;

CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_revocation_no_delete
BEFORE DELETE ON compute_external_pool_adapter_provider_active_successor_revocations
BEGIN SELECT RAISE(ABORT,'V274 active successor revocations are immutable'); END;
