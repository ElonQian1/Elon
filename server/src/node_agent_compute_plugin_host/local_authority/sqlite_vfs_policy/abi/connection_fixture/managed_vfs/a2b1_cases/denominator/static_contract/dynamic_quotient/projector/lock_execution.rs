//! Private Lock execution-receipt projection bridge.

use super::*;

pub(in super::super) fn project_validated_dynamic_terminal_with_lock_execution_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
    execution: LockRunnerExecutionReceiptV1,
) -> Result<ValidatedDynamicTerminalV1, ProjectionErrorV1> {
    let prepared = prepare_dynamic_terminal_v1(record, descriptor)?;
    let runner_admission =
        runner_admission::resolve_with_lock_execution_v1(&prepared.key, prepared.member, execution)
            .map_err(map_runner_admission_error)?;
    Ok(finish_prepared_terminal(prepared, runner_admission, false))
}
