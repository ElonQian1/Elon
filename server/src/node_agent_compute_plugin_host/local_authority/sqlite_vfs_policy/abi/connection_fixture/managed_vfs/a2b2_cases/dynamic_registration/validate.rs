use super::super::{
    case_key::CaseKey,
    invariants,
    model::{
        Case, DmsCustody, EvidenceKind, FailureClass, LogicalRoutePhase, NodePrecondition, Path,
        Phase, RegistrationPhase, RegistryRoutePhase, SqliteOutcome, TargetScope, Timing,
        TopologyKind, UnmapMode,
    },
};
use super::{
    actual::{
        RegistrationShutdownActual, RegistrationShutdownActualCounts,
        RegistrationShutdownActualCustody, RegistrationShutdownActualIdentity,
        RegistrationShutdownActualTopology, RegistrationShutdownDmsCustody,
        RegistrationShutdownFailureClass, RegistrationShutdownLogicalRoutePhase,
        RegistrationShutdownPhase, RegistrationShutdownRegistrationPhase,
        RegistrationShutdownRegistryRoutePhase, RegistrationShutdownSelector,
        RegistrationShutdownTiming,
    },
    record::ValidatedRegistrationShutdownObservation,
};

pub(in super::super::super) fn validate_registration_shutdown_report_payload(
    selector: RegistrationShutdownSelector,
    report_payload: &str,
) -> Result<ValidatedRegistrationShutdownObservation, &'static str> {
    let actual = RegistrationShutdownActual::from_report_payload(report_payload)?;
    validate_registration_shutdown_actual(selector, actual, report_payload.to_owned())
}

fn validate_registration_shutdown_actual(
    selector: RegistrationShutdownSelector,
    actual: RegistrationShutdownActual,
    exact_payload: String,
) -> Result<ValidatedRegistrationShutdownObservation, &'static str> {
    if actual.selector != selector {
        return Err("RegistrationShutdown child selector differs from the parent-selected case");
    }
    let cases = invariants::inventory();
    invariants::validate(&cases)?;
    let expected = select_frozen_case(&cases, selector)?;
    if expected.evidence != EvidenceKind::StaticContract {
        return Err("RegistrationShutdown frozen Case must remain StaticContract");
    }

    validate_identity(actual.identity, expected)?;
    validate_case_state(&actual, expected)?;
    validate_topology(actual.pre, expected.pre, "pre")?;
    validate_topology(actual.post, expected.post, "post")?;
    validate_custody(actual.retained, expected)?;
    validate_counts(actual.counts, expected)?;

    Ok(ValidatedRegistrationShutdownObservation::new(
        selector,
        CaseKey::from(expected),
        exact_payload,
    ))
}

pub(super) fn select_frozen_case(
    cases: &[Case],
    selector: RegistrationShutdownSelector,
) -> Result<&Case, &'static str> {
    let mut matches = cases.iter().filter(|case| selector_matches(selector, case));
    let expected = matches
        .next()
        .ok_or("RegistrationShutdown selector has no frozen Case")?;
    if matches.next().is_some() {
        return Err("RegistrationShutdown selector is not unique in frozen Cases");
    }
    Ok(expected)
}

fn selector_matches(selector: RegistrationShutdownSelector, case: &Case) -> bool {
    if case.path != Path::RegistrationShutdown {
        return false;
    }
    let pair = (case.phase, case.timing);
    match selector {
        RegistrationShutdownSelector::OutstandingCallbackGate => {
            pair == (Phase::OutstandingCallbackGate, Timing::Validation)
        }
        RegistrationShutdownSelector::LiveRouteGate => {
            pair == (Phase::LiveRouteGate, Timing::Validation)
        }
        RegistrationShutdownSelector::QuarantinedCustodyGate => {
            pair == (Phase::QuarantinedCustodyGate, Timing::Validation)
        }
        RegistrationShutdownSelector::RouteIndexObservation => {
            pair == (Phase::RouteIndexObservation, Timing::NativeUncertain)
        }
        RegistrationShutdownSelector::VfsUnregisterBeforeCall => {
            pair == (Phase::VfsUnregister, Timing::BeforeCall)
        }
        RegistrationShutdownSelector::VfsUnregisterNativeRetryable => {
            pair == (Phase::VfsUnregister, Timing::NativeRetryable)
        }
        RegistrationShutdownSelector::VfsUnregisterAfterSuccessKnown => {
            pair == (Phase::VfsUnregister, Timing::AfterSuccessKnown)
        }
        RegistrationShutdownSelector::Success => pair == (Phase::Success, Timing::Success),
    }
}

fn validate_identity(
    actual: RegistrationShutdownActualIdentity,
    expected: &Case,
) -> Result<(), &'static str> {
    let target = expected.target;
    if target.registration_id != 1
        || target.route_ordinal != 0
        || target.runtime_generation != 0
        || target.shm_connection_id != 0
    {
        return Err("RegistrationShutdown frozen target normalization changed");
    }
    if actual.path_is_registration_shutdown != (expected.path == Path::RegistrationShutdown)
        || actual.topology_is_registration_only
            != (expected.topology_kind == TopologyKind::RegistrationOnly)
        || actual.unmap_is_not_applicable != (expected.unmap_mode == UnmapMode::NotApplicable)
        || actual.node_is_not_applicable
            != (expected.node_precondition == NodePrecondition::NotApplicable)
        || actual.variant != expected.variant
        || actual.pre_shared_mask != expected.pre_shared_mask
        || actual.pre_exclusive_mask != expected.pre_exclusive_mask
        || phase(actual.phase) != expected.phase
        || actual.cause_phase_is_none != expected.cause_phase.is_none()
        || timing(actual.timing) != expected.timing
        || failure_class(actual.class) != expected.class
        || actual.target.scope_is_registration != (target.scope == TargetScope::Registration)
        || actual.target.registration_id != target.registration_id
        || !actual.target.route_ordinal_is_not_applicable
        || !actual.target.runtime_generation_is_not_applicable
        || !actual.target.shm_connection_id_is_not_applicable
        || actual.target.role_is_none != target.role.is_none()
        || actual.target.callback_is_none != target.callback.is_none()
        || actual.target.occurrence != target.occurrence
        || actual.sqlite_outcome_is_not_applicable
            != (expected.sqlite_outcome == SqliteOutcome::NotApplicable)
    {
        return Err("RegistrationShutdown identity differs from frozen Case");
    }
    Ok(())
}

fn validate_case_state(
    actual: &RegistrationShutdownActual,
    expected: &Case,
) -> Result<(), &'static str> {
    if actual.mutation_may_have_occurred != expected.mutation_may_have_occurred
        || actual.lock_outcome_uncertain != expected.lock_outcome_uncertain
        || actual.domain_terminal != expected.domain_terminal
        || registry_route_phase(actual.registry_route_phase) != expected.registry_route_phase
        || logical_route_phase(actual.logical_route_phase) != expected.logical_route_phase
        || registration_phase(actual.registration_phase) != expected.registration_phase
        || actual.later_callback_allowed != expected.later_callback_allowed
    {
        return Err("RegistrationShutdown state phases differ from frozen Case");
    }
    Ok(())
}

fn validate_topology(
    actual: RegistrationShutdownActualTopology,
    expected: super::super::model::Topology,
    side: &'static str,
) -> Result<(), &'static str> {
    if actual.sqlite_connections != expected.sqlite_connections
        || actual.shm_connections != expected.shm_connections
        || actual.registry_routes != expected.registry_routes
        || actual.logical_names != expected.logical_names
    {
        return match side {
            "pre" => Err("RegistrationShutdown pre topology differs from frozen Case"),
            _ => Err("RegistrationShutdown post topology differs from frozen Case"),
        };
    }
    Ok(())
}

fn validate_custody(
    actual: RegistrationShutdownActualCustody,
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
        return Err("RegistrationShutdown retained custody differs from frozen Case");
    }
    Ok(())
}

fn validate_counts(
    actual: RegistrationShutdownActualCounts,
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
        return Err("RegistrationShutdown operation counts differ from frozen Case");
    }
    Ok(())
}

fn phase(actual: RegistrationShutdownPhase) -> Phase {
    match actual {
        RegistrationShutdownPhase::OutstandingCallbackGate => Phase::OutstandingCallbackGate,
        RegistrationShutdownPhase::LiveRouteGate => Phase::LiveRouteGate,
        RegistrationShutdownPhase::QuarantinedCustodyGate => Phase::QuarantinedCustodyGate,
        RegistrationShutdownPhase::RouteIndexObservation => Phase::RouteIndexObservation,
        RegistrationShutdownPhase::VfsUnregister => Phase::VfsUnregister,
        RegistrationShutdownPhase::Success => Phase::Success,
    }
}

fn timing(actual: RegistrationShutdownTiming) -> Timing {
    match actual {
        RegistrationShutdownTiming::Validation => Timing::Validation,
        RegistrationShutdownTiming::BeforeCall => Timing::BeforeCall,
        RegistrationShutdownTiming::NativeRetryable => Timing::NativeRetryable,
        RegistrationShutdownTiming::NativeUncertain => Timing::NativeUncertain,
        RegistrationShutdownTiming::AfterSuccessKnown => Timing::AfterSuccessKnown,
        RegistrationShutdownTiming::Success => Timing::Success,
    }
}

fn failure_class(actual: RegistrationShutdownFailureClass) -> FailureClass {
    match actual {
        RegistrationShutdownFailureClass::None => FailureClass::None,
        RegistrationShutdownFailureClass::RegistrationRetained => {
            FailureClass::RegistrationRetained
        }
    }
}

fn registry_route_phase(actual: RegistrationShutdownRegistryRoutePhase) -> RegistryRoutePhase {
    match actual {
        RegistrationShutdownRegistryRoutePhase::Active => RegistryRoutePhase::Active,
        RegistrationShutdownRegistryRoutePhase::Closing => RegistryRoutePhase::Closing,
        RegistrationShutdownRegistryRoutePhase::AwaitingRetirement => {
            RegistryRoutePhase::AwaitingRetirement
        }
        RegistrationShutdownRegistryRoutePhase::Removed => RegistryRoutePhase::Removed,
        RegistrationShutdownRegistryRoutePhase::TerminalQuarantine => {
            RegistryRoutePhase::TerminalQuarantine
        }
    }
}

fn logical_route_phase(actual: RegistrationShutdownLogicalRoutePhase) -> LogicalRoutePhase {
    match actual {
        RegistrationShutdownLogicalRoutePhase::Indexed => LogicalRoutePhase::Indexed,
        RegistrationShutdownLogicalRoutePhase::Removed => LogicalRoutePhase::Removed,
        RegistrationShutdownLogicalRoutePhase::Retained => LogicalRoutePhase::Retained,
    }
}

fn registration_phase(actual: RegistrationShutdownRegistrationPhase) -> RegistrationPhase {
    match actual {
        RegistrationShutdownRegistrationPhase::Registered => RegistrationPhase::Registered,
        RegistrationShutdownRegistrationPhase::Unregistered => RegistrationPhase::Unregistered,
        RegistrationShutdownRegistrationPhase::RetainedRegistered => {
            RegistrationPhase::RetainedRegistered
        }
        RegistrationShutdownRegistrationPhase::RetainedAfterUnregister => {
            RegistrationPhase::RetainedAfterUnregister
        }
    }
}

fn dms_custody(actual: RegistrationShutdownDmsCustody) -> DmsCustody {
    match actual {
        RegistrationShutdownDmsCustody::Absent => DmsCustody::Absent,
        RegistrationShutdownDmsCustody::Shared => DmsCustody::Shared,
        RegistrationShutdownDmsCustody::Released => DmsCustody::Released,
        RegistrationShutdownDmsCustody::OutcomeUncertain => DmsCustody::OutcomeUncertain,
    }
}
