CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_runs_no_update
BEFORE UPDATE ON compute_external_pool_adapter_task_protocol_conformance_run_receipts
BEGIN SELECT RAISE(ABORT,'V272 task protocol conformance run receipts are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_runs_no_delete
BEFORE DELETE ON compute_external_pool_adapter_task_protocol_conformance_run_receipts
BEGIN SELECT RAISE(ABORT,'V272 task protocol conformance run receipts are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_revocations_no_update
BEFORE UPDATE ON compute_external_pool_adapter_task_protocol_conformance_revocations
BEGIN SELECT RAISE(ABORT,'V272 task protocol conformance revocations are immutable'); END;
CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_revocations_no_delete
BEFORE DELETE ON compute_external_pool_adapter_task_protocol_conformance_revocations
BEGIN SELECT RAISE(ABORT,'V272 task protocol conformance revocations are immutable'); END;
