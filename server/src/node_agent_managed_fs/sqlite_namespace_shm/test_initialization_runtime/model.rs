use super::super::{
    test_lock_runtime::ManagedSqliteShmTestLockReceipt,
    types::ManagedSqliteShmLockAction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestInitializationFailureV1 {
    CreatedFirstExclusiveReleaseOutcomeUncertain,
}

impl ManagedSqliteShmTestInitializationFailureV1 {
    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::CreatedFirstExclusiveReleaseOutcomeUncertain => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestInitializationEvidenceV1 {
    ControlledFaultActual,
}

impl ManagedSqliteShmTestInitializationEvidenceV1 {
    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::ControlledFaultActual => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestInitializationNativeObservationV1 {
    ReturnReceiptUnavailable,
}

impl ManagedSqliteShmTestInitializationNativeObservationV1 {
    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::ReturnReceiptUnavailable => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestInitializationExpectationV1 {
    pub(crate) case_v1: ManagedSqliteShmTestInitializationFailureV1,
    pub(crate) action: ManagedSqliteShmLockAction,
    pub(crate) first: u8,
    pub(crate) count: u8,
    pub(crate) mask: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestInitializationNativeReceiptV1 {
    pub(crate) observation: ManagedSqliteShmTestInitializationNativeObservationV1,
    pub(crate) offset: u64,
    pub(crate) length: u64,
    pub(crate) exact_call_occurrence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestInitializationReceiptV1 {
    expectation: ManagedSqliteShmTestInitializationExpectationV1,
    native: ManagedSqliteShmTestInitializationNativeReceiptV1,
    requested_lock: ManagedSqliteShmTestLockReceipt,
    ordered_values: [u64; 32],
}

impl ManagedSqliteShmTestInitializationReceiptV1 {
    pub(super) const fn new(
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        native: ManagedSqliteShmTestInitializationNativeReceiptV1,
        requested_lock: ManagedSqliteShmTestLockReceipt,
        ordered_values: [u64; 32],
    ) -> Self {
        Self {
            expectation,
            native,
            requested_lock,
            ordered_values,
        }
    }

    pub(crate) const fn case_v1(self) -> ManagedSqliteShmTestInitializationFailureV1 {
        self.expectation.case_v1
    }

    pub(crate) const fn evidence_v1(self) -> ManagedSqliteShmTestInitializationEvidenceV1 {
        ManagedSqliteShmTestInitializationEvidenceV1::ControlledFaultActual
    }

    pub(crate) const fn expectation(self) -> ManagedSqliteShmTestInitializationExpectationV1 {
        self.expectation
    }

    pub(crate) const fn native_receipt(
        self,
    ) -> ManagedSqliteShmTestInitializationNativeReceiptV1 {
        self.native
    }

    pub(crate) const fn requested_lock_receipt(self) -> ManagedSqliteShmTestLockReceipt {
        self.requested_lock
    }

    pub(crate) const fn ordered_values(self) -> [u64; 32] {
        self.ordered_values
    }
}

pub(super) const fn lock_action_tag(action: ManagedSqliteShmLockAction) -> u64 {
    match action {
        ManagedSqliteShmLockAction::LockShared => 1,
        ManagedSqliteShmLockAction::LockExclusive => 2,
        ManagedSqliteShmLockAction::UnlockShared => 3,
        ManagedSqliteShmLockAction::UnlockExclusive => 4,
    }
}
