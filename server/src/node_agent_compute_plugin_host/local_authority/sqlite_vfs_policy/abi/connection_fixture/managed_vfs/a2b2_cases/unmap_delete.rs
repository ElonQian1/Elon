use super::model::{
    base, failure, observed_but_pending, route_terminal, terminal, CallbackKind, Case, DmsCustody,
    FailureClass, Path, Phase, Timing, TopologyKind, UnmapMode,
};

pub(super) fn cases() -> Vec<Case> {
    let mut cases = vec![
        authorization(1, FailureClass::ProtocolViolation, false),
        authorization(2, FailureClass::ProtocolViolation, false),
        authorization(3, FailureClass::ProtocolViolation, false),
        authorization(4, FailureClass::OutcomeUncertainPoisoned, true),
    ];
    for timing in [
        Timing::BeforeCall,
        Timing::NativeRetryable,
        Timing::NativeUncertain,
        Timing::AfterSuccessKnown,
        Timing::AfterSuccessUncertain,
    ] {
        cases.push(delete_failure(timing));
    }
    cases.push(detach_after_delete(Timing::BeforeCall));
    cases.push(detach_after_delete(Timing::AfterSuccessKnown));
    cases.push(detach_after_delete(Timing::AfterSuccessUncertain));
    cases.push(completion_after_delete());
    cases.push(success(false));
    cases.push(success(true));
    cases
}

fn authorization(variant: u8, class: FailureClass, lock_uncertain: bool) -> Case {
    let mut case = failure(
        base(
            Path::Unmap,
            TopologyKind::FinalConnection,
            Phase::DeleteAuthorization,
            Some(CallbackKind::Shm),
        ),
        Timing::Validation,
        class,
    );
    case.unmap_mode = UnmapMode::Delete;
    case.variant = variant;
    case.lock_outcome_uncertain = lock_uncertain;
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 1;
    if lock_uncertain {
        case.counts.callback_complete_success = 0;
        terminal(case, class, false)
    } else {
        case
    }
}

fn delete_failure(timing: Timing) -> Case {
    let uncertain = matches!(
        timing,
        Timing::NativeUncertain | Timing::AfterSuccessUncertain
    );
    let class = if uncertain || timing == Timing::NativeRetryable {
        FailureClass::OutcomeUncertainPoisoned
    } else {
        FailureClass::MutatedButKnown
    };
    let mut case = failure(
        base(
            Path::Unmap,
            TopologyKind::FinalConnection,
            Phase::ExactSiblingDelete,
            Some(CallbackKind::Shm),
        ),
        timing,
        class,
    );
    case.unmap_mode = UnmapMode::Delete;
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 0;
    case.counts.selected_action_attempt = u8::from(timing != Timing::BeforeCall);
    case.counts.selected_action_success = u8::from(matches!(
        timing,
        Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
    ));
    case.retained.node = false;
    case.retained.views = 0;
    case.retained.mappings = 0;
    case.retained.dms = DmsCustody::Absent;
    case.retained.shm_file = false;
    terminal(case, class, true)
}

fn detach_after_delete(timing: Timing) -> Case {
    let uncertain = timing == Timing::AfterSuccessUncertain;
    let class = if uncertain {
        FailureClass::OutcomeUncertainPoisoned
    } else {
        FailureClass::MutatedButKnown
    };
    let mut case = failure(
        base(
            Path::Unmap,
            TopologyKind::FinalConnection,
            Phase::ConnectionDetach,
            Some(CallbackKind::Shm),
        ),
        timing,
        class,
    );
    case.unmap_mode = UnmapMode::Delete;
    case.variant = 1;
    case.retained.node = false;
    case.retained.views = 0;
    case.retained.mappings = 0;
    case.retained.dms = DmsCustody::Absent;
    case.retained.shm_file = false;
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 0;
    if matches!(
        timing,
        Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
    ) {
        case.post.shm_connections = 0;
        case.counts.selected_action_attempt = 1;
        case.counts.selected_action_success = 1;
        case.counts.shm_detach = 1;
    }
    terminal(case, class, true)
}

fn completion_after_delete() -> Case {
    let mut case = detach_after_delete(Timing::AfterSuccessKnown);
    case.phase = Phase::CallbackCompletion;
    case.timing = Timing::NativeUncertain;
    case.counts.fault_observe = 0;
    case.counts.fault_trigger = 0;
    case.counts.callback_complete_success = 0;
    route_terminal(case, FailureClass::RegistryRejected, true)
}

fn success(not_found: bool) -> Case {
    let mut case = base(
        Path::Unmap,
        TopologyKind::FinalConnection,
        Phase::Success,
        Some(CallbackKind::Shm),
    );
    case.unmap_mode = UnmapMode::Delete;
    case.variant = u8::from(not_found);
    case.post.shm_connections = 0;
    case.retained.node = false;
    case.retained.views = 0;
    case.retained.mappings = 0;
    case.retained.dms = DmsCustody::Absent;
    case.retained.shm_file = false;
    case.counts.callback_begin = 1;
    case.counts.callback_complete_attempt = 1;
    case.counts.callback_complete_success = 1;
    case.counts.selected_action_attempt = 5;
    case.counts.selected_action_success = 5;
    case.counts.shm_detach = 1;
    if not_found {
        // The after-success selector was observed, but NotFound is not a delete success. Its
        // one-shot token therefore remains untriggered and pending.
        observed_but_pending(case)
    } else {
        case
    }
}
