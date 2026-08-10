use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass as Class, ManagedSqliteShmFailurePhase as Phase,
};

use super::model::{
    before_registry_callback, case, A2b1Path, Case, FaultTiming, RetainedCustody, SqliteResult,
    BASE_RETAINED,
};
use super::operation::{lock, LockAction, OperationShape, INVALID_RANGE_LOCK};

const LOCK_HELD: RetainedCustody = RetainedCustody {
    exclusive_mask: 1,
    ..BASE_RETAINED
};

pub(super) const CASES: &[Case] = &[
    before_registry_callback(case(
        "lock.invalid_request",
        A2b1Path::ShmLock,
        INVALID_RANGE_LOCK,
        FaultTiming::Native,
        Phase::RequestValidation,
        Class::ProtocolViolation,
        SqliteResult::IoerrShmLock,
        false,
        false,
        false,
        BASE_RETAINED,
        0,
        0,
    )),
    case(
        "lock.local_sibling_contention",
        A2b1Path::ShmLock,
        acquire_operation(1, 0),
        FaultTiming::Native,
        Phase::LockAcquire,
        Class::BusyNoMutation,
        SqliteResult::Busy,
        false,
        false,
        false,
        BASE_RETAINED,
        0,
        0,
    ),
    case(
        "lock.os_contention",
        A2b1Path::ShmLock,
        acquire_operation(0, 0),
        FaultTiming::Native,
        Phase::LockAcquire,
        Class::BusyNoMutation,
        SqliteResult::Busy,
        false,
        false,
        false,
        BASE_RETAINED,
        1,
        0,
    ),
    acquire_before(),
    acquire_after_known(),
    acquire_after_uncertain(),
    release_before(),
    release_after_known(),
    release_after_uncertain(),
    case(
        "lock.release.native_uncertain",
        A2b1Path::ShmLock,
        release_operation(),
        FaultTiming::Native,
        Phase::LockRelease,
        Class::OutcomeUncertainPoisoned,
        SqliteResult::IoerrShmLock,
        false,
        true,
        true,
        LOCK_HELD,
        1,
        0,
    ),
];

const fn acquire_before() -> Case {
    case(
        "lock.acquire.before",
        A2b1Path::ShmLock,
        acquire_operation(0, 0),
        FaultTiming::BeforeCall,
        Phase::LockAcquire,
        Class::IoBeforeMutation,
        SqliteResult::IoerrShmLock,
        false,
        false,
        false,
        BASE_RETAINED,
        0,
        0,
    )
}

const fn acquire_after_known() -> Case {
    case(
        "lock.acquire.after_known",
        A2b1Path::ShmLock,
        acquire_operation(0, 0),
        FaultTiming::AfterSuccess,
        Phase::LockAcquire,
        Class::MutatedButKnown,
        SqliteResult::IoerrShmLock,
        true,
        false,
        true,
        LOCK_HELD,
        1,
        1,
    )
}

const fn acquire_after_uncertain() -> Case {
    case(
        "lock.acquire.after_uncertain",
        A2b1Path::ShmLock,
        acquire_operation(0, 0),
        FaultTiming::AfterSuccess,
        Phase::LockAcquire,
        Class::OutcomeUncertainPoisoned,
        SqliteResult::IoerrShmLock,
        true,
        true,
        true,
        LOCK_HELD,
        1,
        1,
    )
}

const fn release_before() -> Case {
    case(
        "lock.release.before",
        A2b1Path::ShmLock,
        release_operation(),
        FaultTiming::BeforeCall,
        Phase::LockRelease,
        Class::IoBeforeMutation,
        SqliteResult::IoerrShmLock,
        false,
        false,
        false,
        LOCK_HELD,
        0,
        0,
    )
}

const fn release_after_known() -> Case {
    case(
        "lock.release.after_known",
        A2b1Path::ShmLock,
        release_operation(),
        FaultTiming::AfterSuccess,
        Phase::LockRelease,
        Class::MutatedButKnown,
        SqliteResult::IoerrShmLock,
        true,
        false,
        true,
        BASE_RETAINED,
        1,
        1,
    )
}

const fn release_after_uncertain() -> Case {
    case(
        "lock.release.after_uncertain",
        A2b1Path::ShmLock,
        release_operation(),
        FaultTiming::AfterSuccess,
        Phase::LockRelease,
        Class::OutcomeUncertainPoisoned,
        SqliteResult::IoerrShmLock,
        true,
        true,
        true,
        BASE_RETAINED,
        1,
        1,
    )
}

const fn acquire_operation(sibling_shared_mask: u8, sibling_exclusive_mask: u8) -> OperationShape {
    lock(
        LockAction::LockExclusive,
        0,
        0,
        sibling_shared_mask,
        sibling_exclusive_mask,
    )
}

const fn release_operation() -> OperationShape {
    lock(LockAction::UnlockExclusive, 0, 1, 0, 0)
}
