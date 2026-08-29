use std::collections::BTreeSet;

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

use super::super::{
    case_key::CaseKey,
    model::{
        CallbackKind, Case, DmsCustody, EvidenceKind, FailureClass, LogicalRoutePhase,
        NodePrecondition, Path, Phase, RegistrationPhase, RegistryRoutePhase, SqliteOutcome,
        TargetScope, Timing, TopologyKind, UnmapMode,
    },
    unmap_delete, unmap_nonfinal, unmap_teardown,
};
use super::{actual::*, record::ValidatedUnmapObservation};

pub(in super::super::super) fn validate_unmap_report_payload(
    selector: UnmapSelector,
    report_payload: &str,
) -> Result<ValidatedUnmapObservation, &'static str> {
    let actual = UnmapActual::from_report_payload(report_payload)?;
    if actual.selector != selector {
        return Err("Unmap child selector differs from the parent-selected case");
    }
    let cases = frozen_unmap_cases();
    validate_frozen_unmap_exact_set(&cases)?;
    let expected = select_frozen_case(&cases, selector)?;
    if expected.evidence != EvidenceKind::StaticContract {
        return Err("Unmap frozen Case must remain StaticContract");
    }
    let expected_key = independently_frozen_case_key(selector);
    if CaseKey::from(expected) != expected_key {
        return Err("Unmap selected CaseKey differs from independent authority");
    }
    validate_identity(actual.identity, expected)?;
    validate_state(&actual, expected)?;
    validate_topology(actual.pre, expected.pre, "pre")?;
    validate_topology(actual.post, expected.post, "post")?;
    validate_custody(actual.retained, expected)?;
    validate_counts(actual.counts, expected)?;
    Ok(ValidatedUnmapObservation::new(
        selector,
        actual.identity.target.registration_id,
        expected_key,
        report_payload.to_owned(),
    ))
}

pub(super) fn frozen_unmap_cases() -> Vec<Case> {
    let mut cases = unmap_nonfinal::cases();
    cases.extend(unmap_teardown::cases());
    cases.extend(unmap_delete::cases());
    cases
}

pub(super) fn validate_frozen_unmap_exact_set(cases: &[Case]) -> Result<(), &'static str> {
    if cases.len() != UnmapSelector::ALL.len() || cases.iter().any(|case| case.path != Path::Unmap)
    {
        return Err("Unmap frozen Case count or family differs from selector authority");
    }
    let mut keys = BTreeSet::new();
    for selector in UnmapSelector::ALL {
        let case = select_frozen_case(cases, selector)?;
        if case.evidence != EvidenceKind::StaticContract || !keys.insert(CaseKey::from(case)) {
            return Err("Unmap selectors are not a bijection over frozen keys");
        }
    }
    Ok(())
}

pub(super) fn select_frozen_case(
    cases: &[Case],
    selector: UnmapSelector,
) -> Result<&Case, &'static str> {
    let key = independently_frozen_case_key(selector);
    let mut matches = cases.iter().filter(|case| CaseKey::from(*case) == key);
    let expected = matches.next().ok_or("Unmap selector has no frozen Case")?;
    if matches.next().is_some() {
        return Err("Unmap selector is not unique in frozen Cases");
    }
    Ok(expected)
}

fn independently_frozen_case_key(selector: UnmapSelector) -> CaseKey {
    use FailureClass::{IoBeforeMutation as Io, MutatedButKnown as Mutated};
    use FailureClass::{None as NoFailure, OutcomeUncertainPoisoned as Uncertain};
    use FailureClass::{ProtocolViolation as Protocol, RegistryRejected as Registry};
    use Phase::*;
    use Timing::{AfterSuccessKnown as After, AfterSuccessUncertain as AfterUncertain};
    use Timing::{BeforeCall as Before, NativeRetryable, NativeUncertain};
    use Timing::{Success as Succeeded, Validation};
    use TopologyKind::{FinalConnection as Final, SharedNonFinal as Shared};
    use UnmapMode::{Delete, Keep};
    use UnmapSelector as S;

    match selector {
        S::SharedDeleteRequestValidation => {
            key(Shared, Delete, RequestValidation, Validation, Protocol)
        }
        S::SharedKeepCallbackAdmission => key(Shared, Keep, CallbackAdmission, Before, Registry),
        S::SharedKeepCallbackWrapperBefore => {
            key(Shared, Keep, ConnectionDetach, Before, Io).variant(1)
        }
        S::SharedKeepHeldSharedLock => {
            key(Shared, Keep, HeldLockGate, Validation, Protocol).masks(1, 0)
        }
        S::SharedKeepHeldExclusiveLock => {
            key(Shared, Keep, HeldLockGate, Validation, Protocol).masks(0, 1)
        }
        S::SharedKeepDetachBefore => key(Shared, Keep, ConnectionDetach, Before, Io),
        S::SharedKeepDetachAfterKnown => key(Shared, Keep, ConnectionDetach, After, Mutated),
        S::SharedKeepDetachAfterUncertain => {
            key(Shared, Keep, ConnectionDetach, AfterUncertain, Uncertain)
        }
        S::SharedKeepCompletionNativeUncertain => {
            key(Shared, Keep, CallbackCompletion, NativeUncertain, Registry)
        }
        S::SharedKeepSuccess => key(Shared, Keep, Success, Succeeded, NoFailure),
        S::SharedDeleteSuccess => key(Shared, Delete, Success, Succeeded, NoFailure),
        S::FinalKeepViewUnmapBefore => key(Final, Keep, ViewUnmap, Before, Io),
        S::FinalKeepViewUnmapNativeUncertain => {
            key(Final, Keep, ViewUnmap, NativeUncertain, Uncertain)
        }
        S::FinalKeepViewUnmapAfterKnown => key(Final, Keep, ViewUnmap, After, Mutated),
        S::FinalKeepViewUnmapAfterUncertain => {
            key(Final, Keep, ViewUnmap, AfterUncertain, Uncertain)
        }
        S::FinalKeepMappingCloseBefore => key(Final, Keep, MappingClose, Before, Mutated),
        S::FinalKeepMappingCloseNativeUncertain => {
            key(Final, Keep, MappingClose, NativeUncertain, Uncertain)
        }
        S::FinalKeepMappingCloseAfterKnown => key(Final, Keep, MappingClose, After, Mutated),
        S::FinalKeepMappingCloseAfterUncertain => {
            key(Final, Keep, MappingClose, AfterUncertain, Uncertain)
        }
        S::FinalKeepDmsReleaseBefore => key(Final, Keep, DmsSharedRelease, Before, Mutated),
        S::FinalKeepDmsReleaseNativeUncertain => {
            key(Final, Keep, DmsSharedRelease, NativeUncertain, Uncertain)
        }
        S::FinalKeepDmsReleaseAfterKnown => key(Final, Keep, DmsSharedRelease, After, Mutated),
        S::FinalKeepDmsReleaseAfterUncertain => {
            key(Final, Keep, DmsSharedRelease, AfterUncertain, Uncertain)
        }
        S::FinalKeepFileCloseBefore => key(Final, Keep, ShmFileClose, Before, Mutated),
        S::FinalKeepFileCloseNativeRetryable => {
            key(Final, Keep, ShmFileClose, NativeRetryable, Uncertain)
        }
        S::FinalKeepFileCloseNativeUncertain => {
            key(Final, Keep, ShmFileClose, NativeUncertain, Uncertain)
        }
        S::FinalKeepFileCloseAfterKnown => key(Final, Keep, ShmFileClose, After, Mutated),
        S::FinalKeepFileCloseAfterUncertain => {
            key(Final, Keep, ShmFileClose, AfterUncertain, Uncertain)
        }
        S::FinalKeepDetachBefore => key(Final, Keep, ConnectionDetach, Before, Mutated),
        S::FinalKeepDetachAfterKnown => key(Final, Keep, ConnectionDetach, After, Mutated),
        S::FinalKeepDetachAfterUncertain => {
            key(Final, Keep, ConnectionDetach, AfterUncertain, Uncertain)
        }
        S::FinalKeepCompletionNativeUncertain => {
            key(Final, Keep, CallbackCompletion, NativeUncertain, Registry)
        }
        S::FinalKeepSuccessLiveNode => key(Final, Keep, Success, Succeeded, NoFailure),
        S::FinalKeepSuccessNodeAbsent => {
            key(Final, Keep, Success, Succeeded, NoFailure).node(NodePrecondition::Absent)
        }
        S::FinalDeleteAuthMainIdentityMissing => {
            key(Final, Delete, DeleteAuthorization, Validation, Protocol).variant(1)
        }
        S::FinalDeleteAuthMainOrGenerationMismatch => {
            key(Final, Delete, DeleteAuthorization, Validation, Protocol).variant(2)
        }
        S::FinalDeleteAuthMainNotExclusive => {
            key(Final, Delete, DeleteAuthorization, Validation, Protocol).variant(3)
        }
        S::FinalDeleteAuthLockStateUncertain => {
            key(Final, Delete, DeleteAuthorization, Validation, Uncertain).variant(4)
        }
        S::FinalDeleteSiblingBefore => key(Final, Delete, ExactSiblingDelete, Before, Mutated),
        S::FinalDeleteSiblingNativeRetryable => key(
            Final,
            Delete,
            ExactSiblingDelete,
            NativeRetryable,
            Uncertain,
        ),
        S::FinalDeleteSiblingNativeUncertain => key(
            Final,
            Delete,
            ExactSiblingDelete,
            NativeUncertain,
            Uncertain,
        ),
        S::FinalDeleteSiblingAfterKnown => key(Final, Delete, ExactSiblingDelete, After, Mutated),
        S::FinalDeleteSiblingAfterUncertain => {
            key(Final, Delete, ExactSiblingDelete, AfterUncertain, Uncertain)
        }
        S::FinalDeleteDetachBefore => {
            key(Final, Delete, ConnectionDetach, Before, Mutated).variant(1)
        }
        S::FinalDeleteDetachAfterKnown => {
            key(Final, Delete, ConnectionDetach, After, Mutated).variant(1)
        }
        S::FinalDeleteDetachAfterUncertain => {
            key(Final, Delete, ConnectionDetach, AfterUncertain, Uncertain).variant(1)
        }
        S::FinalDeleteCompletionNativeUncertain => {
            key(Final, Delete, CallbackCompletion, NativeUncertain, Registry).variant(1)
        }
        S::FinalDeleteSuccessDeleted => key(Final, Delete, Success, Succeeded, NoFailure),
        S::FinalDeleteSuccessNotFound => {
            key(Final, Delete, Success, Succeeded, NoFailure).variant(1)
        }
    }
}

fn key(
    topology: TopologyKind,
    mode: UnmapMode,
    phase: Phase,
    timing: Timing,
    class: FailureClass,
) -> CaseKey {
    CaseKey::expected(
        Path::Unmap,
        topology,
        mode,
        phase,
        timing,
        class,
        Some(CallbackKind::Shm),
    )
}

fn validate_identity(actual: UnmapActualIdentity, expected: &Case) -> Result<(), &'static str> {
    let target = expected.target;
    if target.registration_id != 1
        || path(actual.path) != expected.path
        || topology(actual.topology) != expected.topology_kind
        || mode(actual.mode) != expected.unmap_mode
        || node(actual.node) != expected.node_precondition
        || actual.variant != expected.variant
        || actual.pre_shared_mask != expected.pre_shared_mask
        || actual.pre_exclusive_mask != expected.pre_exclusive_mask
        || phase(actual.phase) != expected.phase
        || actual.cause != UnmapCause::None
        || expected.cause_phase.is_some()
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
        return Err("Unmap identity differs from frozen Case");
    }
    Ok(())
}

fn validate_state(actual: &UnmapActual, expected: &Case) -> Result<(), &'static str> {
    if actual.mutation_may_have_occurred != expected.mutation_may_have_occurred
        || actual.lock_outcome_uncertain != expected.lock_outcome_uncertain
        || actual.domain_terminal != expected.domain_terminal
        || registry_route_phase(actual.registry_route_phase) != expected.registry_route_phase
        || logical_route_phase(actual.logical_route_phase) != expected.logical_route_phase
        || registration_phase(actual.registration_phase) != expected.registration_phase
        || actual.later_callback_allowed != expected.later_callback_allowed
    {
        return Err("Unmap state phases differ from frozen Case");
    }
    Ok(())
}

fn validate_topology(
    actual: UnmapActualTopology,
    expected: super::super::model::Topology,
    side: &'static str,
) -> Result<(), &'static str> {
    if actual.sqlite_connections != expected.sqlite_connections
        || actual.shm_connections != expected.shm_connections
        || actual.registry_routes != expected.registry_routes
        || actual.logical_names != expected.logical_names
    {
        return if side == "pre" {
            Err("Unmap pre topology differs from frozen Case")
        } else {
            Err("Unmap post topology differs from frozen Case")
        };
    }
    Ok(())
}

fn validate_custody(actual: UnmapActualCustody, expected: &Case) -> Result<(), &'static str> {
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
        return Err("Unmap retained custody differs from frozen Case");
    }
    Ok(())
}

fn validate_counts(actual: UnmapActualCounts, expected: &Case) -> Result<(), &'static str> {
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
        return Err("Unmap operation counts differ from frozen Case");
    }
    Ok(())
}

fn path(_: UnmapPath) -> Path {
    Path::Unmap
}
fn topology(value: UnmapTopology) -> TopologyKind {
    match value {
        UnmapTopology::SharedNonFinal => TopologyKind::SharedNonFinal,
        UnmapTopology::FinalConnection => TopologyKind::FinalConnection,
    }
}
fn mode(value: super::actual::UnmapMode) -> UnmapMode {
    match value {
        super::actual::UnmapMode::Keep => UnmapMode::Keep,
        super::actual::UnmapMode::Delete => UnmapMode::Delete,
    }
}
fn node(value: UnmapNode) -> NodePrecondition {
    match value {
        UnmapNode::Live => NodePrecondition::Live,
        UnmapNode::Absent => NodePrecondition::Absent,
    }
}
fn phase(value: UnmapPhase) -> Phase {
    match value {
        UnmapPhase::RequestValidation => Phase::RequestValidation,
        UnmapPhase::CallbackAdmission => Phase::CallbackAdmission,
        UnmapPhase::HeldLockGate => Phase::HeldLockGate,
        UnmapPhase::ConnectionDetach => Phase::ConnectionDetach,
        UnmapPhase::ViewUnmap => Phase::ViewUnmap,
        UnmapPhase::MappingClose => Phase::MappingClose,
        UnmapPhase::DmsSharedRelease => Phase::DmsSharedRelease,
        UnmapPhase::ShmFileClose => Phase::ShmFileClose,
        UnmapPhase::DeleteAuthorization => Phase::DeleteAuthorization,
        UnmapPhase::ExactSiblingDelete => Phase::ExactSiblingDelete,
        UnmapPhase::CallbackCompletion => Phase::CallbackCompletion,
        UnmapPhase::Success => Phase::Success,
    }
}
fn timing(value: UnmapTiming) -> Timing {
    match value {
        UnmapTiming::Validation => Timing::Validation,
        UnmapTiming::BeforeCall => Timing::BeforeCall,
        UnmapTiming::NativeRetryable => Timing::NativeRetryable,
        UnmapTiming::NativeUncertain => Timing::NativeUncertain,
        UnmapTiming::AfterSuccessKnown => Timing::AfterSuccessKnown,
        UnmapTiming::AfterSuccessUncertain => Timing::AfterSuccessUncertain,
        UnmapTiming::Success => Timing::Success,
    }
}
fn failure_class(value: UnmapFailureClass) -> FailureClass {
    match value {
        UnmapFailureClass::None => FailureClass::None,
        UnmapFailureClass::ProtocolViolation => FailureClass::ProtocolViolation,
        UnmapFailureClass::IoBeforeMutation => FailureClass::IoBeforeMutation,
        UnmapFailureClass::MutatedButKnown => FailureClass::MutatedButKnown,
        UnmapFailureClass::OutcomeUncertainPoisoned => FailureClass::OutcomeUncertainPoisoned,
        UnmapFailureClass::RegistryRejected => FailureClass::RegistryRejected,
    }
}
fn target_scope(_: UnmapTargetScope) -> TargetScope {
    TargetScope::RouteMain
}
fn role(_: UnmapRole) -> Option<ManagedSqliteLogicalFileRole> {
    Some(ManagedSqliteLogicalFileRole::Main)
}
fn callback(_: UnmapCallback) -> Option<CallbackKind> {
    Some(CallbackKind::Shm)
}
fn sqlite_outcome(value: UnmapSqliteOutcome) -> SqliteOutcome {
    match value {
        UnmapSqliteOutcome::Ok => SqliteOutcome::Ok,
        UnmapSqliteOutcome::Ioerr => SqliteOutcome::Ioerr,
    }
}
fn registry_route_phase(value: UnmapRegistryRoutePhase) -> RegistryRoutePhase {
    match value {
        UnmapRegistryRoutePhase::Active => RegistryRoutePhase::Active,
        UnmapRegistryRoutePhase::TerminalQuarantine => RegistryRoutePhase::TerminalQuarantine,
    }
}
fn logical_route_phase(value: UnmapLogicalRoutePhase) -> LogicalRoutePhase {
    match value {
        UnmapLogicalRoutePhase::Indexed => LogicalRoutePhase::Indexed,
        UnmapLogicalRoutePhase::Retained => LogicalRoutePhase::Retained,
    }
}
fn registration_phase(_: UnmapRegistrationPhase) -> RegistrationPhase {
    RegistrationPhase::Registered
}
fn dms_custody(value: UnmapDmsCustody) -> DmsCustody {
    match value {
        UnmapDmsCustody::Absent => DmsCustody::Absent,
        UnmapDmsCustody::Shared => DmsCustody::Shared,
        UnmapDmsCustody::Released => DmsCustody::Released,
        UnmapDmsCustody::OutcomeUncertain => DmsCustody::OutcomeUncertain,
    }
}
