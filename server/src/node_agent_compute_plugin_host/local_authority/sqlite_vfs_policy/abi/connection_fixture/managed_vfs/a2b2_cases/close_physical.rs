use super::model::{
    base, failure, native_observed, route_terminal, terminal, CallbackKind, Case, DmsCustody,
    FailureClass, Path, Phase, RegistryRoutePhase, Timing, TopologyKind, UnmapMode,
};

const SHM_PLATFORM_PHASES: [Phase; 3] = [
    Phase::ViewUnmap,
    Phase::MappingClose,
    Phase::DmsSharedRelease,
];

pub(super) fn cases() -> Vec<Case> {
    let mut cases = vec![
        raw_take_rejected(),
        begin_close_rejected(),
        callback_admission_rejected(),
        wrapper_before(),
    ];
    for phase in SHM_PLATFORM_PHASES {
        for timing in [
            Timing::BeforeCall,
            Timing::NativeUncertain,
            Timing::AfterSuccessKnown,
            Timing::AfterSuccessUncertain,
        ] {
            cases.push(shm_lift(phase, timing));
        }
    }
    for timing in [
        Timing::BeforeCall,
        Timing::NativeRetryable,
        Timing::NativeUncertain,
        Timing::AfterSuccessKnown,
        Timing::AfterSuccessUncertain,
    ] {
        cases.push(shm_lift(Phase::ShmFileClose, timing));
    }
    cases.push(shm_lift(Phase::ConnectionDetach, Timing::BeforeCall));
    cases.push(shm_lift(Phase::ConnectionDetach, Timing::AfterSuccessKnown));
    cases.push(shm_lift(
        Phase::ConnectionDetach,
        Timing::AfterSuccessUncertain,
    ));
    for (timing, variant) in [
        (Timing::BeforeCall, 0),
        (Timing::NativeUncertain, 0),
        (Timing::NativeUncertain, 1),
        (Timing::AfterSuccessKnown, 0),
    ] {
        let mut case = main_failure(Phase::MainLockRelease, timing);
        case.variant = variant;
        cases.push(case);
    }
    for timing in platform_timings() {
        cases.push(main_failure(Phase::MainFileClose, timing));
    }
    cases.push(physical_success());
    cases
}

fn platform_timings() -> [Timing; 4] {
    [
        Timing::BeforeCall,
        Timing::NativeRetryable,
        Timing::NativeUncertain,
        Timing::AfterSuccessKnown,
    ]
}

fn close_base(phase: Phase) -> Case {
    let mut case = base(
        Path::JointClose,
        TopologyKind::FinalConnection,
        phase,
        Some(CallbackKind::Close),
    );
    case.unmap_mode = UnmapMode::Keep;
    case.counts.raw_state_take_attempt = 1;
    case.counts.raw_state_take_success = 1;
    case.counts.methods_clear = 1;
    case
}

fn raw_take_rejected() -> Case {
    let mut case = failure(
        close_base(Phase::RawStateTake),
        Timing::Validation,
        FailureClass::ProtocolViolation,
    );
    case.counts.raw_state_take_success = 0;
    case.counts.methods_clear = 0;
    case
}

fn begin_close_rejected() -> Case {
    let mut case = failure(
        close_base(Phase::BeginConnectionClose),
        Timing::BeforeCall,
        FailureClass::RegistryRejected,
    );
    case.counts.fault_observe = 0;
    case.counts.fault_trigger = 0;
    case.counts.custody_retain = 1;
    route_terminal(case, FailureClass::RegistryRejected, false)
}

fn wrapper_before() -> Case {
    let mut case = failure(
        close_base(Phase::MainFileClose),
        Timing::BeforeCall,
        FailureClass::IoBeforeMutation,
    );
    case.variant = 1;
    case.counts.custody_retain = 1;
    route_terminal(case, FailureClass::IoBeforeMutation, false)
}

fn callback_admission_rejected() -> Case {
    let mut case = failure(
        close_base(Phase::CallbackAdmission),
        Timing::BeforeCall,
        FailureClass::RegistryRejected,
    );
    case.counts.fault_observe = 0;
    case.counts.fault_trigger = 0;
    route_terminal(case, FailureClass::RegistryRejected, false)
}

fn shm_lift(cause: Phase, timing: Timing) -> Case {
    let selected_success = matches!(
        timing,
        Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
    );
    let uncertain = matches!(
        timing,
        Timing::NativeUncertain | Timing::AfterSuccessUncertain
    );
    let prior_mutation = !matches!(cause, Phase::ViewUnmap);
    let mutation = prior_mutation || selected_success || uncertain;
    let class = if uncertain || (cause == Phase::ShmFileClose && timing == Timing::NativeRetryable)
    {
        FailureClass::OutcomeUncertainPoisoned
    } else if mutation {
        FailureClass::MutatedButKnown
    } else {
        FailureClass::IoBeforeMutation
    };
    let mut case = failure(close_base(Phase::ShmUnmapLift), timing, class);
    case.cause_phase = Some(cause);
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 0;
    case.counts.selected_action_attempt = u8::from(timing != Timing::BeforeCall);
    case.counts.selected_action_success = u8::from(selected_success);
    case.lock_outcome_uncertain = cause == Phase::DmsSharedRelease
        && matches!(
            timing,
            Timing::NativeUncertain | Timing::AfterSuccessUncertain
        );
    match cause {
        Phase::ViewUnmap if selected_success => case.retained.views = 0,
        Phase::MappingClose => {
            case.retained.views = 0;
            if selected_success {
                case.retained.mappings = 0;
            }
        }
        Phase::DmsSharedRelease => {
            case.retained.views = 0;
            case.retained.mappings = 0;
            if timing == Timing::NativeUncertain {
                case.retained.dms = DmsCustody::OutcomeUncertain;
            } else if selected_success {
                case.retained.dms = DmsCustody::Released;
            }
        }
        Phase::ShmFileClose => {
            case.retained.views = 0;
            case.retained.mappings = 0;
            case.retained.dms = DmsCustody::Released;
            if timing != Timing::BeforeCall {
                case.retained.node = false;
                case.retained.dms = DmsCustody::Absent;
            }
            if selected_success {
                case.retained.shm_file = false;
            }
        }
        Phase::ConnectionDetach => {
            case.retained.node = false;
            case.retained.views = 0;
            case.retained.mappings = 0;
            case.retained.dms = DmsCustody::Absent;
            case.retained.shm_file = false;
            if selected_success {
                case.post.shm_connections = 0;
                case.counts.shm_detach = 1;
            }
        }
        _ => {}
    }
    if mutation || uncertain {
        terminal(case, class, mutation)
    } else {
        route_terminal(case, class, false)
    }
}

fn main_failure(phase: Phase, timing: Timing) -> Case {
    let selected_success = matches!(
        timing,
        Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
    );
    let uncertain = matches!(
        timing,
        Timing::NativeUncertain | Timing::AfterSuccessUncertain
    );
    let class = if uncertain {
        FailureClass::OutcomeUncertainPoisoned
    } else {
        FailureClass::MutatedButKnown
    };
    let mut case = failure(close_base(phase), timing, class);
    if matches!(timing, Timing::NativeRetryable | Timing::NativeUncertain) {
        case = native_observed(case);
    }
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 0;
    case.counts.shm_detach = 1;
    case.post.shm_connections = 0;
    case.lock_outcome_uncertain = phase == Phase::MainLockRelease && uncertain;
    case.retained.node = false;
    case.retained.views = 0;
    case.retained.mappings = 0;
    case.retained.dms = DmsCustody::Absent;
    case.retained.shm_file = false;
    case.counts.main_unlock_attempt =
        u8::from(phase == Phase::MainFileClose || timing != Timing::BeforeCall);
    case.counts.main_unlock_success = u8::from(phase == Phase::MainFileClose || selected_success);
    case.counts.main_file_close_attempt =
        u8::from(phase == Phase::MainFileClose && timing != Timing::BeforeCall);
    case.counts.main_file_close_success =
        u8::from(phase == Phase::MainFileClose && selected_success);
    if phase == Phase::MainFileClose && selected_success {
        case.retained.main_file = false;
        case.retained.main_lock_owner = false;
    }
    route_terminal(case, class, true)
}

fn physical_success() -> Case {
    let mut case = close_base(Phase::Success);
    case.post.shm_connections = 0;
    case.retained.node = false;
    case.retained.views = 0;
    case.retained.mappings = 0;
    case.retained.dms = DmsCustody::Absent;
    case.retained.shm_file = false;
    case.retained.main_file = false;
    case.retained.main_lock_owner = false;
    case.counts.callback_begin = 1;
    case.retained.callback_leases = 1;
    case.counts.shm_detach = 1;
    case.counts.main_unlock_attempt = 1;
    case.counts.main_unlock_success = 1;
    case.counts.main_file_close_attempt = 1;
    case.counts.main_file_close_success = 1;
    case.registry_route_phase = RegistryRoutePhase::Closing;
    case
}
