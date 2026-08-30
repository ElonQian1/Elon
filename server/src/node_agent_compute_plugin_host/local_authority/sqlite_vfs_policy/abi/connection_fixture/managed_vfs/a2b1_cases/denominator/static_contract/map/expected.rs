use super::super::model::{
    CustodyState, DmsLockCustody, Expected, FailureClass, LockEffect, MutationState,
    ObservableCounts, RootOperation, SqliteResult, TerminalDisposition,
};

pub(super) fn unavailable(
    phase: &'static str,
    failure: FailureClass,
    mutation: MutationState,
    disposition: TerminalDisposition,
    counts: ObservableCounts,
) -> Expected {
    let mut expected = Expected::unavailable(RootOperation::Map, phase);
    expected.disposition = disposition;
    expected.failure = failure;
    expected.mutation = mutation;
    expected.raw_slots = CustodyState::Unchanged;
    expected.route = CustodyState::Unchanged;
    expected.callback = CustodyState::Released;
    expected.file = CustodyState::Retained;
    expected.counts = counts;
    expected
}

pub(super) fn raw_fallback(raw_slots: CustodyState, payload: CustodyState) -> Expected {
    let mut expected = Expected::unavailable(RootOperation::Map, "RawAdmission");
    expected.disposition = TerminalDisposition::Abandoned;
    expected.failure = FailureClass::ProtocolViolation;
    expected.raw_slots = raw_slots;
    expected.payload = payload;
    expected
}

pub(super) fn success(mapped: bool, mutation: MutationState, counts: ObservableCounts) -> Expected {
    Expected {
        sqlite: SqliteResult::Ok,
        disposition: TerminalDisposition::Returned,
        phase: "Success",
        failure: if mapped {
            FailureClass::None
        } else {
            FailureClass::NotPresent
        },
        mutation,
        lock_outcome_uncertain: false,
        lock_effect: LockEffect::NotReached,
        dms_lock: DmsLockCustody::NotReached,
        raw_slots: CustodyState::Unchanged,
        route: CustodyState::Unchanged,
        callback: CustodyState::Released,
        file: CustodyState::Retained,
        mapping: if mapped {
            CustodyState::Retained
        } else {
            CustodyState::Unchanged
        },
        view: if mapped {
            CustodyState::Retained
        } else {
            CustodyState::Unchanged
        },
        payload: if mapped {
            CustodyState::Retained
        } else {
            CustodyState::Released
        },
        counts,
    }
}
