use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteMainCloseTestFaultPhase {
    Unlock,
    FileClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteMainCloseTestFaultTiming {
    BeforeCall,
    AfterSuccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteMainCloseTestFault {
    pub(crate) phase: ManagedSqliteMainCloseTestFaultPhase,
    pub(crate) timing: ManagedSqliteMainCloseTestFaultTiming,
}

pub(crate) trait ManagedSqliteMainCloseTestFaults: Send + Sync + 'static {
    fn before(&self, phase: ManagedSqliteMainCloseTestFaultPhase) -> Result<bool, ()>;
    fn after_success(&self, phase: ManagedSqliteMainCloseTestFaultPhase) -> Result<bool, ()>;
    fn native_failure(&self, phase: ManagedSqliteMainCloseTestFaultPhase);
}

pub(super) fn triggered(
    faults: &Option<Arc<dyn ManagedSqliteMainCloseTestFaults>>,
    phase: ManagedSqliteMainCloseTestFaultPhase,
    timing: ManagedSqliteMainCloseTestFaultTiming,
) -> bool {
    faults.as_ref().is_some_and(|faults| {
        match timing {
            ManagedSqliteMainCloseTestFaultTiming::BeforeCall => faults.before(phase),
            ManagedSqliteMainCloseTestFaultTiming::AfterSuccess => faults.after_success(phase),
        }
        .unwrap_or(true)
    })
}
