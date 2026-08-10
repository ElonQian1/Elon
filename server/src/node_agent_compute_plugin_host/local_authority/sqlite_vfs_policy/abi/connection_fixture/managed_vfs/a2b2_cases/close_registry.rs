use super::model::{
    base, failure, native_observed, route_terminal, CallbackKind, Case, DmsCustody, FailureClass,
    LogicalRoutePhase, Path, Phase, RegistryRoutePhase, Timing, TopologyKind, UnmapMode, EMPTY,
    ONE,
};

pub(super) fn cases() -> Vec<Case> {
    vec![
        registry_close(Timing::BeforeCall),
        registry_close(Timing::NativeUncertain),
        registry_close(Timing::AfterSuccessKnown),
        callback_completion(Timing::BeforeCall),
        callback_completion(Timing::NativeUncertain),
        callback_completion(Timing::AfterSuccessKnown),
        connection_observation(Timing::BeforeCall, false),
        connection_observation(Timing::Validation, true),
        connection_observation(Timing::AfterSuccessKnown, false),
        registry_route_remove(Timing::BeforeCall, 0),
        registry_route_remove(Timing::NativeUncertain, 1),
        registry_route_remove(Timing::NativeUncertain, 2),
        registry_route_remove(Timing::AfterSuccessKnown, 0),
        logical_route_remove(Timing::BeforeCall, 0),
        logical_route_remove(Timing::NativeUncertain, 1),
        logical_route_remove(Timing::NativeUncertain, 2),
        logical_route_remove(Timing::AfterSuccessKnown, 0),
        success(TopologyKind::SharedNonFinal),
        success(TopologyKind::FinalConnection),
    ]
}

fn lifecycle_base(path: Path, phase: Phase) -> Case {
    let mut case = base(
        path,
        TopologyKind::FinalConnection,
        phase,
        Some(CallbackKind::Close),
    );
    case.unmap_mode = UnmapMode::Keep;
    case.counts.raw_state_take_attempt = 1;
    case.counts.raw_state_take_success = 1;
    case.counts.methods_clear = 1;
    case.counts.callback_begin = 1;
    case.counts.shm_detach = 1;
    case.counts.main_unlock_attempt = 1;
    case.counts.main_unlock_success = 1;
    case.counts.main_file_close_attempt = 1;
    case.counts.main_file_close_success = 1;
    case.post.shm_connections = 0;
    case.retained.node = false;
    case.retained.views = 0;
    case.retained.mappings = 0;
    case.retained.dms = DmsCustody::Absent;
    case.retained.shm_file = false;
    case.retained.main_file = false;
    case.retained.main_lock_owner = false;
    case
}

fn registry_close(timing: Timing) -> Case {
    let mut case = failure(
        lifecycle_base(Path::JointClose, Phase::RegistryWalMainClose),
        timing,
        FailureClass::RegistryRejected,
    );
    if timing == Timing::NativeUncertain {
        case = native_observed(case);
    }
    case.counts.registry_close_attempt = u8::from(timing != Timing::BeforeCall);
    case.counts.registry_close_success = u8::from(timing == Timing::AfterSuccessKnown);
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = u8::from(timing != Timing::NativeUncertain);
    if timing == Timing::AfterSuccessKnown {
        case.retained.main_lease = false;
        case.retained.shm_lease = false;
    }
    route_terminal(case, FailureClass::RegistryRejected, true)
}

fn callback_completion(timing: Timing) -> Case {
    let mut case = failure(
        lifecycle_base(Path::RegistryLifecycle, Phase::CallbackCompletion),
        timing,
        FailureClass::RegistryRejected,
    );
    if timing == Timing::NativeUncertain {
        case = native_observed(case);
    }
    case.counts.registry_close_attempt = 1;
    case.counts.registry_close_success = 1;
    case.counts.callback_complete_attempt = u8::from(timing != Timing::BeforeCall);
    case.counts.callback_complete_success = u8::from(timing == Timing::AfterSuccessKnown);
    case.retained.main_lease = false;
    case.retained.shm_lease = false;
    route_terminal(case, FailureClass::RegistryRejected, true)
}

fn connection_observation(timing: Timing, outstanding_sidecar: bool) -> Case {
    let mut case = failure(
        lifecycle_base(Path::RegistryLifecycle, Phase::ConnectionObservation),
        timing,
        FailureClass::RegistryRejected,
    );
    if outstanding_sidecar {
        case = native_observed(case);
    }
    case.variant = u8::from(outstanding_sidecar);
    case.counts.registry_close_attempt = 1;
    case.counts.registry_close_success = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 1;
    case.counts.connection_observe_attempt =
        u8::from(outstanding_sidecar || timing == Timing::AfterSuccessKnown);
    case.counts.connection_observe_success = u8::from(timing == Timing::AfterSuccessKnown);
    case.retained.main_lease = false;
    case.retained.shm_lease = false;
    if timing == Timing::AfterSuccessKnown {
        case.registry_route_phase = RegistryRoutePhase::AwaitingRetirement;
    }
    route_terminal(case, FailureClass::RegistryRejected, true)
}

fn registry_route_remove(timing: Timing, variant: u8) -> Case {
    let mut case = connection_observation(Timing::AfterSuccessKnown, false);
    case.phase = Phase::RegistryRouteRemoval;
    case.timing = timing;
    case.variant = variant;
    case.counts.fault_observe = u8::from(timing != Timing::NativeUncertain || variant == 1);
    case.counts.fault_trigger = u8::from(timing != Timing::NativeUncertain);
    case.counts.registry_route_remove_attempt = u8::from(timing != Timing::BeforeCall);
    case.counts.registry_route_remove_success =
        u8::from(timing == Timing::AfterSuccessKnown || variant == 2);
    if case.counts.registry_route_remove_success == 1 {
        case.post.registry_routes = 0;
        case.retained.registry_entry = false;
        case.registry_route_phase = RegistryRoutePhase::Removed;
    }
    route_terminal(case, FailureClass::RegistryRejected, true)
}

fn logical_route_remove(timing: Timing, variant: u8) -> Case {
    let mut case = registry_route_remove(Timing::AfterSuccessKnown, 0);
    case.phase = Phase::LogicalRouteRemoval;
    case.timing = timing;
    case.variant = variant;
    case.post.sqlite_connections = 0;
    case.sqlite_outcome = super::model::SqliteOutcome::NotApplicable;
    case.counts.fault_observe = u8::from(timing != Timing::NativeUncertain || variant == 2);
    case.counts.fault_trigger = u8::from(timing != Timing::NativeUncertain);
    case.counts.logical_names_remove_attempt =
        u8::from(timing == Timing::AfterSuccessKnown || variant == 2);
    case.counts.logical_names_remove_success = u8::from(timing == Timing::AfterSuccessKnown);
    case.counts.logical_names_remove = if timing == Timing::AfterSuccessKnown {
        3
    } else {
        0
    };
    if timing == Timing::AfterSuccessKnown {
        case.post.logical_names = 0;
        case.retained.logical_names = 0;
        case.logical_route_phase = LogicalRoutePhase::Removed;
    }
    route_terminal(case, FailureClass::RegistryRejected, true)
}

fn success(topology: TopologyKind) -> Case {
    let mut case = lifecycle_base(Path::RegistryLifecycle, Phase::Success);
    case.topology_kind = topology;
    case.counts.registry_close_attempt = 1;
    case.counts.registry_close_success = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 1;
    case.counts.connection_observe_attempt = 1;
    case.counts.connection_observe_success = 1;
    case.counts.registry_route_remove_attempt = 1;
    case.counts.registry_route_remove_success = 1;
    case.counts.logical_names_remove_attempt = 1;
    case.counts.logical_names_remove_success = 1;
    case.counts.logical_names_remove = 3;
    case.retained.main_lease = false;
    case.retained.shm_lease = false;
    case.retained.registry_entry = false;
    case.retained.logical_names = 0;
    case.registry_route_phase = RegistryRoutePhase::Removed;
    case.logical_route_phase = LogicalRoutePhase::Removed;
    case.later_callback_allowed = false;
    if topology == TopologyKind::SharedNonFinal {
        case.pre = super::model::TWO;
        case.post = ONE;
    } else {
        case.pre = ONE;
        case.post = EMPTY;
    }
    case
}
