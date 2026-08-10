#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapMode {
    Observe,
    Extend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LockAction {
    LockExclusive,
    UnlockExclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DmsCustodyEvidence {
    Absent,
    Released,
    Shared,
    ExclusiveKnown,
    ExclusiveOutcomeUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OperationShape {
    Map {
        mode: MapMode,
        region: u32,
        region_size: u32,
        phase_dms: DmsCustodyEvidence,
    },
    Lock {
        first: u8,
        count: u8,
        action: LockAction,
        pre_shared_mask: u8,
        pre_exclusive_mask: u8,
        sibling_shared_mask: u8,
        sibling_exclusive_mask: u8,
    },
}

pub(super) const REGION_ZERO_OBSERVE: OperationShape = OperationShape::Map {
    mode: MapMode::Observe,
    region: 0,
    region_size: 32 * 1024,
    phase_dms: DmsCustodyEvidence::Shared,
};

pub(super) const fn map_at_dms(phase_dms: DmsCustodyEvidence) -> OperationShape {
    OperationShape::Map {
        mode: MapMode::Extend,
        region: 0,
        region_size: 32 * 1024,
        phase_dms,
    }
}

pub(super) const fn lock(
    action: LockAction,
    pre_shared_mask: u8,
    pre_exclusive_mask: u8,
    sibling_shared_mask: u8,
    sibling_exclusive_mask: u8,
) -> OperationShape {
    OperationShape::Lock {
        first: 0,
        count: 1,
        action,
        pre_shared_mask,
        pre_exclusive_mask,
        sibling_shared_mask,
        sibling_exclusive_mask,
    }
}

pub(super) const INVALID_RANGE_LOCK: OperationShape = OperationShape::Lock {
    first: 7,
    count: 2,
    action: LockAction::LockExclusive,
    pre_shared_mask: 0,
    pre_exclusive_mask: 0,
    sibling_shared_mask: 0,
    sibling_exclusive_mask: 0,
};

pub(super) fn validate_operation(case: &Case) -> Result<(), &'static str> {
    match (case.path, case.operation) {
        (
            A2b1Path::ShmMap,
            OperationShape::Map {
                mode,
                region,
                region_size,
                phase_dms,
            },
        ) => validate_map(case, mode, region, region_size, phase_dms),
        (
            A2b1Path::ShmLock,
            OperationShape::Lock {
                first,
                count,
                action,
                pre_shared_mask,
                pre_exclusive_mask,
                sibling_shared_mask,
                sibling_exclusive_mask,
            },
        ) => validate_lock(
            case,
            first,
            count,
            action,
            pre_shared_mask,
            pre_exclusive_mask,
            sibling_shared_mask,
            sibling_exclusive_mask,
        ),
        _ => Err("A2b1 callback path and operation shape differ"),
    }
}

fn validate_map(
    case: &Case,
    mode: MapMode,
    region: u32,
    region_size: u32,
    phase_dms: DmsCustodyEvidence,
) -> Result<(), &'static str> {
    let expected_phase_dms = match case.phase {
        ManagedSqliteShmFailurePhase::ExactSiblingOpen => DmsCustodyEvidence::Absent,
        ManagedSqliteShmFailurePhase::DmsExclusiveAcquire
        | ManagedSqliteShmFailurePhase::DmsSharedAcquire => DmsCustodyEvidence::Released,
        ManagedSqliteShmFailurePhase::DmsTruncate
        | ManagedSqliteShmFailurePhase::DmsExclusiveRelease => DmsCustodyEvidence::ExclusiveKnown,
        ManagedSqliteShmFailurePhase::FileSize
        | ManagedSqliteShmFailurePhase::FileGrow
        | ManagedSqliteShmFailurePhase::MappingCreate
        | ManagedSqliteShmFailurePhase::ViewMap => DmsCustodyEvidence::Shared,
        _ => return Err("A2b1 map phase is outside the declared subset"),
    };
    let expected_retained_dms = match (case.phase, case.timing) {
        (ManagedSqliteShmFailurePhase::ExactSiblingOpen, FaultTiming::BeforeCall)
        | (ManagedSqliteShmFailurePhase::DmsExclusiveAcquire, FaultTiming::BeforeCall)
        | (ManagedSqliteShmFailurePhase::DmsTruncate, FaultTiming::BeforeCall)
        | (ManagedSqliteShmFailurePhase::DmsSharedAcquire, FaultTiming::BeforeCall) => {
            DmsCustodyEvidence::Absent
        }
        (ManagedSqliteShmFailurePhase::ExactSiblingOpen, FaultTiming::AfterSuccess)
        | (ManagedSqliteShmFailurePhase::DmsExclusiveRelease, FaultTiming::AfterSuccess) => {
            DmsCustodyEvidence::Released
        }
        (ManagedSqliteShmFailurePhase::DmsExclusiveAcquire, FaultTiming::AfterSuccess)
        | (ManagedSqliteShmFailurePhase::DmsTruncate, FaultTiming::AfterSuccess)
        | (ManagedSqliteShmFailurePhase::DmsExclusiveRelease, FaultTiming::BeforeCall) => {
            DmsCustodyEvidence::ExclusiveKnown
        }
        _ => DmsCustodyEvidence::Shared,
    };
    if region != 0
        || region_size == 0
        || phase_dms != expected_phase_dms
        || case.retained.dms != expected_retained_dms
        || (mode == MapMode::Observe) != (case.sqlite_result == SqliteResult::Ok)
    {
        return Err("A2b1 map operation shape is inconsistent");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_lock(
    case: &Case,
    first: u8,
    count: u8,
    action: LockAction,
    pre_shared_mask: u8,
    pre_exclusive_mask: u8,
    sibling_shared_mask: u8,
    sibling_exclusive_mask: u8,
) -> Result<(), &'static str> {
    let range_is_valid = count != 0 && first.checked_add(count).is_some_and(|end| end <= 8);
    if pre_shared_mask & pre_exclusive_mask != 0
        || sibling_shared_mask & sibling_exclusive_mask != 0
        || (case.phase == ManagedSqliteShmFailurePhase::RequestValidation) != !range_is_valid
    {
        return Err("A2b1 lock operation shape is inconsistent");
    }
    if !range_is_valid {
        if case.retained.dms != DmsCustodyEvidence::Shared
            || case.retained.shared_mask != pre_shared_mask
            || case.retained.exclusive_mask != pre_exclusive_mask
        {
            return Err("A2b1 invalid lock request changed retained custody");
        }
        return Ok(());
    }
    if (case.phase == ManagedSqliteShmFailurePhase::LockAcquire)
        != (action == LockAction::LockExclusive)
        || (case.phase == ManagedSqliteShmFailurePhase::LockRelease)
            != (action == LockAction::UnlockExclusive)
    {
        return Err("A2b1 lock phase and action differ");
    }
    let mask = (((1u16 << count) - 1) << first) as u8;
    let sibling_overlap = (sibling_shared_mask | sibling_exclusive_mask) & mask != 0;
    let (expected_shared, expected_exclusive) = match (action, case.timing) {
        (LockAction::LockExclusive, FaultTiming::AfterSuccess) => {
            (pre_shared_mask, pre_exclusive_mask | mask)
        }
        (LockAction::UnlockExclusive, FaultTiming::AfterSuccess) => {
            (pre_shared_mask, pre_exclusive_mask & !mask)
        }
        _ => (pre_shared_mask, pre_exclusive_mask),
    };
    if case.retained.dms != DmsCustodyEvidence::Shared
        || pre_shared_mask & mask != 0
        || action == LockAction::LockExclusive && pre_exclusive_mask & mask != 0
        || action == LockAction::UnlockExclusive && pre_exclusive_mask & mask != mask
        || action == LockAction::UnlockExclusive && sibling_overlap
        || case.sqlite_result == SqliteResult::Busy
            && sibling_overlap != (case.counts.selected_phase_platform_attempt == 0)
        || case.sqlite_result != SqliteResult::Busy && sibling_overlap
        || case.retained.shared_mask != expected_shared
        || case.retained.exclusive_mask != expected_exclusive
    {
        return Err("A2b1 lock pre-state or retained mask is inconsistent");
    }
    Ok(())
}
use crate::node_agent_managed_fs::ManagedSqliteShmFailurePhase;

use super::model::{A2b1Path, Case, FaultTiming, SqliteResult};
