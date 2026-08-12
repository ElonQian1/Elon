//! Sanitized actual-to-frozen bridge for the two process-isolated registration runners.
//!
//! This bridge deliberately accepts counts and presence bits only. It cannot carry a raw VFS
//! table, name pointer, context pointer, connection identifier, or native handle. The zero
//! SQLite/SHM connection counts are facts of the direct registration-only runner construction;
//! they are not a general physical topology snapshot. Route and logical-name counts must come
//! from the managed registration witnesses rather than being inferred from those construction
//! facts.

use super::{
    invariants,
    model::{
        Case, DmsCustody, EvidenceKind, FailureClass, LogicalRoutePhase, NodePrecondition, Path,
        Phase, RegistrationPhase, RegistryRoutePhase, SqliteOutcome, TargetScope, Timing,
        TopologyKind,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum DynamicRegistrationTiming {
    BeforeCall,
    AfterSuccessKnown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum DynamicRegistrationRetainedDisposition {
    Registered,
    Unregistered,
}

impl DynamicRegistrationTiming {
    fn frozen(self) -> Timing {
        match self {
            Self::BeforeCall => Timing::BeforeCall,
            Self::AfterSuccessKnown => Timing::AfterSuccessKnown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct DynamicRegistrationActual {
    pub(in super::super) timing: DynamicRegistrationTiming,
    pub(in super::super) pre_sqlite_connections: u8,
    pub(in super::super) pre_shm_connections: u8,
    pub(in super::super) pre_registry_routes: u8,
    pub(in super::super) pre_logical_names: u8,
    pub(in super::super) post_sqlite_connections: u8,
    pub(in super::super) post_shm_connections: u8,
    pub(in super::super) post_registry_routes: u8,
    pub(in super::super) post_logical_names: u8,
    pub(in super::super) lookup_present_before: bool,
    pub(in super::super) lookup_present_after: bool,
    pub(in super::super) before_call_observations: u8,
    pub(in super::super) before_call_triggers: u8,
    pub(in super::super) after_success_observations: u8,
    pub(in super::super) after_success_triggers: u8,
    pub(in super::super) lifecycle_pending: u8,
    pub(in super::super) lifecycle_terminal: bool,
    pub(in super::super) retained_routes: u8,
    pub(in super::super) retained_logical_names: u8,
    pub(in super::super) retained_vfs_table: bool,
    pub(in super::super) retained_vfs_name: bool,
    pub(in super::super) retained_vfs_context: bool,
    pub(in super::super) retained_disposition: DynamicRegistrationRetainedDisposition,
    pub(in super::super) custody_retained: bool,
    pub(in super::super) root_present_after_failure: bool,
}

pub(in super::super) fn validate_dynamic_registration(
    actual: DynamicRegistrationActual,
) -> Result<(), &'static str> {
    let inventory = invariants::inventory();
    invariants::validate(&inventory)?;
    let expected = select_frozen_case(&inventory, actual.timing.frozen())?;
    validate_frozen_registration_boundary(&expected)?;

    check(
        actual.pre_sqlite_connections,
        expected.pre.sqlite_connections,
        "dynamic registration pre SQLite connection count differs from frozen case",
    )?;
    check(
        actual.pre_shm_connections,
        expected.pre.shm_connections,
        "dynamic registration pre SHM connection count differs from frozen case",
    )?;
    check(
        actual.pre_registry_routes,
        expected.pre.registry_routes,
        "dynamic registration pre registry route count differs from frozen case",
    )?;
    check(
        actual.pre_logical_names,
        expected.pre.logical_names,
        "dynamic registration pre logical-name count differs from frozen case",
    )?;
    check(
        actual.post_sqlite_connections,
        expected.post.sqlite_connections,
        "dynamic registration post SQLite connection count differs from frozen case",
    )?;
    check(
        actual.post_shm_connections,
        expected.post.shm_connections,
        "dynamic registration post SHM connection count differs from frozen case",
    )?;
    check(
        actual.post_registry_routes,
        expected.post.registry_routes,
        "dynamic registration post registry route count differs from frozen case",
    )?;
    check(
        actual.post_logical_names,
        expected.post.logical_names,
        "dynamic registration post logical-name count differs from frozen case",
    )?;

    let expected_lookup_after =
        expected.registration_phase == RegistrationPhase::RetainedRegistered;
    check(
        actual.lookup_present_before,
        true,
        "dynamic registration VFS lookup must be present before unregister",
    )?;
    check(
        actual.lookup_present_after,
        expected_lookup_after,
        "dynamic registration post-unregister VFS lookup differs from frozen phase",
    )?;

    let (before_observations, before_triggers, after_observations, after_triggers) =
        expected_lifecycle_shape(expected.timing)?;
    check(
        actual.before_call_observations,
        before_observations,
        "dynamic registration before-call observation count differs from frozen timing",
    )?;
    check(
        actual.before_call_triggers,
        before_triggers,
        "dynamic registration before-call trigger count differs from frozen timing",
    )?;
    check(
        actual.after_success_observations,
        after_observations,
        "dynamic registration after-success observation count differs from frozen timing",
    )?;
    check(
        actual.after_success_triggers,
        after_triggers,
        "dynamic registration after-success trigger count differs from frozen timing",
    )?;

    // The exact lifecycle shape above proves these are observations of one selected fault step,
    // even though an after-success step emits both a before and an after callback observation.
    let actual_fault_observe = u8::from(
        actual
            .before_call_observations
            .saturating_add(actual.after_success_observations)
            > 0,
    );
    let actual_fault_trigger = actual
        .before_call_triggers
        .saturating_add(actual.after_success_triggers);
    let actual_unregister_attempt =
        u8::from(actual.before_call_observations > 0 && actual.before_call_triggers == 0);
    let actual_unregister_success = u8::from(
        actual.after_success_observations > 0
            && actual.lookup_present_before
            && !actual.lookup_present_after,
    );
    check(
        actual_fault_observe,
        expected.counts.fault_observe,
        "dynamic registration fault observation count differs from frozen case",
    )?;
    check(
        actual_fault_trigger,
        expected.counts.fault_trigger,
        "dynamic registration fault trigger count differs from frozen case",
    )?;
    check(
        actual.lifecycle_pending,
        expected.counts.fault_pending,
        "dynamic registration pending fault count differs from frozen case",
    )?;
    check(
        actual_unregister_attempt,
        expected.counts.vfs_unregister_attempt,
        "dynamic registration unregister attempt count differs from frozen case",
    )?;
    check(
        actual_unregister_success,
        expected.counts.vfs_unregister_success,
        "dynamic registration unregister success count differs from frozen case",
    )?;
    check(
        actual.lifecycle_terminal,
        expected.counts.fault_trigger == 1 && expected.counts.fault_pending == 0,
        "dynamic registration lifecycle terminal state differs from frozen fault state",
    )?;

    check(
        actual.retained_routes,
        u8::from(expected.retained.registry_entry),
        "dynamic registration retained route count differs from frozen custody",
    )?;
    check(
        actual.retained_logical_names,
        expected.retained.logical_names,
        "dynamic registration retained logical-name count differs from frozen custody",
    )?;
    check(
        actual.retained_vfs_table,
        expected.retained.vfs_table,
        "dynamic registration retained VFS-table presence differs from frozen custody",
    )?;
    check(
        actual.retained_vfs_name,
        expected.retained.vfs_name,
        "dynamic registration retained VFS-name presence differs from frozen custody",
    )?;
    check(
        actual.retained_vfs_context,
        expected.retained.vfs_context,
        "dynamic registration retained VFS-context presence differs from frozen custody",
    )?;
    let expected_disposition = match expected.registration_phase {
        RegistrationPhase::Registered | RegistrationPhase::RetainedRegistered => {
            DynamicRegistrationRetainedDisposition::Registered
        }
        RegistrationPhase::Unregistered | RegistrationPhase::RetainedAfterUnregister => {
            DynamicRegistrationRetainedDisposition::Unregistered
        }
    };
    check(
        actual.retained_disposition,
        expected_disposition,
        "dynamic registration retained disposition differs from frozen registration phase",
    )?;
    check(
        u8::from(actual.custody_retained),
        expected.counts.custody_retain,
        "dynamic registration custody-retain count differs from frozen case",
    )?;
    check(
        actual.root_present_after_failure,
        true,
        "dynamic registration child root is missing before process-isolated cleanup",
    )?;

    let actual_registration_phase = match (actual.custody_retained, actual.retained_disposition) {
        (true, DynamicRegistrationRetainedDisposition::Registered) => {
            RegistrationPhase::RetainedRegistered
        }
        (true, DynamicRegistrationRetainedDisposition::Unregistered) => {
            RegistrationPhase::RetainedAfterUnregister
        }
        (false, DynamicRegistrationRetainedDisposition::Unregistered) => {
            RegistrationPhase::Unregistered
        }
        (false, DynamicRegistrationRetainedDisposition::Registered) => {
            RegistrationPhase::Registered
        }
    };
    check(
        actual_registration_phase,
        expected.registration_phase,
        "dynamic registration phase derived from disposition and custody differs from frozen case",
    )?;
    check(
        actual.lookup_present_before && !actual.lookup_present_after,
        expected.mutation_may_have_occurred,
        "dynamic registration unregister mutation differs from frozen case",
    )?;

    Ok(())
}

fn select_frozen_case(inventory: &[Case], timing: Timing) -> Result<Case, &'static str> {
    let mut matches = inventory.iter().copied().filter(|case| {
        case.path == Path::RegistrationShutdown
            && case.phase == Phase::VfsUnregister
            && case.timing == timing
    });
    let selected = matches
        .next()
        .ok_or("frozen registration case is missing for dynamic timing")?;
    if matches.next().is_some() {
        return Err("frozen registration case is ambiguous for dynamic timing");
    }
    Ok(selected)
}

fn validate_frozen_registration_boundary(expected: &Case) -> Result<(), &'static str> {
    check(
        expected.topology_kind,
        TopologyKind::RegistrationOnly,
        "frozen dynamic registration topology is not RegistrationOnly",
    )?;
    check(
        expected.node_precondition,
        NodePrecondition::NotApplicable,
        "frozen dynamic registration node precondition changed",
    )?;
    check(
        expected.class,
        FailureClass::RegistrationRetained,
        "frozen dynamic registration failure class changed",
    )?;
    check(
        expected.target.scope,
        TargetScope::Registration,
        "frozen dynamic registration target scope changed",
    )?;
    check(
        expected.sqlite_outcome,
        SqliteOutcome::NotApplicable,
        "frozen dynamic registration SQLite outcome changed",
    )?;
    check(
        expected.registry_route_phase,
        RegistryRoutePhase::Removed,
        "frozen dynamic registration registry route phase changed",
    )?;
    check(
        expected.logical_route_phase,
        LogicalRoutePhase::Removed,
        "frozen dynamic registration logical route phase changed",
    )?;
    check(
        expected.domain_terminal,
        false,
        "frozen dynamic registration domain terminal state changed",
    )?;
    check(
        expected.evidence,
        EvidenceKind::StaticContract,
        "dynamic registration bridge must not promote evidence to WindowsDynamic",
    )?;
    check(
        expected.retained.node,
        false,
        "frozen dynamic registration unexpectedly retains a route node",
    )?;
    check(
        expected.retained.views,
        0,
        "frozen dynamic registration unexpectedly retains views",
    )?;
    check(
        expected.retained.mappings,
        0,
        "frozen dynamic registration unexpectedly retains mappings",
    )?;
    check(
        expected.retained.dms,
        DmsCustody::Absent,
        "frozen dynamic registration unexpectedly retains DMS custody",
    )?;
    check(
        expected.retained.root_deletable,
        false,
        "frozen dynamic registration unexpectedly authorizes in-child root deletion",
    )?;
    Ok(())
}

fn expected_lifecycle_shape(timing: Timing) -> Result<(u8, u8, u8, u8), &'static str> {
    match timing {
        Timing::BeforeCall => Ok((1, 1, 0, 0)),
        Timing::AfterSuccessKnown => Ok((1, 0, 1, 1)),
        _ => Err("dynamic registration runner selected an unsupported frozen timing"),
    }
}

fn check<T: PartialEq>(actual: T, expected: T, error: &'static str) -> Result<(), &'static str> {
    if actual == expected {
        Ok(())
    } else {
        Err(error)
    }
}
