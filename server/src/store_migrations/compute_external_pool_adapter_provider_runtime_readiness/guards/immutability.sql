CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_receipts_no_update
BEFORE UPDATE ON compute_external_pool_adapter_provider_runtime_readiness_receipts
BEGIN SELECT RAISE(ABORT,'V270 readiness receipts are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_receipts_no_delete
BEFORE DELETE ON compute_external_pool_adapter_provider_runtime_readiness_receipts
BEGIN SELECT RAISE(ABORT,'V270 readiness receipts are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_revocations_no_update
BEFORE UPDATE ON compute_external_pool_adapter_provider_runtime_readiness_revocations
BEGIN SELECT RAISE(ABORT,'V270 readiness revocations are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_revocations_no_delete
BEFORE DELETE ON compute_external_pool_adapter_provider_runtime_readiness_revocations
BEGIN SELECT RAISE(ABORT,'V270 readiness revocations are immutable'); END;
