//! Exact Expected vectors for the q11 Lock raw-state rejection cases.

use super::super::super::super::super::source_leaf_authority::{
    CustodyStateV1, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
    ObservableCountsV1, SqliteResultV1, TerminalDispositionV1,
};
use super::super::super::super::{DynamicExpectedV1};
use super::super::super::super::super::terminal_descriptor::PhaseV1;
use super::case::LockRawStateRejectionCaseV1;

pub(super) fn expected_v1(case: LockRawStateRejectionCaseV1) -> DynamicExpectedV1 {
    let mut expected = base_v1(case.phase_v1());
    match case {
        LockRawStateRejectionCaseV1::NullFileDirect => {}
        LockRawStateRejectionCaseV1::UninstalledDirect => {
            expected.raw_slots = CustodyStateV1::Cleared;
        }
        LockRawStateRejectionCaseV1::MethodsNullStatePresentDirect
        | LockRawStateRejectionCaseV1::ForeignMethodsStateNullDirect
        | LockRawStateRejectionCaseV1::ForeignMethodsStatePresentDirect
        | LockRawStateRejectionCaseV1::ExactMethodsStateNullDirect => {
            expected.raw_slots = CustodyStateV1::Retained;
        }
        LockRawStateRejectionCaseV1::OtherTypePayloadMissingDropCompleted
        | LockRawStateRejectionCaseV1::ExpectedTypePayloadMissingDropCompleted => {
            expected.disposition = TerminalDispositionV1::Abandoned;
            expected.raw_slots = CustodyStateV1::Cleared;
            expected.payload = CustodyStateV1::Cleared;
        }
        LockRawStateRejectionCaseV1::OtherTypePayloadPresentDropCompleted => {
            expected.disposition = TerminalDispositionV1::Abandoned;
            expected.raw_slots = CustodyStateV1::Cleared;
            expected.payload = CustodyStateV1::Released;
        }
        LockRawStateRejectionCaseV1::OtherTypePayloadPresentDropUnwindCaught => {
            expected.disposition = TerminalDispositionV1::Quarantined;
            expected.raw_slots = CustodyStateV1::Cleared;
            expected.payload = CustodyStateV1::Quarantined;
        }
        LockRawStateRejectionCaseV1::HandleBoundFileMissingDirect => {
            expected.raw_slots = CustodyStateV1::Unchanged;
            expected.file = CustodyStateV1::Cleared;
            expected.payload = CustodyStateV1::Retained;
        }
    }
    expected
}

fn base_v1(phase: PhaseV1) -> DynamicExpectedV1 {
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Returned,
        phase,
        failure: FailureClassV1::ProtocolViolation,
        mutation: MutationStateV1::None,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::NotReached,
        dms_lock: DmsLockCustodyV1::NotReached,
        raw_slots: CustodyStateV1::NotReached,
        route: CustodyStateV1::NotReached,
        callback: CustodyStateV1::NotReached,
        file: CustodyStateV1::NotReached,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1::default(),
    }
}
