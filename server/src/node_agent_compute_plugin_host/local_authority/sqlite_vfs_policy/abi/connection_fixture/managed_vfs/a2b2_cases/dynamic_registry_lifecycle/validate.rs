use std::collections::BTreeSet;

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

use super::super::{
    case_key::CaseKey,
    close_registry,
    model::{
        CallbackKind, Case, DmsCustody, EvidenceKind, FailureClass, LogicalRoutePhase,
        NodePrecondition, Path, Phase, RegistrationPhase, RegistryRoutePhase, SqliteOutcome,
        TargetScope, Timing, TopologyKind, UnmapMode,
    },
};
use super::{actual::*, record::ValidatedRegistryLifecycleObservation};

pub(in super::super::super) fn validate_registry_lifecycle_report_payload(
    selector: RegistryLifecycleSelector,
    report_payload: &str,
) -> Result<ValidatedRegistryLifecycleObservation, &'static str> {
    let actual = RegistryLifecycleActual::from_report_payload(report_payload)?;
    if actual.selector != selector {
        return Err("RegistryLifecycle child selector differs from the parent-selected case");
    }
    let cases = close_registry::cases();
    validate_frozen_registry_lifecycle_exact_set(&cases)?;
    let expected = select_frozen_case(&cases, selector)?;
    if expected.evidence != EvidenceKind::StaticContract {
        return Err("RegistryLifecycle frozen Case must remain StaticContract");
    }
    let expected_key = independently_frozen_case_key(selector);
    if CaseKey::from(expected) != expected_key {
        return Err("RegistryLifecycle selected CaseKey differs from independent authority");
    }
    validate_identity(actual.identity, expected)?;
    validate_state(&actual, expected)?;
    validate_topology(actual.pre, expected.pre, "pre")?;
    validate_topology(actual.post, expected.post, "post")?;
    validate_custody(actual.retained, expected)?;
    validate_counts(actual.counts, expected)?;
    Ok(ValidatedRegistryLifecycleObservation::new(
        selector,
        actual.identity.target.registration_id,
        expected_key,
        report_payload.to_owned(),
    ))
}

pub(super) fn validate_frozen_registry_lifecycle_exact_set(
    cases: &[Case],
) -> Result<(), &'static str> {
    if cases
        .iter()
        .filter(|case| case.path == Path::RegistryLifecycle)
        .count()
        != RegistryLifecycleSelector::ALL.len()
    {
        return Err("RegistryLifecycle frozen Case count differs from selector count");
    }
    let mut keys = BTreeSet::new();
    for selector in RegistryLifecycleSelector::ALL {
        let case = select_frozen_case(cases, selector)?;
        if case.evidence != EvidenceKind::StaticContract || !keys.insert(CaseKey::from(case)) {
            return Err("RegistryLifecycle selectors are not a bijection over frozen keys");
        }
    }
    Ok(())
}

pub(super) fn select_frozen_case(
    cases: &[Case],
    selector: RegistryLifecycleSelector,
) -> Result<&Case, &'static str> {
    let key = independently_frozen_case_key(selector);
    let mut matches = cases.iter().filter(|case| CaseKey::from(*case) == key);
    let expected = matches
        .next()
        .ok_or("RegistryLifecycle selector has no frozen Case")?;
    if matches.next().is_some() {
        return Err("RegistryLifecycle selector is not unique in frozen Cases");
    }
    Ok(expected)
}

fn independently_frozen_case_key(selector: RegistryLifecycleSelector) -> CaseKey {
    use RegistryLifecycleSelector as S;
    let (topology, phase, timing, class, variant) = match selector {
        S::CallbackCompletionBefore => key(Phase::CallbackCompletion, Timing::BeforeCall, 0),
        S::CallbackCompletionNativeUncertain => {
            key(Phase::CallbackCompletion, Timing::NativeUncertain, 0)
        }
        S::CallbackCompletionAfterSuccessKnown => {
            key(Phase::CallbackCompletion, Timing::AfterSuccessKnown, 0)
        }
        S::ConnectionObservationBefore => key(Phase::ConnectionObservation, Timing::BeforeCall, 0),
        S::ConnectionObservationOutstandingSidecar => {
            key(Phase::ConnectionObservation, Timing::Validation, 1)
        }
        S::ConnectionObservationAfterSuccessKnown => {
            key(Phase::ConnectionObservation, Timing::AfterSuccessKnown, 0)
        }
        S::RegistryRouteRemovalBefore => key(Phase::RegistryRouteRemoval, Timing::BeforeCall, 0),
        S::RegistryRouteRemovalOwnerNative => {
            key(Phase::RegistryRouteRemoval, Timing::NativeUncertain, 1)
        }
        S::RegistryRouteRemovalPublishNative => {
            key(Phase::RegistryRouteRemoval, Timing::NativeUncertain, 2)
        }
        S::RegistryRouteRemovalAfterSuccessKnown => {
            key(Phase::RegistryRouteRemoval, Timing::AfterSuccessKnown, 0)
        }
        S::LogicalRouteRemovalBefore => key(Phase::LogicalRouteRemoval, Timing::BeforeCall, 0),
        S::LogicalRouteRemovalClaimNative => {
            key(Phase::LogicalRouteRemoval, Timing::NativeUncertain, 1)
        }
        S::LogicalRouteRemovalIndexNative => {
            key(Phase::LogicalRouteRemoval, Timing::NativeUncertain, 2)
        }
        S::LogicalRouteRemovalAfterSuccessKnown => {
            key(Phase::LogicalRouteRemoval, Timing::AfterSuccessKnown, 0)
        }
        S::SuccessSharedNonFinal => (
            TopologyKind::SharedNonFinal,
            Phase::Success,
            Timing::Success,
            FailureClass::None,
            0,
        ),
        S::SuccessFinal => (
            TopologyKind::FinalConnection,
            Phase::Success,
            Timing::Success,
            FailureClass::None,
            0,
        ),
    };
    CaseKey::expected(
        Path::RegistryLifecycle,
        topology,
        UnmapMode::Keep,
        phase,
        timing,
        class,
        Some(CallbackKind::Close),
    )
    .variant(variant)
}

fn key(
    phase: Phase,
    timing: Timing,
    variant: u8,
) -> (TopologyKind, Phase, Timing, FailureClass, u8) {
    (
        TopologyKind::FinalConnection,
        phase,
        timing,
        FailureClass::RegistryRejected,
        variant,
    )
}

fn validate_identity(
    actual: RegistryLifecycleActualIdentity,
    expected: &Case,
) -> Result<(), &'static str> {
    let target = expected.target;
    if target.registration_id != 1
        || actual.path_is_registry_lifecycle != (expected.path == Path::RegistryLifecycle)
        || actual.topology_is_shared_non_final
            != (expected.topology_kind == TopologyKind::SharedNonFinal)
        || actual.unmap_is_keep != (expected.unmap_mode == UnmapMode::Keep)
        || actual.node_is_live != (expected.node_precondition == NodePrecondition::Live)
        || actual.variant != expected.variant
        || actual.pre_shared_mask != expected.pre_shared_mask
        || actual.pre_exclusive_mask != expected.pre_exclusive_mask
        || phase(actual.phase) != expected.phase
        || actual.cause_phase_is_none != expected.cause_phase.is_none()
        || timing(actual.timing) != expected.timing
        || failure_class(actual.class) != expected.class
        || actual.target.scope_is_route_main != (target.scope == TargetScope::RouteMain)
        || actual.target.registration_id == 0
        || actual.target.route_ordinal != target.route_ordinal
        || actual.target.runtime_generation != target.runtime_generation
        || actual.target.shm_connection_id != target.shm_connection_id
        || actual.target.role_is_main != (target.role == Some(ManagedSqliteLogicalFileRole::Main))
        || actual.target.callback_is_close != (target.callback == Some(CallbackKind::Close))
        || actual.target.occurrence != target.occurrence
        || sqlite_outcome(actual.sqlite_outcome) != expected.sqlite_outcome
    {
        return Err("RegistryLifecycle identity differs from frozen Case");
    }
    Ok(())
}

fn validate_state(actual: &RegistryLifecycleActual, expected: &Case) -> Result<(), &'static str> {
    if actual.mutation_may_have_occurred != expected.mutation_may_have_occurred
        || actual.lock_outcome_uncertain != expected.lock_outcome_uncertain
        || actual.domain_terminal != expected.domain_terminal
        || registry_route_phase(actual.registry_route_phase) != expected.registry_route_phase
        || logical_route_phase(actual.logical_route_phase) != expected.logical_route_phase
        || registration_phase(actual.registration_phase) != expected.registration_phase
        || actual.later_callback_allowed != expected.later_callback_allowed
    {
        return Err("RegistryLifecycle state phases differ from frozen Case");
    }
    Ok(())
}

fn validate_topology(
    actual: RegistryLifecycleActualTopology,
    expected: super::super::model::Topology,
    side: &'static str,
) -> Result<(), &'static str> {
    if actual.sqlite_connections != expected.sqlite_connections
        || actual.shm_connections != expected.shm_connections
        || actual.registry_routes != expected.registry_routes
        || actual.logical_names != expected.logical_names
    {
        return if side == "pre" {
            Err("RegistryLifecycle pre topology differs from frozen Case")
        } else {
            Err("RegistryLifecycle post topology differs from frozen Case")
        };
    }
    Ok(())
}

fn validate_custody(
    actual: RegistryLifecycleActualCustody,
    expected: &Case,
) -> Result<(), &'static str> {
    let expected = expected.retained;
    if actual.node != expected.node
        || actual.views != expected.views
        || actual.mappings != expected.mappings
        || dms_custody(actual.dms) != expected.dms
        || actual.shm_file != expected.shm_file
        || actual.main_file != expected.main_file
        || actual.main_lock_owner != expected.main_lock_owner
        || actual.main_lease != expected.main_lease
        || actual.shm_lease != expected.shm_lease
        || actual.callback_leases != expected.callback_leases
        || actual.registry_entry != expected.registry_entry
        || actual.logical_names != expected.logical_names
        || actual.vfs_table != expected.vfs_table
        || actual.vfs_name != expected.vfs_name
        || actual.vfs_context != expected.vfs_context
        || actual.root_deletable != expected.root_deletable
    {
        return Err("RegistryLifecycle retained custody differs from frozen Case");
    }
    Ok(())
}

fn validate_counts(
    actual: RegistryLifecycleActualCounts,
    expected: &Case,
) -> Result<(), &'static str> {
    let expected = expected.counts;
    if actual.raw_state_take_attempt != expected.raw_state_take_attempt
        || actual.raw_state_take_success != expected.raw_state_take_success
        || actual.raw_state_abandon != expected.raw_state_abandon
        || actual.methods_clear != expected.methods_clear
        || actual.callback_begin != expected.callback_begin
        || actual.callback_complete_attempt != expected.callback_complete_attempt
        || actual.callback_complete_success != expected.callback_complete_success
        || actual.selected_action_attempt != expected.selected_action_attempt
        || actual.selected_action_success != expected.selected_action_success
        || actual.shm_detach != expected.shm_detach
        || actual.main_unlock_attempt != expected.main_unlock_attempt
        || actual.main_unlock_success != expected.main_unlock_success
        || actual.main_file_close_attempt != expected.main_file_close_attempt
        || actual.main_file_close_success != expected.main_file_close_success
        || actual.registry_close_attempt != expected.registry_close_attempt
        || actual.registry_close_success != expected.registry_close_success
        || actual.connection_observe_attempt != expected.connection_observe_attempt
        || actual.connection_observe_success != expected.connection_observe_success
        || actual.registry_route_remove_attempt != expected.registry_route_remove_attempt
        || actual.registry_route_remove_success != expected.registry_route_remove_success
        || actual.logical_names_remove_attempt != expected.logical_names_remove_attempt
        || actual.logical_names_remove_success != expected.logical_names_remove_success
        || actual.logical_names_remove != expected.logical_names_remove
        || actual.vfs_unregister_attempt != expected.vfs_unregister_attempt
        || actual.vfs_unregister_success != expected.vfs_unregister_success
        || actual.fault_observe != expected.fault_observe
        || actual.fault_trigger != expected.fault_trigger
        || actual.fault_pending != expected.fault_pending
        || actual.custody_retain != expected.custody_retain
        || actual.physical_retry != expected.physical_retry
    {
        return Err("RegistryLifecycle operation counts differ from frozen Case");
    }
    Ok(())
}

fn phase(value: RegistryLifecyclePhase) -> Phase {
    match value {
        RegistryLifecyclePhase::CallbackCompletion => Phase::CallbackCompletion,
        RegistryLifecyclePhase::ConnectionObservation => Phase::ConnectionObservation,
        RegistryLifecyclePhase::RegistryRouteRemoval => Phase::RegistryRouteRemoval,
        RegistryLifecyclePhase::LogicalRouteRemoval => Phase::LogicalRouteRemoval,
        RegistryLifecyclePhase::Success => Phase::Success,
    }
}

fn timing(value: RegistryLifecycleTiming) -> Timing {
    match value {
        RegistryLifecycleTiming::Validation => Timing::Validation,
        RegistryLifecycleTiming::BeforeCall => Timing::BeforeCall,
        RegistryLifecycleTiming::NativeUncertain => Timing::NativeUncertain,
        RegistryLifecycleTiming::AfterSuccessKnown => Timing::AfterSuccessKnown,
        RegistryLifecycleTiming::Success => Timing::Success,
    }
}

fn failure_class(value: RegistryLifecycleFailureClass) -> FailureClass {
    match value {
        RegistryLifecycleFailureClass::None => FailureClass::None,
        RegistryLifecycleFailureClass::RegistryRejected => FailureClass::RegistryRejected,
    }
}

fn sqlite_outcome(value: RegistryLifecycleSqliteOutcome) -> SqliteOutcome {
    match value {
        RegistryLifecycleSqliteOutcome::Ok => SqliteOutcome::Ok,
        RegistryLifecycleSqliteOutcome::IoerrClose => SqliteOutcome::IoerrClose,
        RegistryLifecycleSqliteOutcome::NotApplicable => SqliteOutcome::NotApplicable,
    }
}

fn registry_route_phase(value: RegistryLifecycleRegistryRoutePhase) -> RegistryRoutePhase {
    match value {
        RegistryLifecycleRegistryRoutePhase::Active => RegistryRoutePhase::Active,
        RegistryLifecycleRegistryRoutePhase::AwaitingRetirement => {
            RegistryRoutePhase::AwaitingRetirement
        }
        RegistryLifecycleRegistryRoutePhase::Removed => RegistryRoutePhase::Removed,
        RegistryLifecycleRegistryRoutePhase::TerminalQuarantine => {
            RegistryRoutePhase::TerminalQuarantine
        }
    }
}

fn logical_route_phase(value: RegistryLifecycleLogicalRoutePhase) -> LogicalRoutePhase {
    match value {
        RegistryLifecycleLogicalRoutePhase::Indexed => LogicalRoutePhase::Indexed,
        RegistryLifecycleLogicalRoutePhase::Removed => LogicalRoutePhase::Removed,
        RegistryLifecycleLogicalRoutePhase::Retained => LogicalRoutePhase::Retained,
    }
}

fn registration_phase(value: RegistryLifecycleRegistrationPhase) -> RegistrationPhase {
    match value {
        RegistryLifecycleRegistrationPhase::Registered => RegistrationPhase::Registered,
    }
}

fn dms_custody(value: RegistryLifecycleDmsCustody) -> DmsCustody {
    match value {
        RegistryLifecycleDmsCustody::Absent => DmsCustody::Absent,
        RegistryLifecycleDmsCustody::Shared => DmsCustody::Shared,
        RegistryLifecycleDmsCustody::Released => DmsCustody::Released,
        RegistryLifecycleDmsCustody::OutcomeUncertain => DmsCustody::OutcomeUncertain,
    }
}
