use std::collections::BTreeSet;

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

use super::super::{
    case_key::CaseKey,
    close_physical, close_registry,
    model::{
        CallbackKind, Case, DmsCustody, EvidenceKind, FailureClass, LogicalRoutePhase,
        NodePrecondition, Path, Phase, RegistrationPhase, RegistryRoutePhase, SqliteOutcome,
        TargetScope, Timing, TopologyKind, UnmapMode,
    },
};
use super::{actual::*, record::ValidatedJointCloseObservation};

pub(in super::super::super) fn validate_joint_close_report_payload(
    selector: JointCloseSelector,
    report_payload: &str,
) -> Result<ValidatedJointCloseObservation, &'static str> {
    let actual = JointCloseActual::from_report_payload(report_payload)?;
    if actual.selector != selector {
        return Err("JointClose child selector differs from the parent-selected case");
    }
    let cases = frozen_joint_close_cases();
    validate_frozen_joint_close_exact_set(&cases)?;
    let expected = select_frozen_case(&cases, selector)?;
    if expected.evidence != EvidenceKind::StaticContract {
        return Err("JointClose frozen Case must remain StaticContract");
    }
    let expected_key = independently_frozen_case_key(selector);
    if CaseKey::from(expected) != expected_key {
        return Err("JointClose selected CaseKey differs from independent authority");
    }
    validate_identity(selector, actual.identity, expected)?;
    validate_state(&actual, expected)?;
    validate_topology(actual.pre, expected.pre, "pre")?;
    validate_topology(actual.post, expected.post, "post")?;
    validate_custody(actual.retained, expected)?;
    validate_counts(actual.counts, expected)?;
    Ok(ValidatedJointCloseObservation::new(
        selector,
        actual.identity.target.registration_id,
        expected_key,
        report_payload.to_owned(),
    ))
}

pub(super) fn frozen_joint_close_cases() -> Vec<Case> {
    let mut cases = close_physical::cases();
    cases.extend(
        close_registry::cases()
            .into_iter()
            .filter(|case| case.path == Path::JointClose),
    );
    cases
}

pub(super) fn validate_frozen_joint_close_exact_set(cases: &[Case]) -> Result<(), &'static str> {
    if cases.len() != JointCloseSelector::ALL.len()
        || cases.iter().any(|case| case.path != Path::JointClose)
    {
        return Err("JointClose frozen Case count or family differs from selector authority");
    }
    let actual_keys = cases.iter().map(CaseKey::from).collect::<BTreeSet<_>>();
    let expected_keys = JointCloseSelector::ALL
        .map(independently_frozen_case_key)
        .into_iter()
        .collect::<BTreeSet<_>>();
    if actual_keys.len() != cases.len() || actual_keys != expected_keys {
        return Err("JointClose selectors are not an exact bijection over frozen keys");
    }
    for selector in JointCloseSelector::ALL {
        let case = select_frozen_case(cases, selector)?;
        if case.evidence != EvidenceKind::StaticContract {
            return Err("JointClose frozen Case must remain StaticContract");
        }
    }
    Ok(())
}

pub(super) fn select_frozen_case(
    cases: &[Case],
    selector: JointCloseSelector,
) -> Result<&Case, &'static str> {
    let key = independently_frozen_case_key(selector);
    let mut matches = cases.iter().filter(|case| CaseKey::from(*case) == key);
    let expected = matches
        .next()
        .ok_or("JointClose selector has no frozen Case")?;
    if matches.next().is_some() {
        return Err("JointClose selector is not unique in frozen Cases");
    }
    Ok(expected)
}

fn independently_frozen_case_key(selector: JointCloseSelector) -> CaseKey {
    use FailureClass::{IoBeforeMutation as Io, MutatedButKnown as Mutated};
    use FailureClass::{None as NoFailure, OutcomeUncertainPoisoned as Uncertain};
    use FailureClass::{ProtocolViolation as Protocol, RegistryRejected as Registry};
    use JointCloseSelector as S;
    use Phase::*;
    use Timing::{AfterSuccessKnown as After, AfterSuccessUncertain as AfterUncertain};
    use Timing::{BeforeCall as Before, NativeRetryable, NativeUncertain};
    use Timing::{Success as Succeeded, Validation};

    match selector {
        S::RawStateTakeRejected => key(RawStateTake, Validation, Protocol),
        S::BeginConnectionCloseRejected => key(BeginConnectionClose, Before, Registry),
        S::CallbackAdmissionRejected => key(CallbackAdmission, Before, Registry),
        S::CallbackWrapperBefore => key(MainFileClose, Before, Io).variant(1),
        S::ShmViewUnmapBefore => key(ShmUnmapLift, Before, Io).cause(ViewUnmap),
        S::ShmViewUnmapNativeUncertain => {
            key(ShmUnmapLift, NativeUncertain, Uncertain).cause(ViewUnmap)
        }
        S::ShmViewUnmapAfterKnown => key(ShmUnmapLift, After, Mutated).cause(ViewUnmap),
        S::ShmViewUnmapAfterUncertain => {
            key(ShmUnmapLift, AfterUncertain, Uncertain).cause(ViewUnmap)
        }
        S::ShmMappingCloseBefore => key(ShmUnmapLift, Before, Mutated).cause(MappingClose),
        S::ShmMappingCloseNativeUncertain => {
            key(ShmUnmapLift, NativeUncertain, Uncertain).cause(MappingClose)
        }
        S::ShmMappingCloseAfterKnown => key(ShmUnmapLift, After, Mutated).cause(MappingClose),
        S::ShmMappingCloseAfterUncertain => {
            key(ShmUnmapLift, AfterUncertain, Uncertain).cause(MappingClose)
        }
        S::ShmDmsReleaseBefore => key(ShmUnmapLift, Before, Mutated).cause(DmsSharedRelease),
        S::ShmDmsReleaseNativeUncertain => {
            key(ShmUnmapLift, NativeUncertain, Uncertain).cause(DmsSharedRelease)
        }
        S::ShmDmsReleaseAfterKnown => key(ShmUnmapLift, After, Mutated).cause(DmsSharedRelease),
        S::ShmDmsReleaseAfterUncertain => {
            key(ShmUnmapLift, AfterUncertain, Uncertain).cause(DmsSharedRelease)
        }
        S::ShmFileCloseBefore => key(ShmUnmapLift, Before, Mutated).cause(ShmFileClose),
        S::ShmFileCloseNativeRetryable => {
            key(ShmUnmapLift, NativeRetryable, Uncertain).cause(ShmFileClose)
        }
        S::ShmFileCloseNativeUncertain => {
            key(ShmUnmapLift, NativeUncertain, Uncertain).cause(ShmFileClose)
        }
        S::ShmFileCloseAfterKnown => key(ShmUnmapLift, After, Mutated).cause(ShmFileClose),
        S::ShmFileCloseAfterUncertain => {
            key(ShmUnmapLift, AfterUncertain, Uncertain).cause(ShmFileClose)
        }
        S::ShmDetachBefore => key(ShmUnmapLift, Before, Mutated).cause(ConnectionDetach),
        S::ShmDetachAfterKnown => key(ShmUnmapLift, After, Mutated).cause(ConnectionDetach),
        S::ShmDetachAfterUncertain => {
            key(ShmUnmapLift, AfterUncertain, Uncertain).cause(ConnectionDetach)
        }
        S::MainLockReleaseBefore => key(MainLockRelease, Before, Mutated),
        S::MainLockReleaseNativeUncertainShared => key(MainLockRelease, NativeUncertain, Uncertain),
        S::MainLockReleaseNativeUncertainReserved => {
            key(MainLockRelease, NativeUncertain, Uncertain).variant(1)
        }
        S::MainLockReleaseAfterKnown => key(MainLockRelease, After, Mutated),
        S::MainFileCloseBefore => key(MainFileClose, Before, Mutated),
        S::MainFileCloseNativeRetryable => key(MainFileClose, NativeRetryable, Mutated),
        S::MainFileCloseNativeUncertain => key(MainFileClose, NativeUncertain, Uncertain),
        S::MainFileCloseAfterKnown => key(MainFileClose, After, Mutated),
        S::PhysicalSuccess => key(Success, Succeeded, NoFailure),
        S::RegistryWalMainCloseBefore => key(RegistryWalMainClose, Before, Registry),
        S::RegistryWalMainCloseNativeUncertain => {
            key(RegistryWalMainClose, NativeUncertain, Registry)
        }
        S::RegistryWalMainCloseAfterKnown => key(RegistryWalMainClose, After, Registry),
    }
}

fn key(phase: Phase, timing: Timing, class: FailureClass) -> CaseKey {
    CaseKey::expected(
        Path::JointClose,
        TopologyKind::FinalConnection,
        UnmapMode::Keep,
        phase,
        timing,
        class,
        Some(CallbackKind::Close),
    )
}

fn validate_identity(
    selector: JointCloseSelector,
    actual: JointCloseActualIdentity,
    expected: &Case,
) -> Result<(), &'static str> {
    let target = expected.target;
    let (expected_prestate, expected_offset_class) = main_lock_witness(selector);
    if target.registration_id != 1
        || path(actual.path) != expected.path
        || topology(actual.topology) != expected.topology_kind
        || mode(actual.mode) != expected.unmap_mode
        || node(actual.node) != expected.node_precondition
        || actual.variant != expected.variant
        || actual.pre_shared_mask != expected.pre_shared_mask
        || actual.pre_exclusive_mask != expected.pre_exclusive_mask
        || actual.main_lock_prestate != expected_prestate
        || actual.main_lock_offset_class != expected_offset_class
        || phase(actual.phase) != expected.phase
        || cause(actual.cause) != expected.cause_phase
        || timing(actual.timing) != expected.timing
        || failure_class(actual.class) != expected.class
        || target_scope(actual.target.scope) != target.scope
        || actual.target.registration_id == 0
        || actual.target.route_ordinal != target.route_ordinal
        || actual.target.runtime_generation != target.runtime_generation
        || actual.target.shm_connection_id != target.shm_connection_id
        || role(actual.target.role) != target.role
        || callback(actual.target.callback) != target.callback
        || actual.target.occurrence != target.occurrence
        || sqlite_outcome(actual.sqlite_outcome) != expected.sqlite_outcome
    {
        return Err("JointClose identity differs from frozen Case");
    }
    Ok(())
}

pub(super) const fn main_lock_witness(
    selector: JointCloseSelector,
) -> (JointCloseMainLockPrestate, JointCloseMainLockOffsetClass) {
    match selector {
        JointCloseSelector::MainLockReleaseNativeUncertainShared => (
            JointCloseMainLockPrestate::Shared,
            JointCloseMainLockOffsetClass::SharedRange,
        ),
        JointCloseSelector::MainLockReleaseNativeUncertainReserved => (
            JointCloseMainLockPrestate::ReservedShared,
            JointCloseMainLockOffsetClass::ReservedByte,
        ),
        _ => (
            JointCloseMainLockPrestate::NotApplicable,
            JointCloseMainLockOffsetClass::NotApplicable,
        ),
    }
}

fn validate_state(actual: &JointCloseActual, expected: &Case) -> Result<(), &'static str> {
    if actual.mutation_may_have_occurred != expected.mutation_may_have_occurred
        || actual.lock_outcome_uncertain != expected.lock_outcome_uncertain
        || actual.domain_terminal != expected.domain_terminal
        || registry_route_phase(actual.registry_route_phase) != expected.registry_route_phase
        || logical_route_phase(actual.logical_route_phase) != expected.logical_route_phase
        || registration_phase(actual.registration_phase) != expected.registration_phase
        || actual.later_callback_allowed != expected.later_callback_allowed
    {
        return Err("JointClose state phases differ from frozen Case");
    }
    Ok(())
}

fn validate_topology(
    actual: JointCloseActualTopology,
    expected: super::super::model::Topology,
    side: &'static str,
) -> Result<(), &'static str> {
    if actual.sqlite_connections != expected.sqlite_connections
        || actual.shm_connections != expected.shm_connections
        || actual.registry_routes != expected.registry_routes
        || actual.logical_names != expected.logical_names
    {
        return if side == "pre" {
            Err("JointClose pre topology differs from frozen Case")
        } else {
            Err("JointClose post topology differs from frozen Case")
        };
    }
    Ok(())
}

fn validate_custody(actual: JointCloseActualCustody, expected: &Case) -> Result<(), &'static str> {
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
        return Err("JointClose retained custody differs from frozen Case");
    }
    Ok(())
}

fn validate_counts(actual: JointCloseActualCounts, expected: &Case) -> Result<(), &'static str> {
    let expected = expected.counts;
    let actual = [
        actual.raw_state_take_attempt,
        actual.raw_state_take_success,
        actual.raw_state_abandon,
        actual.methods_clear,
        actual.callback_begin,
        actual.callback_complete_attempt,
        actual.callback_complete_success,
        actual.selected_action_attempt,
        actual.selected_action_success,
        actual.shm_detach,
        actual.main_unlock_attempt,
        actual.main_unlock_success,
        actual.main_file_close_attempt,
        actual.main_file_close_success,
        actual.registry_close_attempt,
        actual.registry_close_success,
        actual.connection_observe_attempt,
        actual.connection_observe_success,
        actual.registry_route_remove_attempt,
        actual.registry_route_remove_success,
        actual.logical_names_remove_attempt,
        actual.logical_names_remove_success,
        actual.logical_names_remove,
        actual.vfs_unregister_attempt,
        actual.vfs_unregister_success,
        actual.fault_observe,
        actual.fault_trigger,
        actual.fault_pending,
        actual.custody_retain,
        actual.physical_retry,
    ];
    let expected = [
        expected.raw_state_take_attempt,
        expected.raw_state_take_success,
        expected.raw_state_abandon,
        expected.methods_clear,
        expected.callback_begin,
        expected.callback_complete_attempt,
        expected.callback_complete_success,
        expected.selected_action_attempt,
        expected.selected_action_success,
        expected.shm_detach,
        expected.main_unlock_attempt,
        expected.main_unlock_success,
        expected.main_file_close_attempt,
        expected.main_file_close_success,
        expected.registry_close_attempt,
        expected.registry_close_success,
        expected.connection_observe_attempt,
        expected.connection_observe_success,
        expected.registry_route_remove_attempt,
        expected.registry_route_remove_success,
        expected.logical_names_remove_attempt,
        expected.logical_names_remove_success,
        expected.logical_names_remove,
        expected.vfs_unregister_attempt,
        expected.vfs_unregister_success,
        expected.fault_observe,
        expected.fault_trigger,
        expected.fault_pending,
        expected.custody_retain,
        expected.physical_retry,
    ];
    if actual != expected {
        return Err("JointClose operation counts differ from frozen Case");
    }
    Ok(())
}

fn path(_: JointClosePath) -> Path {
    Path::JointClose
}
fn topology(_: JointCloseTopology) -> TopologyKind {
    TopologyKind::FinalConnection
}
fn mode(_: JointCloseMode) -> UnmapMode {
    UnmapMode::Keep
}
fn node(_: JointCloseNode) -> NodePrecondition {
    NodePrecondition::Live
}
fn phase(value: JointClosePhase) -> Phase {
    match value {
        JointClosePhase::RawStateTake => Phase::RawStateTake,
        JointClosePhase::BeginConnectionClose => Phase::BeginConnectionClose,
        JointClosePhase::CallbackAdmission => Phase::CallbackAdmission,
        JointClosePhase::ShmUnmapLift => Phase::ShmUnmapLift,
        JointClosePhase::MainLockRelease => Phase::MainLockRelease,
        JointClosePhase::MainFileClose => Phase::MainFileClose,
        JointClosePhase::RegistryWalMainClose => Phase::RegistryWalMainClose,
        JointClosePhase::Success => Phase::Success,
    }
}
fn cause(value: JointCloseCause) -> Option<Phase> {
    match value {
        JointCloseCause::None => None,
        JointCloseCause::ViewUnmap => Some(Phase::ViewUnmap),
        JointCloseCause::MappingClose => Some(Phase::MappingClose),
        JointCloseCause::DmsSharedRelease => Some(Phase::DmsSharedRelease),
        JointCloseCause::ShmFileClose => Some(Phase::ShmFileClose),
        JointCloseCause::ConnectionDetach => Some(Phase::ConnectionDetach),
    }
}
fn timing(value: JointCloseTiming) -> Timing {
    match value {
        JointCloseTiming::Validation => Timing::Validation,
        JointCloseTiming::BeforeCall => Timing::BeforeCall,
        JointCloseTiming::NativeRetryable => Timing::NativeRetryable,
        JointCloseTiming::NativeUncertain => Timing::NativeUncertain,
        JointCloseTiming::AfterSuccessKnown => Timing::AfterSuccessKnown,
        JointCloseTiming::AfterSuccessUncertain => Timing::AfterSuccessUncertain,
        JointCloseTiming::Success => Timing::Success,
    }
}
fn failure_class(value: JointCloseFailureClass) -> FailureClass {
    match value {
        JointCloseFailureClass::None => FailureClass::None,
        JointCloseFailureClass::ProtocolViolation => FailureClass::ProtocolViolation,
        JointCloseFailureClass::IoBeforeMutation => FailureClass::IoBeforeMutation,
        JointCloseFailureClass::MutatedButKnown => FailureClass::MutatedButKnown,
        JointCloseFailureClass::OutcomeUncertainPoisoned => FailureClass::OutcomeUncertainPoisoned,
        JointCloseFailureClass::RegistryRejected => FailureClass::RegistryRejected,
    }
}
fn target_scope(_: JointCloseTargetScope) -> TargetScope {
    TargetScope::RouteMain
}
fn role(_: JointCloseRole) -> Option<ManagedSqliteLogicalFileRole> {
    Some(ManagedSqliteLogicalFileRole::Main)
}
fn callback(_: JointCloseCallback) -> Option<CallbackKind> {
    Some(CallbackKind::Close)
}
fn sqlite_outcome(value: JointCloseSqliteOutcome) -> SqliteOutcome {
    match value {
        JointCloseSqliteOutcome::Ok => SqliteOutcome::Ok,
        JointCloseSqliteOutcome::IoerrClose => SqliteOutcome::IoerrClose,
    }
}
fn registry_route_phase(value: JointCloseRegistryRoutePhase) -> RegistryRoutePhase {
    match value {
        JointCloseRegistryRoutePhase::Active => RegistryRoutePhase::Active,
        JointCloseRegistryRoutePhase::Closing => RegistryRoutePhase::Closing,
        JointCloseRegistryRoutePhase::TerminalQuarantine => RegistryRoutePhase::TerminalQuarantine,
    }
}
fn logical_route_phase(value: JointCloseLogicalRoutePhase) -> LogicalRoutePhase {
    match value {
        JointCloseLogicalRoutePhase::Indexed => LogicalRoutePhase::Indexed,
        JointCloseLogicalRoutePhase::Retained => LogicalRoutePhase::Retained,
    }
}
fn registration_phase(_: JointCloseRegistrationPhase) -> RegistrationPhase {
    RegistrationPhase::Registered
}
fn dms_custody(value: JointCloseDmsCustody) -> DmsCustody {
    match value {
        JointCloseDmsCustody::Absent => DmsCustody::Absent,
        JointCloseDmsCustody::Shared => DmsCustody::Shared,
        JointCloseDmsCustody::Released => DmsCustody::Released,
        JointCloseDmsCustody::OutcomeUncertain => DmsCustody::OutcomeUncertain,
    }
}
