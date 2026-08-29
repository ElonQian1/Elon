use std::collections::BTreeSet;

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

use super::super::{
    barrier,
    case_key::CaseKey,
    model::{
        CallbackKind, Case, DmsCustody, EvidenceKind, FailureClass, LogicalRoutePhase,
        NodePrecondition, Path, Phase, RegistrationPhase, RegistryRoutePhase, SqliteOutcome,
        TargetScope, Timing, TopologyKind, UnmapMode,
    },
};
use super::{actual::*, record::ValidatedBarrierObservation};

pub(in super::super::super) fn validate_barrier_report_payload(
    selector: BarrierSelector,
    report_payload: &str,
) -> Result<ValidatedBarrierObservation, &'static str> {
    let actual = BarrierActual::from_report_payload(report_payload)?;
    if actual.selector != selector {
        return Err("Barrier child selector differs from the parent-selected case");
    }
    let cases = barrier::cases();
    validate_frozen_barrier_exact_set(&cases)?;
    let expected = select_frozen_case(&cases, selector)?;
    if expected.evidence != EvidenceKind::StaticContract {
        return Err("Barrier frozen Case must remain StaticContract");
    }
    let expected_key = independently_frozen_case_key(selector);
    if CaseKey::from(expected) != expected_key {
        return Err("Barrier selected CaseKey differs from independent frozen authority");
    }
    validate_identity(actual.identity, expected)?;
    validate_state(&actual, expected)?;
    validate_topology(actual.pre, expected.pre, "pre")?;
    validate_topology(actual.post, expected.post, "post")?;
    validate_custody(actual.retained, expected)?;
    validate_counts(actual.counts, expected)?;
    Ok(ValidatedBarrierObservation::new(
        selector,
        actual.identity.target.registration_id,
        expected_key,
        report_payload.to_owned(),
    ))
}

pub(super) fn validate_frozen_barrier_exact_set(cases: &[Case]) -> Result<(), &'static str> {
    if cases.len() != BarrierSelector::ALL.len() {
        return Err("Barrier frozen Case count differs from exact selector count");
    }
    let mut keys = BTreeSet::new();
    for selector in BarrierSelector::ALL {
        let case = select_frozen_case(cases, selector)?;
        if case.evidence != EvidenceKind::StaticContract || !keys.insert(CaseKey::from(case)) {
            return Err("Barrier selectors are not a bijection over unique StaticContract keys");
        }
    }
    Ok(())
}

pub(super) fn select_frozen_case(
    cases: &[Case],
    selector: BarrierSelector,
) -> Result<&Case, &'static str> {
    let mut matches = cases.iter().filter(|case| selector_matches(selector, case));
    let expected = matches
        .next()
        .ok_or("Barrier selector has no frozen Case")?;
    if matches.next().is_some() {
        return Err("Barrier selector is not unique in frozen Cases");
    }
    Ok(expected)
}

fn selector_matches(selector: BarrierSelector, case: &Case) -> bool {
    CaseKey::from(case) == independently_frozen_case_key(selector)
}

fn independently_frozen_case_key(selector: BarrierSelector) -> CaseKey {
    let (phase, timing, class, variant) = match selector {
        BarrierSelector::AdmissionRejected => (
            Phase::CallbackAdmission,
            Timing::BeforeCall,
            FailureClass::RegistryRejected,
            0,
        ),
        BarrierSelector::WrapperBefore => (
            Phase::BarrierFence,
            Timing::BeforeCall,
            FailureClass::IoBeforeMutation,
            1,
        ),
        BarrierSelector::FenceBefore => (
            Phase::BarrierFence,
            Timing::BeforeCall,
            FailureClass::IoBeforeMutation,
            0,
        ),
        BarrierSelector::FenceAfter => (
            Phase::BarrierFence,
            Timing::AfterSuccessUncertain,
            FailureClass::OutcomeUncertainPoisoned,
            0,
        ),
        BarrierSelector::CompletionBefore => (
            Phase::CallbackCompletion,
            Timing::BeforeCall,
            FailureClass::RegistryRejected,
            0,
        ),
        BarrierSelector::CompletionNativeUncertain => (
            Phase::CallbackCompletion,
            Timing::NativeUncertain,
            FailureClass::RegistryRejected,
            0,
        ),
        BarrierSelector::CompletionAfterSuccessKnown => (
            Phase::CallbackCompletion,
            Timing::AfterSuccessKnown,
            FailureClass::RegistryRejected,
            0,
        ),
        BarrierSelector::Success => (Phase::Success, Timing::Success, FailureClass::None, 0),
    };
    CaseKey::expected(
        Path::Barrier,
        TopologyKind::SharedNonFinal,
        UnmapMode::NotApplicable,
        phase,
        timing,
        class,
        Some(CallbackKind::Shm),
    )
    .variant(variant)
}

fn validate_identity(actual: BarrierActualIdentity, expected: &Case) -> Result<(), &'static str> {
    let target = expected.target;
    if target.registration_id != 1 {
        return Err("Barrier frozen registration normalization changed");
    }
    if actual.path_is_barrier != (expected.path == Path::Barrier)
        || actual.topology_is_shared_non_final
            != (expected.topology_kind == TopologyKind::SharedNonFinal)
        || actual.unmap_is_not_applicable != (expected.unmap_mode == UnmapMode::NotApplicable)
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
        || actual.target.callback_is_shm != (target.callback == Some(CallbackKind::Shm))
        || actual.target.occurrence != target.occurrence
        || actual.sqlite_outcome_is_void_no_result_code
            != (expected.sqlite_outcome == SqliteOutcome::VoidNoResultCode)
    {
        return Err("Barrier identity differs from frozen Case");
    }
    Ok(())
}

fn validate_state(actual: &BarrierActual, expected: &Case) -> Result<(), &'static str> {
    if actual.mutation_may_have_occurred != expected.mutation_may_have_occurred
        || actual.lock_outcome_uncertain != expected.lock_outcome_uncertain
        || actual.domain_terminal != expected.domain_terminal
        || registry_route_phase(actual.registry_route_phase) != expected.registry_route_phase
        || logical_route_phase(actual.logical_route_phase) != expected.logical_route_phase
        || registration_phase(actual.registration_phase) != expected.registration_phase
        || actual.later_callback_allowed != expected.later_callback_allowed
    {
        return Err("Barrier state phases differ from frozen Case");
    }
    Ok(())
}

fn validate_topology(
    actual: BarrierActualTopology,
    expected: super::super::model::Topology,
    side: &'static str,
) -> Result<(), &'static str> {
    if actual.sqlite_connections != expected.sqlite_connections
        || actual.shm_connections != expected.shm_connections
        || actual.registry_routes != expected.registry_routes
        || actual.logical_names != expected.logical_names
    {
        return match side {
            "pre" => Err("Barrier pre topology differs from frozen Case"),
            _ => Err("Barrier post topology differs from frozen Case"),
        };
    }
    Ok(())
}

fn validate_custody(actual: BarrierActualCustody, expected: &Case) -> Result<(), &'static str> {
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
        return Err("Barrier retained custody differs from frozen Case");
    }
    Ok(())
}

fn validate_counts(actual: BarrierActualCounts, expected: &Case) -> Result<(), &'static str> {
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
        return Err("Barrier operation counts differ from frozen Case");
    }
    Ok(())
}

fn phase(value: BarrierPhase) -> Phase {
    match value {
        BarrierPhase::CallbackAdmission => Phase::CallbackAdmission,
        BarrierPhase::BarrierFence => Phase::BarrierFence,
        BarrierPhase::CallbackCompletion => Phase::CallbackCompletion,
        BarrierPhase::Success => Phase::Success,
    }
}

fn timing(value: BarrierTiming) -> Timing {
    match value {
        BarrierTiming::BeforeCall => Timing::BeforeCall,
        BarrierTiming::NativeUncertain => Timing::NativeUncertain,
        BarrierTiming::AfterSuccessKnown => Timing::AfterSuccessKnown,
        BarrierTiming::AfterSuccessUncertain => Timing::AfterSuccessUncertain,
        BarrierTiming::Success => Timing::Success,
    }
}

fn failure_class(value: BarrierFailureClass) -> FailureClass {
    match value {
        BarrierFailureClass::None => FailureClass::None,
        BarrierFailureClass::IoBeforeMutation => FailureClass::IoBeforeMutation,
        BarrierFailureClass::OutcomeUncertainPoisoned => FailureClass::OutcomeUncertainPoisoned,
        BarrierFailureClass::RegistryRejected => FailureClass::RegistryRejected,
    }
}

fn registry_route_phase(value: BarrierRegistryRoutePhase) -> RegistryRoutePhase {
    match value {
        BarrierRegistryRoutePhase::Active => RegistryRoutePhase::Active,
        BarrierRegistryRoutePhase::TerminalQuarantine => RegistryRoutePhase::TerminalQuarantine,
    }
}

fn logical_route_phase(value: BarrierLogicalRoutePhase) -> LogicalRoutePhase {
    match value {
        BarrierLogicalRoutePhase::Indexed => LogicalRoutePhase::Indexed,
        BarrierLogicalRoutePhase::Retained => LogicalRoutePhase::Retained,
    }
}

fn registration_phase(value: BarrierRegistrationPhase) -> RegistrationPhase {
    match value {
        BarrierRegistrationPhase::Registered => RegistrationPhase::Registered,
    }
}

fn dms_custody(value: BarrierDmsCustody) -> DmsCustody {
    match value {
        BarrierDmsCustody::Absent => DmsCustody::Absent,
        BarrierDmsCustody::Shared => DmsCustody::Shared,
        BarrierDmsCustody::Released => DmsCustody::Released,
        BarrierDmsCustody::OutcomeUncertain => DmsCustody::OutcomeUncertain,
    }
}
