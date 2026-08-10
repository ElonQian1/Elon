use std::{collections::BTreeSet, num::NonZeroU32};

use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole,
    node_agent_managed_fs::{ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase},
};

use super::operation::{DmsCustodyEvidence, OperationShape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum A2b1Path {
    ShmMap,
    ShmLock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FaultTiming {
    BeforeCall,
    AfterSuccess,
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SqliteResult {
    Ok,
    Busy,
    IoerrShmMap,
    IoerrShmLock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RoutePhase {
    Active,
    TerminalQuarantine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EvidenceKind {
    StaticContract,
    WindowsDynamic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExactTarget {
    pub(super) registration_id: u64,
    pub(super) route_ordinal: u64,
    pub(super) runtime_generation: u64,
    pub(super) shm_connection_id: u64,
    pub(super) role: ManagedSqliteLogicalFileRole,
    pub(super) phase_occurrence: Option<NonZeroU32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RetainedCustody {
    pub(super) dms: DmsCustodyEvidence,
    pub(super) views: u8,
    pub(super) mappings: u8,
    pub(super) shm_file: bool,
    pub(super) main_file: bool,
    pub(super) main_lease: bool,
    pub(super) shm_lease: bool,
    pub(super) callback_lease: bool,
    pub(super) registration_context: bool,
    pub(super) shared_mask: u8,
    pub(super) exclusive_mask: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExactCounts {
    pub(super) raw_state_take: u8,
    pub(super) selected_phase_platform_attempt: u8,
    pub(super) selected_phase_platform_success: u8,
    pub(super) callback_begin: u8,
    pub(super) callback_complete_attempt: u8,
    pub(super) callback_complete_success: u8,
    pub(super) registry_route_retire: u8,
    pub(super) logical_route_remove: u8,
    pub(super) custody_retain: u8,
    pub(super) vfs_unregister: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Case {
    pub(super) id: &'static str,
    pub(super) path: A2b1Path,
    pub(super) target: ExactTarget,
    pub(super) operation: OperationShape,
    pub(super) timing: FaultTiming,
    pub(super) phase: ManagedSqliteShmFailurePhase,
    pub(super) class: ManagedSqliteShmFailureClass,
    pub(super) sqlite_result: SqliteResult,
    pub(super) mutation_may_have_occurred: bool,
    pub(super) lock_outcome_uncertain: bool,
    pub(super) domain_terminal: bool,
    pub(super) route_phase: RoutePhase,
    pub(super) remaining_connections: u8,
    pub(super) live_routes: u8,
    pub(super) logical_name_entries: u8,
    pub(super) retained: RetainedCustody,
    pub(super) later_callback_allowed: bool,
    pub(super) counts: ExactCounts,
    pub(super) evidence: EvidenceKind,
}

pub(super) const TARGET: ExactTarget = ExactTarget {
    registration_id: 1,
    route_ordinal: 1,
    runtime_generation: 1,
    shm_connection_id: 1,
    role: ManagedSqliteLogicalFileRole::Main,
    phase_occurrence: None,
};

pub(super) const BASE_RETAINED: RetainedCustody = RetainedCustody {
    dms: DmsCustodyEvidence::Shared,
    views: 0,
    mappings: 0,
    shm_file: true,
    main_file: true,
    main_lease: true,
    shm_lease: true,
    callback_lease: false,
    registration_context: true,
    shared_mask: 0,
    exclusive_mask: 0,
};

pub(super) const fn counts(
    selected_phase_platform_attempt: u8,
    selected_phase_platform_success: u8,
    custody_retain: u8,
    callback_complete_success: u8,
) -> ExactCounts {
    ExactCounts {
        raw_state_take: 0,
        selected_phase_platform_attempt,
        selected_phase_platform_success,
        callback_begin: 1,
        callback_complete_attempt: 1,
        callback_complete_success,
        registry_route_retire: 0,
        logical_route_remove: 0,
        custody_retain,
        vfs_unregister: 0,
    }
}

pub(super) const fn case(
    id: &'static str,
    path: A2b1Path,
    operation: OperationShape,
    timing: FaultTiming,
    phase: ManagedSqliteShmFailurePhase,
    class: ManagedSqliteShmFailureClass,
    sqlite_result: SqliteResult,
    mutation: bool,
    lock_uncertain: bool,
    terminal: bool,
    retained: RetainedCustody,
    selected_phase_platform_attempt: u8,
    selected_phase_platform_success: u8,
) -> Case {
    let retained = if terminal {
        RetainedCustody {
            callback_lease: true,
            ..retained
        }
    } else {
        retained
    };
    Case {
        id,
        path,
        target: ExactTarget {
            phase_occurrence: match timing {
                FaultTiming::BeforeCall | FaultTiming::AfterSuccess => NonZeroU32::new(1),
                FaultTiming::Native => None,
            },
            ..TARGET
        },
        operation,
        timing,
        phase,
        class,
        sqlite_result,
        mutation_may_have_occurred: mutation,
        lock_outcome_uncertain: lock_uncertain,
        domain_terminal: terminal,
        route_phase: if terminal {
            RoutePhase::TerminalQuarantine
        } else {
            RoutePhase::Active
        },
        remaining_connections: 2,
        live_routes: 2,
        logical_name_entries: 6,
        retained,
        later_callback_allowed: !terminal,
        counts: counts(
            selected_phase_platform_attempt,
            selected_phase_platform_success,
            if terminal { 2 } else { 0 },
            if terminal { 0 } else { 1 },
        ),
        evidence: EvidenceKind::StaticContract,
    }
}

pub(super) const fn before_registry_callback(case: Case) -> Case {
    Case {
        counts: ExactCounts {
            callback_begin: 0,
            callback_complete_attempt: 0,
            callback_complete_success: 0,
            ..case.counts
        },
        ..case
    }
}

pub(super) fn validate_matrix(map: &[Case], lock: &[Case]) -> Result<(), &'static str> {
    let mut ids = BTreeSet::new();
    for case in map.iter().chain(lock) {
        validate_case(case)?;
        if !ids.insert(case.id) {
            return Err("duplicate A2b1 case id");
        }
    }
    require_phases(map, MAP_PHASES)?;
    require_phases(lock, LOCK_PHASES)?;
    if !lock
        .iter()
        .any(|case| case.sqlite_result == SqliteResult::Busy)
    {
        return Err("lock matrix must preserve native BUSY contention");
    }
    Ok(())
}

fn validate_case(case: &Case) -> Result<(), &'static str> {
    let callback_expected = u8::from(case.phase != ManagedSqliteShmFailurePhase::RequestValidation);
    if case.id.is_empty()
        || case.target.registration_id == 0
        || case.target.route_ordinal == 0
        || case.target.runtime_generation == 0
        || case.target.shm_connection_id == 0
        || case.target.role != ManagedSqliteLogicalFileRole::Main
        || case.target.phase_occurrence
            != match case.timing {
                FaultTiming::BeforeCall | FaultTiming::AfterSuccess => NonZeroU32::new(1),
                FaultTiming::Native => None,
            }
    {
        return Err("A2b1 exact target is incomplete");
    }
    if case.logical_name_entries
        != case
            .live_routes
            .checked_mul(3)
            .ok_or("route count overflow")?
        || case.remaining_connections != 2
        || case.retained.views > case.retained.mappings
        || case.retained.dms == DmsCustodyEvidence::Absent
            && (case.retained.views != 0 || case.retained.mappings != 0 || case.retained.shm_file)
        || case.retained.dms != DmsCustodyEvidence::Absent && !case.retained.shm_file
        || case.retained.shared_mask & case.retained.exclusive_mask != 0
        || !case.retained.main_file
        || !case.retained.main_lease
        || !case.retained.shm_lease
        || !case.retained.registration_context
    {
        return Err("A2b1 custody or route conservation mismatch");
    }
    super::operation::validate_operation(case)?;
    if case.domain_terminal != (case.route_phase == RoutePhase::TerminalQuarantine)
        || case.later_callback_allowed == case.domain_terminal
        || case.counts.raw_state_take != 0
        || case.counts.callback_begin != callback_expected
        || case.counts.callback_complete_attempt != callback_expected
        || case.counts.callback_complete_success
            != callback_expected * u8::from(!case.domain_terminal)
        || case.counts.registry_route_retire != 0
        || case.counts.logical_route_remove != 0
        || case.counts.vfs_unregister != 0
        || case.counts.custody_retain != if case.domain_terminal { 2 } else { 0 }
        || case.evidence != EvidenceKind::StaticContract
    {
        return Err("A2b1 terminal or exact-count contract mismatch");
    }
    if case.counts.selected_phase_platform_success > case.counts.selected_phase_platform_attempt
        || case.lock_outcome_uncertain
            && case.class != ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
        || (case.mutation_may_have_occurred || case.lock_outcome_uncertain) && !case.domain_terminal
    {
        return Err("A2b1 platform or lock outcome mismatch");
    }
    match case.timing {
        FaultTiming::BeforeCall
            if case.counts.selected_phase_platform_attempt != 0
                || case.counts.selected_phase_platform_success != 0
                || case.class
                    != if case.mutation_may_have_occurred {
                        ManagedSqliteShmFailureClass::MutatedButKnown
                    } else {
                        ManagedSqliteShmFailureClass::IoBeforeMutation
                    } =>
        {
            return Err("A2b1 before-call phase contract mismatch");
        }
        FaultTiming::AfterSuccess
            if case.phase == ManagedSqliteShmFailurePhase::FileSize
                || case.counts.selected_phase_platform_attempt != 1
                || case.counts.selected_phase_platform_success != 1
                || !case.mutation_may_have_occurred
                || !matches!(
                    case.class,
                    ManagedSqliteShmFailureClass::MutatedButKnown
                        | ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
                ) =>
        {
            return Err("A2b1 after-success phase contract mismatch");
        }
        _ => {}
    }
    match (case.path, case.sqlite_result) {
        (A2b1Path::ShmMap, SqliteResult::IoerrShmMap | SqliteResult::Ok)
        | (A2b1Path::ShmLock, SqliteResult::IoerrShmLock | SqliteResult::Busy) => Ok(()),
        _ => Err("A2b1 SQLite result does not match callback path"),
    }
}

fn require_phases(
    cases: &[Case],
    required: &[ManagedSqliteShmFailurePhase],
) -> Result<(), &'static str> {
    for phase in required {
        if !cases.iter().any(|case| case.phase == *phase) {
            return Err("A2b1 phase is missing from static cases");
        }
    }
    Ok(())
}

const MAP_PHASES: &[ManagedSqliteShmFailurePhase] = &[
    ManagedSqliteShmFailurePhase::ExactSiblingOpen,
    ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
    ManagedSqliteShmFailurePhase::DmsTruncate,
    ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
    ManagedSqliteShmFailurePhase::DmsSharedAcquire,
    ManagedSqliteShmFailurePhase::FileSize,
    ManagedSqliteShmFailurePhase::FileGrow,
    ManagedSqliteShmFailurePhase::MappingCreate,
    ManagedSqliteShmFailurePhase::ViewMap,
];

const LOCK_PHASES: &[ManagedSqliteShmFailurePhase] = &[
    ManagedSqliteShmFailurePhase::RequestValidation,
    ManagedSqliteShmFailurePhase::LockAcquire,
    ManagedSqliteShmFailurePhase::LockRelease,
];

#[allow(dead_code)]
const DYNAMIC_EVIDENCE_IS_NOT_THIS_BATCH: EvidenceKind = EvidenceKind::WindowsDynamic;
