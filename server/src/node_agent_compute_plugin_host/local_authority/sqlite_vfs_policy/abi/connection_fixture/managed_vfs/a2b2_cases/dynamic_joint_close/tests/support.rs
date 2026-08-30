use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

use super::super::super::model::*;
use super::super::{
    actual::*,
    validate::{frozen_joint_close_cases, main_lock_witness, select_frozen_case},
};

pub(super) fn sample_payload() -> String {
    sample_actual(JointCloseSelector::PhysicalSuccess).to_report_payload()
}

pub(super) fn replace_field(payload: &str, index: usize, replacement: &str) -> String {
    let mut fields: Vec<_> = payload.split(',').collect();
    fields[index] = replacement;
    fields.join(",")
}

pub(super) fn main_lock_actual(
    selector: JointCloseSelector,
    variant: u8,
    prestate: JointCloseMainLockPrestate,
    offset_class: JointCloseMainLockOffsetClass,
) -> JointCloseActual {
    let cases = frozen_joint_close_cases();
    let case = select_frozen_case(&cases, selector).expect("main-lock JointClose Case");
    let actual = actual_from_case(selector, case, 37);
    assert_eq!(actual.identity.variant, variant);
    assert_eq!(actual.identity.main_lock_prestate, prestate);
    assert_eq!(actual.identity.main_lock_offset_class, offset_class);
    actual
}

pub(super) fn sample_actual(selector: JointCloseSelector) -> JointCloseActual {
    let cases = frozen_joint_close_cases();
    let case = select_frozen_case(&cases, selector).expect("sample JointClose Case");
    actual_from_case(selector, case, 37)
}

pub(super) fn actual_from_case(
    selector: JointCloseSelector,
    case: &Case,
    registration_id: u64,
) -> JointCloseActual {
    assert_eq!(case.path, Path::JointClose);
    assert_eq!(case.topology_kind, TopologyKind::FinalConnection);
    assert_eq!(case.unmap_mode, UnmapMode::Keep);
    assert_eq!(case.node_precondition, NodePrecondition::Live);
    assert_eq!(case.target.scope, TargetScope::RouteMain);
    assert_eq!(case.target.role, Some(ManagedSqliteLogicalFileRole::Main));
    assert_eq!(case.target.callback, Some(CallbackKind::Close));
    let (main_lock_prestate, main_lock_offset_class) = main_lock_witness(selector);
    JointCloseActual {
        selector,
        identity: JointCloseActualIdentity {
            path: JointClosePath::JointClose,
            topology: JointCloseTopology::FinalConnection,
            mode: JointCloseMode::Keep,
            node: JointCloseNode::Live,
            variant: case.variant,
            pre_shared_mask: case.pre_shared_mask,
            pre_exclusive_mask: case.pre_exclusive_mask,
            main_lock_prestate,
            main_lock_offset_class,
            phase: actual_phase(case.phase),
            cause: actual_cause(case.cause_phase),
            timing: actual_timing(case.timing),
            class: actual_class(case.class),
            target: JointCloseActualTarget {
                scope: JointCloseTargetScope::RouteMain,
                registration_id,
                route_ordinal: case.target.route_ordinal,
                runtime_generation: case.target.runtime_generation,
                shm_connection_id: case.target.shm_connection_id,
                role: JointCloseRole::Main,
                callback: JointCloseCallback::Close,
                occurrence: case.target.occurrence,
            },
            sqlite_outcome: match case.sqlite_outcome {
                SqliteOutcome::Ok => JointCloseSqliteOutcome::Ok,
                SqliteOutcome::IoerrClose => JointCloseSqliteOutcome::IoerrClose,
                _ => panic!("unsupported JointClose SQLite outcome"),
            },
        },
        mutation_may_have_occurred: case.mutation_may_have_occurred,
        lock_outcome_uncertain: case.lock_outcome_uncertain,
        domain_terminal: case.domain_terminal,
        registry_route_phase: match case.registry_route_phase {
            RegistryRoutePhase::Active => JointCloseRegistryRoutePhase::Active,
            RegistryRoutePhase::Closing => JointCloseRegistryRoutePhase::Closing,
            RegistryRoutePhase::TerminalQuarantine => {
                JointCloseRegistryRoutePhase::TerminalQuarantine
            }
            _ => panic!("unsupported JointClose registry route phase"),
        },
        logical_route_phase: match case.logical_route_phase {
            LogicalRoutePhase::Indexed => JointCloseLogicalRoutePhase::Indexed,
            LogicalRoutePhase::Retained => JointCloseLogicalRoutePhase::Retained,
            _ => panic!("unsupported JointClose logical route phase"),
        },
        registration_phase: JointCloseRegistrationPhase::Registered,
        later_callback_allowed: case.later_callback_allowed,
        pre: actual_topology(case.pre),
        post: actual_topology(case.post),
        retained: actual_custody(case.retained),
        counts: actual_counts(case.counts),
    }
}

fn actual_phase(value: Phase) -> JointClosePhase {
    match value {
        Phase::RawStateTake => JointClosePhase::RawStateTake,
        Phase::BeginConnectionClose => JointClosePhase::BeginConnectionClose,
        Phase::CallbackAdmission => JointClosePhase::CallbackAdmission,
        Phase::ShmUnmapLift => JointClosePhase::ShmUnmapLift,
        Phase::MainLockRelease => JointClosePhase::MainLockRelease,
        Phase::MainFileClose => JointClosePhase::MainFileClose,
        Phase::RegistryWalMainClose => JointClosePhase::RegistryWalMainClose,
        Phase::Success => JointClosePhase::Success,
        _ => panic!("unsupported JointClose phase"),
    }
}

fn actual_cause(value: Option<Phase>) -> JointCloseCause {
    match value {
        None => JointCloseCause::None,
        Some(Phase::ViewUnmap) => JointCloseCause::ViewUnmap,
        Some(Phase::MappingClose) => JointCloseCause::MappingClose,
        Some(Phase::DmsSharedRelease) => JointCloseCause::DmsSharedRelease,
        Some(Phase::ShmFileClose) => JointCloseCause::ShmFileClose,
        Some(Phase::ConnectionDetach) => JointCloseCause::ConnectionDetach,
        _ => panic!("unsupported JointClose cause"),
    }
}

fn actual_timing(value: Timing) -> JointCloseTiming {
    match value {
        Timing::Validation => JointCloseTiming::Validation,
        Timing::BeforeCall => JointCloseTiming::BeforeCall,
        Timing::NativeRetryable => JointCloseTiming::NativeRetryable,
        Timing::NativeUncertain => JointCloseTiming::NativeUncertain,
        Timing::AfterSuccessKnown => JointCloseTiming::AfterSuccessKnown,
        Timing::AfterSuccessUncertain => JointCloseTiming::AfterSuccessUncertain,
        Timing::Success => JointCloseTiming::Success,
    }
}

fn actual_class(value: FailureClass) -> JointCloseFailureClass {
    match value {
        FailureClass::None => JointCloseFailureClass::None,
        FailureClass::ProtocolViolation => JointCloseFailureClass::ProtocolViolation,
        FailureClass::IoBeforeMutation => JointCloseFailureClass::IoBeforeMutation,
        FailureClass::MutatedButKnown => JointCloseFailureClass::MutatedButKnown,
        FailureClass::OutcomeUncertainPoisoned => JointCloseFailureClass::OutcomeUncertainPoisoned,
        FailureClass::RegistryRejected => JointCloseFailureClass::RegistryRejected,
        _ => panic!("unsupported JointClose failure class"),
    }
}

fn actual_topology(value: Topology) -> JointCloseActualTopology {
    JointCloseActualTopology {
        sqlite_connections: value.sqlite_connections,
        shm_connections: value.shm_connections,
        registry_routes: value.registry_routes,
        logical_names: value.logical_names,
    }
}

fn actual_custody(value: Custody) -> JointCloseActualCustody {
    JointCloseActualCustody {
        node: value.node,
        views: value.views,
        mappings: value.mappings,
        dms: match value.dms {
            DmsCustody::Absent => JointCloseDmsCustody::Absent,
            DmsCustody::Shared => JointCloseDmsCustody::Shared,
            DmsCustody::Released => JointCloseDmsCustody::Released,
            DmsCustody::OutcomeUncertain => JointCloseDmsCustody::OutcomeUncertain,
        },
        shm_file: value.shm_file,
        main_file: value.main_file,
        main_lock_owner: value.main_lock_owner,
        main_lease: value.main_lease,
        shm_lease: value.shm_lease,
        callback_leases: value.callback_leases,
        registry_entry: value.registry_entry,
        logical_names: value.logical_names,
        vfs_table: value.vfs_table,
        vfs_name: value.vfs_name,
        vfs_context: value.vfs_context,
        root_deletable: value.root_deletable,
    }
}

fn actual_counts(value: Counts) -> JointCloseActualCounts {
    JointCloseActualCounts {
        raw_state_take_attempt: value.raw_state_take_attempt,
        raw_state_take_success: value.raw_state_take_success,
        raw_state_abandon: value.raw_state_abandon,
        methods_clear: value.methods_clear,
        callback_begin: value.callback_begin,
        callback_complete_attempt: value.callback_complete_attempt,
        callback_complete_success: value.callback_complete_success,
        selected_action_attempt: value.selected_action_attempt,
        selected_action_success: value.selected_action_success,
        shm_detach: value.shm_detach,
        main_unlock_attempt: value.main_unlock_attempt,
        main_unlock_success: value.main_unlock_success,
        main_file_close_attempt: value.main_file_close_attempt,
        main_file_close_success: value.main_file_close_success,
        registry_close_attempt: value.registry_close_attempt,
        registry_close_success: value.registry_close_success,
        connection_observe_attempt: value.connection_observe_attempt,
        connection_observe_success: value.connection_observe_success,
        registry_route_remove_attempt: value.registry_route_remove_attempt,
        registry_route_remove_success: value.registry_route_remove_success,
        logical_names_remove_attempt: value.logical_names_remove_attempt,
        logical_names_remove_success: value.logical_names_remove_success,
        logical_names_remove: value.logical_names_remove,
        vfs_unregister_attempt: value.vfs_unregister_attempt,
        vfs_unregister_success: value.vfs_unregister_success,
        fault_observe: value.fault_observe,
        fault_trigger: value.fault_trigger,
        fault_pending: value.fault_pending,
        custody_retain: value.custody_retain,
        physical_retry: value.physical_retry,
    }
}
