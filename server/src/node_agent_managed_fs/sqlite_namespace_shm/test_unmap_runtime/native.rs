use super::super::types::ManagedSqliteShmFailurePhase;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestUnmapNativeTiming {
    Retryable,
    OutcomeUncertain,
}

/// What the narrow adapter learned at the exact Windows boundary.
///
/// Outcome-uncertain seams deliberately discard the native return value; observing either success
/// or failure first and relabeling it as uncertain would not be lawful evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestUnmapNativeObservation {
    NativeFailureObserved,
    ReturnReceiptUnavailable,
}

/// One exact Windows-native boundary whose result is replaced by a one-shot test adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestUnmapNativeOperation {
    ViewUnmapOutcomeUncertain,
    MappingCloseOutcomeUncertain,
    DmsSharedReleaseOutcomeUncertain,
    FileCloseRetryable,
    FileCloseOutcomeUncertain,
    ExactSiblingDeleteRetryable,
    ExactSiblingDeleteOutcomeUncertain,
}

impl ManagedSqliteShmTestUnmapNativeOperation {
    pub(crate) fn phase(self) -> ManagedSqliteShmFailurePhase {
        match self {
            Self::ViewUnmapOutcomeUncertain => ManagedSqliteShmFailurePhase::ViewUnmap,
            Self::MappingCloseOutcomeUncertain => ManagedSqliteShmFailurePhase::MappingClose,
            Self::DmsSharedReleaseOutcomeUncertain => {
                ManagedSqliteShmFailurePhase::DmsSharedRelease
            }
            Self::FileCloseRetryable | Self::FileCloseOutcomeUncertain => {
                ManagedSqliteShmFailurePhase::FileClose
            }
            Self::ExactSiblingDeleteRetryable | Self::ExactSiblingDeleteOutcomeUncertain => {
                ManagedSqliteShmFailurePhase::ExactSiblingDelete
            }
        }
    }

    pub(crate) fn timing(self) -> ManagedSqliteShmTestUnmapNativeTiming {
        match self {
            Self::FileCloseRetryable | Self::ExactSiblingDeleteRetryable => {
                ManagedSqliteShmTestUnmapNativeTiming::Retryable
            }
            _ => ManagedSqliteShmTestUnmapNativeTiming::OutcomeUncertain,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestUnmapNativeReceipt {
    pub(crate) operation: ManagedSqliteShmTestUnmapNativeOperation,
    pub(crate) phase: ManagedSqliteShmFailurePhase,
    pub(crate) timing: ManagedSqliteShmTestUnmapNativeTiming,
    pub(crate) triggered: bool,
    pub(crate) witnessed: bool,
    pub(crate) observation: Option<ManagedSqliteShmTestUnmapNativeObservation>,
}

#[derive(Default)]
pub(super) struct ManagedSqliteShmTestUnmapNativeControl {
    installed: Option<ManagedSqliteShmTestUnmapNativeOperation>,
    consumed: bool,
    observation: Option<ManagedSqliteShmTestUnmapNativeObservation>,
}

impl ManagedSqliteShmTestUnmapNativeControl {
    pub(super) fn install(
        &mut self,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
    ) -> Result<(), &'static str> {
        if self.installed.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_NATIVE_ALREADY_INSTALLED");
        }
        self.installed = Some(operation);
        Ok(())
    }

    pub(super) fn select_for_phase(
        &self,
        phase: ManagedSqliteShmFailurePhase,
    ) -> Option<ManagedSqliteShmTestUnmapNativeOperation> {
        let operation = self.installed?;
        if self.consumed || operation.phase() != phase {
            return None;
        }
        Some(operation)
    }

    pub(super) fn trigger(
        &mut self,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
    ) -> Result<(), &'static str> {
        if self.installed != Some(operation) || self.consumed {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_NATIVE_TRIGGER_INVALID");
        }
        self.consumed = true;
        Ok(())
    }

    pub(super) fn witness(
        &mut self,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
        observation: ManagedSqliteShmTestUnmapNativeObservation,
    ) -> Result<(), &'static str> {
        let expected = match operation.timing() {
            ManagedSqliteShmTestUnmapNativeTiming::Retryable => {
                ManagedSqliteShmTestUnmapNativeObservation::NativeFailureObserved
            }
            ManagedSqliteShmTestUnmapNativeTiming::OutcomeUncertain => {
                ManagedSqliteShmTestUnmapNativeObservation::ReturnReceiptUnavailable
            }
        };
        if self.installed != Some(operation)
            || !self.consumed
            || self.observation.is_some()
            || observation != expected
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_NATIVE_WITNESS_INVALID");
        }
        self.observation = Some(observation);
        Ok(())
    }

    pub(super) fn receipt(&self) -> Option<ManagedSqliteShmTestUnmapNativeReceipt> {
        let operation = self.installed?;
        Some(ManagedSqliteShmTestUnmapNativeReceipt {
            operation,
            phase: operation.phase(),
            timing: operation.timing(),
            triggered: self.consumed,
            witnessed: self.observation.is_some(),
            observation: self.observation,
        })
    }

    pub(super) fn pending(&self) -> usize {
        usize::from(self.installed.is_some() && (!self.consumed || self.observation.is_none()))
    }
}
