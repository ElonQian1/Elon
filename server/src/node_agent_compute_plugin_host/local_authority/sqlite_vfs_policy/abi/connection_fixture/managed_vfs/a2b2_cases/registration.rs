use super::model::{
    base, failure, native_observed, Case, DmsCustody, FailureClass, LogicalRoutePhase,
    NodePrecondition, Path, Phase, RegistrationPhase, RegistryRoutePhase, SqliteOutcome,
    TargetScope, Timing, TopologyKind, EMPTY,
};

pub(super) fn cases() -> Vec<Case> {
    vec![
        gate(Phase::OutstandingCallbackGate, 1),
        gate(Phase::LiveRouteGate, 2),
        gate(Phase::QuarantinedCustodyGate, 3),
        route_index_observation(),
        unregister(Timing::BeforeCall),
        unregister(Timing::NativeRetryable),
        unregister(Timing::AfterSuccessKnown),
        success(),
    ]
}

fn registration_base(phase: Phase) -> Case {
    let mut case = base(
        Path::RegistrationShutdown,
        TopologyKind::RegistrationOnly,
        phase,
        None,
    );
    case.node_precondition = NodePrecondition::NotApplicable;
    case.target.scope = TargetScope::Registration;
    case.target.route_ordinal = 0;
    case.target.runtime_generation = 0;
    case.target.shm_connection_id = 0;
    case.target.role = None;
    case.sqlite_outcome = SqliteOutcome::NotApplicable;
    case
}

fn gate(phase: Phase, variant: u8) -> Case {
    let mut case = failure(
        registration_base(phase),
        Timing::Validation,
        FailureClass::RegistrationRetained,
    );
    case.variant = variant;
    case.registration_phase = RegistrationPhase::RetainedRegistered;
    case.logical_route_phase = LogicalRoutePhase::Retained;
    case.retained.callback_leases = u8::from(phase == Phase::OutstandingCallbackGate);
    case.counts.custody_retain = 1;
    case.later_callback_allowed = true;
    case
}

fn route_index_observation() -> Case {
    let mut case = gate(Phase::RouteIndexObservation, 4);
    case.timing = Timing::NativeUncertain;
    native_observed(case)
}

fn unregister(timing: Timing) -> Case {
    let mut case = failure(
        registration_base(Phase::VfsUnregister),
        timing,
        FailureClass::RegistrationRetained,
    );
    if timing == Timing::NativeRetryable {
        case = native_observed(case);
    }
    case.pre = EMPTY;
    case.post = EMPTY;
    clear_route_custody(&mut case);
    case.registry_route_phase = RegistryRoutePhase::Removed;
    case.logical_route_phase = LogicalRoutePhase::Removed;
    case.counts.vfs_unregister_attempt = u8::from(timing != Timing::BeforeCall);
    case.counts.vfs_unregister_success = u8::from(timing == Timing::AfterSuccessKnown);
    case.mutation_may_have_occurred = timing == Timing::AfterSuccessKnown;
    case.counts.custody_retain = 1;
    case.registration_phase = if timing == Timing::AfterSuccessKnown {
        RegistrationPhase::RetainedAfterUnregister
    } else {
        RegistrationPhase::RetainedRegistered
    };
    case.later_callback_allowed = false;
    case
}

fn success() -> Case {
    let mut case = registration_base(Phase::Success);
    case.pre = EMPTY;
    case.post = EMPTY;
    case.registration_phase = RegistrationPhase::Unregistered;
    case.registry_route_phase = RegistryRoutePhase::Removed;
    case.logical_route_phase = LogicalRoutePhase::Removed;
    clear_route_custody(&mut case);
    case.retained.vfs_table = false;
    case.retained.vfs_name = false;
    case.retained.vfs_context = false;
    case.retained.root_deletable = true;
    case.later_callback_allowed = false;
    case.counts.vfs_unregister_attempt = 1;
    case.counts.vfs_unregister_success = 1;
    case
}

fn clear_route_custody(case: &mut Case) {
    case.retained.node = false;
    case.retained.views = 0;
    case.retained.mappings = 0;
    case.retained.dms = DmsCustody::Absent;
    case.retained.shm_file = false;
    case.retained.main_file = false;
    case.retained.main_lock_owner = false;
    case.retained.main_lease = false;
    case.retained.shm_lease = false;
    case.retained.callback_leases = 0;
    case.retained.registry_entry = false;
    case.retained.logical_names = 0;
}
