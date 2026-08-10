use super::model::{
    base, failure, native_observed, route_terminal, terminal, CallbackKind, Case, Counts,
    FailureClass, Path, Phase, Timing, TopologyKind,
};

pub(super) fn cases() -> Vec<Case> {
    vec![
        admission_rejected(),
        wrapper_before(),
        fence_before(),
        fence_after(),
        completion_rejected(Timing::BeforeCall),
        completion_rejected(Timing::NativeUncertain),
        completion_rejected(Timing::AfterSuccessKnown),
        success(),
    ]
}

fn wrapper_before() -> Case {
    let mut case = failure(
        base(
            Path::Barrier,
            TopologyKind::SharedNonFinal,
            Phase::BarrierFence,
            Some(CallbackKind::Shm),
        ),
        Timing::BeforeCall,
        FailureClass::IoBeforeMutation,
    );
    case.variant = 1;
    case.counts = Counts {
        raw_state_abandon: 1,
        methods_clear: 1,
        custody_retain: 1,
        ..case.counts
    };
    route_terminal(case, FailureClass::IoBeforeMutation, false)
}

fn admission_rejected() -> Case {
    let mut case = failure(
        base(
            Path::Barrier,
            TopologyKind::SharedNonFinal,
            Phase::CallbackAdmission,
            Some(CallbackKind::Shm),
        ),
        Timing::BeforeCall,
        FailureClass::RegistryRejected,
    );
    case.counts = Counts {
        raw_state_abandon: 1,
        methods_clear: 1,
        fault_observe: 0,
        fault_trigger: 0,
        custody_retain: 1,
        ..case.counts
    };
    // Registry admission/raw-state abandonment terminalizes only this exact route. The SHM
    // coordinator never admitted the callback, so the shared FileId domain remains live.
    route_terminal(case, FailureClass::RegistryRejected, false)
}

fn fence_before() -> Case {
    let mut case = failure(
        base(
            Path::Barrier,
            TopologyKind::SharedNonFinal,
            Phase::BarrierFence,
            Some(CallbackKind::Shm),
        ),
        Timing::BeforeCall,
        FailureClass::IoBeforeMutation,
    );
    case.counts = Counts {
        raw_state_abandon: 1,
        methods_clear: 1,
        callback_begin: 1,
        custody_retain: 1,
        ..case.counts
    };
    terminal(case, FailureClass::IoBeforeMutation, false)
}

fn fence_after() -> Case {
    let mut case = fence_before();
    case.timing = Timing::AfterSuccessUncertain;
    case.class = FailureClass::OutcomeUncertainPoisoned;
    case.counts.selected_action_attempt = 1;
    case.counts.selected_action_success = 1;
    terminal(case, FailureClass::OutcomeUncertainPoisoned, false)
}

fn completion_rejected(timing: Timing) -> Case {
    let mut case = failure(
        base(
            Path::Barrier,
            TopologyKind::SharedNonFinal,
            Phase::CallbackCompletion,
            Some(CallbackKind::Shm),
        ),
        timing,
        FailureClass::RegistryRejected,
    );
    if timing == Timing::NativeUncertain {
        case = native_observed(case);
    }
    case.counts = Counts {
        raw_state_abandon: 1,
        methods_clear: 1,
        callback_begin: 1,
        callback_complete_attempt: u8::from(timing != Timing::BeforeCall),
        callback_complete_success: u8::from(timing == Timing::AfterSuccessKnown),
        selected_action_attempt: 1,
        selected_action_success: 1,
        custody_retain: 1,
        ..case.counts
    };
    // Callback completion rejection quarantines the route/receipt after the low-level barrier;
    // it does not poison the otherwise successful SHM coordinator domain.
    route_terminal(case, FailureClass::RegistryRejected, false)
}

fn success() -> Case {
    let mut case = base(
        Path::Barrier,
        TopologyKind::SharedNonFinal,
        Phase::Success,
        Some(CallbackKind::Shm),
    );
    case.counts = Counts {
        callback_begin: 1,
        callback_complete_attempt: 1,
        callback_complete_success: 1,
        selected_action_attempt: 1,
        selected_action_success: 1,
        ..case.counts
    };
    case
}
