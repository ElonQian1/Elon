use std::sync::Arc;

#[cfg(windows)]
use std::num::NonZeroU32;

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

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteMainCloseTestNativeRequest {
    MainLockReleaseNativeUncertainShared,
    MainLockReleaseNativeUncertainReserved,
    MainFileCloseNativeRetryable,
    MainFileCloseNativeUncertain,
}

#[cfg(windows)]
impl ManagedSqliteMainCloseTestNativeRequest {
    fn phase(self) -> ManagedSqliteMainCloseTestFaultPhase {
        match self {
            Self::MainLockReleaseNativeUncertainShared
            | Self::MainLockReleaseNativeUncertainReserved => {
                ManagedSqliteMainCloseTestFaultPhase::Unlock
            }
            Self::MainFileCloseNativeRetryable | Self::MainFileCloseNativeUncertain => {
                ManagedSqliteMainCloseTestFaultPhase::FileClose
            }
        }
    }
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteMainLockHeldRangePrestate {
    Shared,
    ReservedShared,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteMainLockOffsetClass {
    SharedRange,
    ReservedByte,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteMainCloseTestNativeObservation {
    NativeFailureObserved,
    ReturnReceiptUnavailable,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteMainCloseTestNativeEvidence {
    MainLockRelease {
        held_range_prestate: ManagedSqliteMainLockHeldRangePrestate,
        selected_offset_class: ManagedSqliteMainLockOffsetClass,
        exact_call_occurrence: NonZeroU32,
        observation: ManagedSqliteMainCloseTestNativeObservation,
    },
    MainFileClose {
        exact_call_occurrence: NonZeroU32,
        observation: ManagedSqliteMainCloseTestNativeObservation,
    },
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteMainCloseTestProtocolFailure {
    NativeEvidenceObservationRejected(ManagedSqliteMainCloseTestNativeEvidence),
    NativeEvidenceIncomplete {
        request: ManagedSqliteMainCloseTestNativeRequest,
        exact_call_occurrence: Option<NonZeroU32>,
        observation: Option<ManagedSqliteMainCloseTestNativeObservation>,
    },
}

pub(crate) trait ManagedSqliteMainCloseTestFaults: Send + Sync + 'static {
    fn before(&self, phase: ManagedSqliteMainCloseTestFaultPhase) -> Result<bool, ()>;
    fn after_success(&self, phase: ManagedSqliteMainCloseTestFaultPhase) -> Result<bool, ()>;
    fn native_failure(&self, phase: ManagedSqliteMainCloseTestFaultPhase);

    #[cfg(windows)]
    fn claim_test_native(
        &self,
        _phase: ManagedSqliteMainCloseTestFaultPhase,
    ) -> Result<Option<ManagedSqliteMainCloseTestNativeRequest>, ()> {
        Ok(None)
    }

    #[cfg(windows)]
    fn observe_test_native(
        &self,
        _evidence: ManagedSqliteMainCloseTestNativeEvidence,
    ) -> Result<(), ()> {
        Ok(())
    }
}

#[cfg(windows)]
pub(super) fn claim_test_native(
    faults: &Option<Arc<dyn ManagedSqliteMainCloseTestFaults>>,
    phase: ManagedSqliteMainCloseTestFaultPhase,
) -> Result<Option<ManagedSqliteMainCloseTestNativeRequest>, ()> {
    let request = match faults {
        Some(faults) => faults.claim_test_native(phase)?,
        None => None,
    };
    if request.is_some_and(|request| request.phase() != phase) {
        return Err(());
    }
    Ok(request)
}

#[cfg(windows)]
pub(super) fn observe_test_native(
    faults: &Option<Arc<dyn ManagedSqliteMainCloseTestFaults>>,
    evidence: ManagedSqliteMainCloseTestNativeEvidence,
) -> Result<(), ()> {
    faults.as_ref().ok_or(())?.observe_test_native(evidence)
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
