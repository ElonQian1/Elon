use super::*;

fn native_step(route: ManagedTestRouteOrdinal) -> ManagedTestLifecycleFaultStep {
    ManagedTestLifecycleFaultStep::route(
        route,
        ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion,
        1,
        ManagedTestLifecycleFaultTiming::NativeFailure,
    )
    .expect("exact barrier native-failure step")
}

#[test]
fn exact_barrier_native_failure_gate_is_linear_and_observed_only_after_rejection() {
    let route = ManagedTestRouteOrdinal::test_value(1);
    let controller = ManagedTestLifecycleFaultController::new();
    controller.install(&[native_step(route)]).expect("install");
    let binding = controller.binding(route);

    assert!(!binding
        .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("record before"));
    assert!(binding
        .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("claim exact native gate"));
    assert_eq!(controller.pending_count().expect("pending count"), 0);
    assert_eq!(controller.observations().expect("observations").len(), 1);

    binding.native_failure(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion);
    assert_eq!(
        controller.observations().expect("observations"),
        vec![
            ManagedTestLifecycleFaultObservation {
                route: Some(route),
                phase: ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion,
                occurrence: 1,
                timing: ManagedTestLifecycleFaultTiming::BeforeCall,
                triggered: false,
            },
            ManagedTestLifecycleFaultObservation {
                route: Some(route),
                phase: ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion,
                occurrence: 1,
                timing: ManagedTestLifecycleFaultTiming::NativeFailure,
                triggered: false,
            },
        ]
    );
    assert!(!controller.is_terminal());
}

#[test]
fn late_install_uses_key_relative_occurrence_without_erasing_baseline() {
    let route = ManagedTestRouteOrdinal::test_value(1);
    let controller = ManagedTestLifecycleFaultController::new();
    let binding = controller.binding(route);
    assert!(!binding
        .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("record fixture baseline"));
    let baseline = controller.observations().expect("baseline observations");
    controller
        .install(&[native_step(route)])
        .expect("late install");

    assert!(!binding
        .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("record first relative occurrence"));
    assert!(binding
        .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("claim first relative occurrence"));
    binding.native_failure(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion);

    let observations = controller.observations().expect("observations");
    assert_eq!(
        observations
            .strip_prefix(baseline.as_slice())
            .map(|suffix| suffix.len()),
        Some(2)
    );
    assert_eq!(observations[1].occurrence, 1);
    assert_eq!(
        observations[1].timing,
        ManagedTestLifecycleFaultTiming::BeforeCall
    );
    assert_eq!(observations[2].occurrence, 1);
    assert_eq!(
        observations[2].timing,
        ManagedTestLifecycleFaultTiming::NativeFailure
    );
    assert_eq!(controller.pending_count().expect("pending count"), 0);
    assert!(!controller.is_terminal());
}

#[test]
fn unfaulted_barrier_window_is_key_relative_and_preserves_baseline() {
    let route = ManagedTestRouteOrdinal::test_value(1);
    let controller = ManagedTestLifecycleFaultController::new();
    let binding = controller.binding(route);
    assert!(!binding
        .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("record fixture baseline"));
    assert!(!binding
        .after_success(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("complete fixture baseline"));
    let baseline = controller.observations().expect("baseline observations");

    controller
        .begin_unfaulted_barrier_observation_window(route)
        .expect("begin exact Barrier observation window");
    assert!(!binding
        .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("record relative before"));
    assert!(!binding
        .after_success(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("record relative after"));

    let observations = controller.observations().expect("observations");
    assert_eq!(observations[..baseline.len()], baseline);
    assert_eq!(observations[baseline.len()].occurrence, 1);
    assert_eq!(observations[baseline.len() + 1].occurrence, 1);
    assert_eq!(controller.pending_count().expect("pending count"), 0);
}

#[test]
fn barrier_native_failure_gate_rejects_out_of_order_claim() {
    let route = ManagedTestRouteOrdinal::test_value(1);
    let controller = ManagedTestLifecycleFaultController::new();
    controller.install(&[native_step(route)]).expect("install");

    assert!(controller
        .binding(route)
        .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .is_err());
    assert!(controller.is_terminal());
    assert_eq!(controller.pending_count().expect("pending count"), 1);
}

#[test]
fn barrier_native_failure_gate_ignores_other_route() {
    let route = ManagedTestRouteOrdinal::test_value(1);
    let sibling = ManagedTestRouteOrdinal::test_value(2);
    let controller = ManagedTestLifecycleFaultController::new();
    controller.install(&[native_step(route)]).expect("install");
    let sibling_binding = controller.binding(sibling);
    assert!(!sibling_binding
        .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("record sibling before"));

    assert!(!sibling_binding
        .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("ignore another route's native gate"));
    assert!(!controller.is_terminal());
    assert_eq!(controller.pending_count().expect("pending count"), 1);

    let exact_binding = controller.binding(route);
    assert!(!exact_binding
        .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("record exact route before"));
    assert!(exact_binding
        .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("claim exact route native gate"));
    assert!(!controller.is_terminal());
    assert_eq!(controller.pending_count().expect("pending count"), 0);
}

#[test]
fn barrier_native_failure_gate_rejects_double_claim() {
    let route = ManagedTestRouteOrdinal::test_value(1);
    let controller = ManagedTestLifecycleFaultController::new();
    controller.install(&[native_step(route)]).expect("install");
    let binding = controller.binding(route);
    assert!(!binding
        .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("record before"));
    assert!(binding
        .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .expect("first claim"));

    assert!(binding
        .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
        .is_err());
    assert!(controller.is_terminal());
    assert_eq!(controller.pending_count().expect("pending count"), 0);
}
