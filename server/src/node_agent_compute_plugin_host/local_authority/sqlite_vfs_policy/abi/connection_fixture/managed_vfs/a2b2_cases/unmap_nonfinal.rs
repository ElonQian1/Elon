use super::model::{
    base, failure, route_terminal, terminal, CallbackKind, Case, FailureClass, Path, Phase, Timing,
    TopologyKind, UnmapMode, ONE, TWO,
};

pub(super) fn cases() -> Vec<Case> {
    vec![
        validation(),
        callback_admission(),
        callback_wrapper_before(),
        held_lock(false),
        held_lock(true),
        detach_before(),
        detach_after(),
        detach_after_uncertain(),
        completion_after_detach(),
        success(false),
        success(true),
    ]
}

fn validation() -> Case {
    let mut case = failure(
        base(
            Path::Unmap,
            TopologyKind::SharedNonFinal,
            Phase::RequestValidation,
            Some(CallbackKind::Shm),
        ),
        Timing::Validation,
        FailureClass::ProtocolViolation,
    );
    case.unmap_mode = UnmapMode::Delete;
    case
}

fn callback_admission() -> Case {
    let mut case = failure(
        base(
            Path::Unmap,
            TopologyKind::SharedNonFinal,
            Phase::CallbackAdmission,
            Some(CallbackKind::Shm),
        ),
        Timing::BeforeCall,
        FailureClass::RegistryRejected,
    );
    case.unmap_mode = UnmapMode::Keep;
    case.counts.fault_observe = 0;
    case.counts.fault_trigger = 0;
    route_terminal(case, FailureClass::RegistryRejected, false)
}

fn callback_wrapper_before() -> Case {
    let mut case = failure(
        base(
            Path::Unmap,
            TopologyKind::SharedNonFinal,
            Phase::ConnectionDetach,
            Some(CallbackKind::Shm),
        ),
        Timing::BeforeCall,
        FailureClass::IoBeforeMutation,
    );
    case.unmap_mode = UnmapMode::Keep;
    case.variant = 1;
    case
}

fn held_lock(exclusive: bool) -> Case {
    let mut case = failure(
        base(
            Path::Unmap,
            TopologyKind::SharedNonFinal,
            Phase::HeldLockGate,
            Some(CallbackKind::Shm),
        ),
        Timing::Validation,
        FailureClass::ProtocolViolation,
    );
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 1;
    case.unmap_mode = UnmapMode::Keep;
    if exclusive {
        case.pre_exclusive_mask = 1;
    } else {
        case.pre_shared_mask = 1;
    }
    case
}

fn detach_before() -> Case {
    let mut case = callback_wrapper_before();
    case.variant = 0;
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 1;
    case
}

fn detach_after() -> Case {
    let mut case = detach_before();
    case.timing = Timing::AfterSuccessKnown;
    case.post.shm_connections = 1;
    case.counts.selected_action_attempt = 1;
    case.counts.selected_action_success = 1;
    case.counts.shm_detach = 1;
    case.counts.callback_complete_success = 0;
    terminal(case, FailureClass::MutatedButKnown, true)
}

fn detach_after_uncertain() -> Case {
    let mut case = detach_after();
    case.timing = Timing::AfterSuccessUncertain;
    terminal(case, FailureClass::OutcomeUncertainPoisoned, true)
}

fn completion_after_detach() -> Case {
    let mut case = detach_after();
    case.phase = Phase::CallbackCompletion;
    case.timing = Timing::NativeUncertain;
    case.counts.fault_observe = 0;
    case.counts.fault_trigger = 0;
    case.counts.callback_complete_success = 0;
    route_terminal(case, FailureClass::RegistryRejected, true)
}

fn success(delete_requested: bool) -> Case {
    let mut case = base(
        Path::Unmap,
        TopologyKind::SharedNonFinal,
        Phase::Success,
        Some(CallbackKind::Shm),
    );
    case.pre = TWO;
    case.post = TWO;
    case.post.shm_connections = ONE.shm_connections;
    case.retained.shm_lease = true;
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 1;
    case.counts.selected_action_attempt = 1;
    case.counts.selected_action_success = 1;
    case.counts.shm_detach = 1;
    case.unmap_mode = if delete_requested {
        UnmapMode::Delete
    } else {
        UnmapMode::Keep
    };
    case
}
