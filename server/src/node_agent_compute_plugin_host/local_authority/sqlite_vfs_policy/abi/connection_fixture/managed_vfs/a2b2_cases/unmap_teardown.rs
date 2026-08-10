use super::model::{
    base, failure, route_terminal, terminal, CallbackKind, Case, DmsCustody, FailureClass,
    NodePrecondition, Path, Phase, Timing, TopologyKind, UnmapMode,
};

const PLATFORM_PHASES: [Phase; 3] = [
    Phase::ViewUnmap,
    Phase::MappingClose,
    Phase::DmsSharedRelease,
];

pub(super) fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for phase in PLATFORM_PHASES {
        cases.extend(platform_phase(phase));
    }
    cases.extend(file_close_phase());
    cases.push(detach_before());
    cases.push(detach_after());
    cases.push(detach_after_uncertain());
    cases.push(callback_completion());
    cases.push(success(true));
    cases.push(success(false));
    cases
}

fn platform_phase(phase: Phase) -> [Case; 4] {
    [
        phase_failure(phase, Timing::BeforeCall),
        phase_failure(phase, Timing::NativeUncertain),
        phase_failure(phase, Timing::AfterSuccessKnown),
        phase_failure(phase, Timing::AfterSuccessUncertain),
    ]
}

fn file_close_phase() -> [Case; 5] {
    [
        phase_failure(Phase::ShmFileClose, Timing::BeforeCall),
        phase_failure(Phase::ShmFileClose, Timing::NativeRetryable),
        phase_failure(Phase::ShmFileClose, Timing::NativeUncertain),
        phase_failure(Phase::ShmFileClose, Timing::AfterSuccessKnown),
        phase_failure(Phase::ShmFileClose, Timing::AfterSuccessUncertain),
    ]
}

fn phase_failure(phase: Phase, timing: Timing) -> Case {
    let prior_mutation = !matches!(phase, Phase::ViewUnmap);
    let selected_success = matches!(
        timing,
        Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
    );
    let uncertain = matches!(
        timing,
        Timing::NativeUncertain | Timing::AfterSuccessUncertain
    );
    let mutation = prior_mutation || selected_success || uncertain;
    let class = if uncertain || timing == Timing::NativeRetryable {
        FailureClass::OutcomeUncertainPoisoned
    } else if mutation {
        FailureClass::MutatedButKnown
    } else {
        FailureClass::IoBeforeMutation
    };
    let mut case = failure(
        base(
            Path::Unmap,
            TopologyKind::FinalConnection,
            phase,
            Some(CallbackKind::Shm),
        ),
        timing,
        class,
    );
    case.counts.callback_begin = 1;
    case.unmap_mode = UnmapMode::Keep;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 1;
    case.counts.selected_action_attempt = u8::from(!matches!(timing, Timing::BeforeCall));
    case.counts.selected_action_success = u8::from(selected_success);
    match phase {
        Phase::ViewUnmap if selected_success => case.retained.views = 0,
        Phase::MappingClose => {
            // View teardown precedes every mapping-close seam.
            case.retained.views = 0;
            if selected_success {
                case.retained.mappings = 0;
            }
        }
        Phase::DmsSharedRelease => {
            // All views and mappings are gone before the DMS release is selected.
            case.retained.views = 0;
            case.retained.mappings = 0;
            if selected_success {
                case.retained.dms = DmsCustody::Released;
            }
            if timing == Timing::NativeUncertain {
                case.retained.dms = DmsCustody::OutcomeUncertain;
            }
            case.lock_outcome_uncertain = matches!(
                timing,
                Timing::NativeUncertain | Timing::AfterSuccessUncertain
            );
        }
        Phase::ShmFileClose => {
            case.retained.views = 0;
            case.retained.mappings = 0;
            case.retained.dms = DmsCustody::Released;
            if timing != Timing::BeforeCall {
                // The node is taken before the real close attempt. Native failure retains only
                // quarantined file custody; after-success retains the close receipt.
                case.retained.node = false;
                case.retained.dms = DmsCustody::Absent;
            }
            if selected_success {
                case.retained.shm_file = false;
            }
        }
        _ => {}
    }
    if mutation || uncertain {
        // The unsafe SHM failure quarantines/removes the exact route before the outer callback
        // can complete, so completion is attempted but rejected and its lease remains retained.
        case.counts.callback_complete_success = 0;
        terminal(case, class, mutation)
    } else {
        case
    }
}

pub(super) fn validate_dms_receipt(case: &Case) -> Result<(), &'static str> {
    let direct = case.path == Path::Unmap && case.phase == Phase::DmsSharedRelease;
    let joint = case.path == Path::JointClose
        && case.phase == Phase::ShmUnmapLift
        && case.cause_phase == Some(Phase::DmsSharedRelease);
    if !(direct || joint) {
        return Ok(());
    }
    let native_uncertain = case.timing == Timing::NativeUncertain;
    let lock_uncertain = matches!(
        case.timing,
        Timing::NativeUncertain | Timing::AfterSuccessUncertain
    );
    let expected = if native_uncertain {
        DmsCustody::OutcomeUncertain
    } else if case.timing == Timing::BeforeCall {
        DmsCustody::Shared
    } else {
        DmsCustody::Released
    };
    if case.retained.dms != expected || case.lock_outcome_uncertain != lock_uncertain {
        return Err("DMS receipt custody disagrees with native versus after-success timing");
    }
    Ok(())
}

fn detach_before() -> Case {
    let mut case = failure(
        base(
            Path::Unmap,
            TopologyKind::FinalConnection,
            Phase::ConnectionDetach,
            Some(CallbackKind::Shm),
        ),
        Timing::BeforeCall,
        FailureClass::MutatedButKnown,
    );
    case.counts.callback_begin = 1;
    case.unmap_mode = UnmapMode::Keep;
    case.retained.node = false;
    case.retained.views = 0;
    case.retained.mappings = 0;
    case.retained.dms = DmsCustody::Absent;
    case.retained.shm_file = false;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 0;
    terminal(case, FailureClass::MutatedButKnown, true)
}

fn detach_after() -> Case {
    let mut case = detach_before();
    case.timing = Timing::AfterSuccessKnown;
    case.post.shm_connections = 0;
    case.counts.selected_action_attempt = 1;
    case.counts.selected_action_success = 1;
    case.counts.shm_detach = 1;
    terminal(case, FailureClass::MutatedButKnown, true)
}

fn detach_after_uncertain() -> Case {
    let mut case = detach_after();
    case.timing = Timing::AfterSuccessUncertain;
    terminal(case, FailureClass::OutcomeUncertainPoisoned, true)
}

fn callback_completion() -> Case {
    let mut case = detach_after();
    case.phase = Phase::CallbackCompletion;
    case.timing = Timing::NativeUncertain;
    case.counts.fault_observe = 0;
    case.counts.fault_trigger = 0;
    case.counts.callback_complete_success = 0;
    route_terminal(case, FailureClass::RegistryRejected, true)
}

fn success(with_node: bool) -> Case {
    let mut case = base(
        Path::Unmap,
        TopologyKind::FinalConnection,
        Phase::Success,
        Some(CallbackKind::Shm),
    );
    case.post.shm_connections = 0;
    case.unmap_mode = UnmapMode::Keep;
    case.node_precondition = if with_node {
        NodePrecondition::Live
    } else {
        NodePrecondition::Absent
    };
    case.retained.node = false;
    case.retained.views = 0;
    case.retained.mappings = 0;
    case.retained.dms = DmsCustody::Absent;
    case.retained.shm_file = false;
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 1;
    case.counts.shm_detach = 1;
    if with_node {
        case.counts.selected_action_attempt = 4;
        case.counts.selected_action_success = 4;
    }
    case
}
