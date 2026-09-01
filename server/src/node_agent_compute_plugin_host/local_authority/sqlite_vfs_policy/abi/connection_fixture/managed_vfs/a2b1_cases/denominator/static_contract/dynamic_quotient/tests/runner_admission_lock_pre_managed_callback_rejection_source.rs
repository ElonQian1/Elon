//! Source-only admission contracts for all 528 q9 pre-managed rejection programs.

use super::super::runner_admission::{
    compile_for_test, pre_managed_callback_rejection_catalog_row_count_for_test,
    validate_lock_program_for_test,
};
use super::lock_pre_managed_callback_rejection_cases::{
    frozen_lock_pre_managed_callback_rejection_leaves_v1,
    lock_pre_managed_callback_rejection_descriptor_v1, LockPreManagedCallbackRejectionFamilyV1,
    LockPreManagedCallbackRejectionKeyV1, LOCK_PRE_MANAGED_CALLBACK_REJECTION_MEMBER_COUNT,
};
use super::*;

fn supported_key_and_member(
    case: LockPreManagedCallbackRejectionKeyV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = &frozen_lock_pre_managed_callback_rejection_leaves_v1()[&case];
    let descriptor = lock_pre_managed_callback_rejection_descriptor_v1(
        case,
        RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
    );
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

fn assert_rejected(key: DynamicClassKeyV1, member: StaticMemberSealV1, mutation: &str) {
    assert!(
        validate_lock_program_for_test(&key, member, compile_for_test(&key)).is_err(),
        "q9 admission accepted {mutation}"
    );
}

#[test]
fn all_528_exact_q9_descriptors_and_catalog_seals_are_source_present() {
    let leaves = frozen_lock_pre_managed_callback_rejection_leaves_v1();
    assert_eq!(
        leaves.len(),
        LOCK_PRE_MANAGED_CALLBACK_REJECTION_MEMBER_COUNT
    );
    assert_eq!(
        pre_managed_callback_rejection_catalog_row_count_for_test(),
        LOCK_PRE_MANAGED_CALLBACK_REJECTION_MEMBER_COUNT
    );
    for &case in leaves.keys() {
        let (key, member) = supported_key_and_member(case);
        validate_lock_program_for_test(&key, member, compile_for_test(&key))
            .unwrap_or_else(|error| panic!("exact q9 member {case:?} was rejected: {error:?}"));
    }
}

#[test]
fn all_528_q9_programs_are_inventory_present_without_granting_supported() {
    for (&case, leaf) in frozen_lock_pre_managed_callback_rejection_leaves_v1() {
        let prepared = prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            super::super::runner_admission::ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            project_dynamic_class_v1(
                &leaf.record,
                &lock_pre_managed_callback_rejection_descriptor_v1(
                    case,
                    RunnerCapabilityV1::Supported,
                ),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
        );
    }
}

#[test]
fn every_q9_key_rejects_a_sibling_frozen_seal() {
    let leaves = frozen_lock_pre_managed_callback_rejection_leaves_v1();
    let members = leaves.values().map(|leaf| leaf.member).collect::<Vec<_>>();
    for (index, &case) in leaves.keys().enumerate() {
        let (key, member) = supported_key_and_member(case);
        let sibling = members[(index + 1) % members.len()];
        assert_ne!(member, sibling);
        assert_rejected(key, sibling, "a sibling frozen member seal");
    }
}

#[test]
fn q9_matcher_rejects_typed_identity_recipe_axes_and_expected_drift() {
    let case = frozen_lock_pre_managed_callback_rejection_leaves_v1()
        .keys()
        .copied()
        .find(|case| {
            case.family == LockPreManagedCallbackRejectionFamilyV1::UnsupportedFileRoleCompleted
                && case.action == LockActionV1::LockShared
        })
        .expect("q9 adapter representative");
    let (key, member) = supported_key_and_member(case);

    let mut source = key;
    source.source_site = SourceSiteV1::RegistryCallbackAdmission;
    assert_rejected(source, member, "source-site drift");

    let mut stimulus = key;
    stimulus.stimulus = StimulusV1::LockManaged(LockManagedStimulusV1::AdmissionCounterOverflow);
    assert_rejected(stimulus, member, "stimulus drift");

    let mut prestate = key;
    prestate.prestate = PrestateV1::Lock(LockPrestateV1::NoHeldLocks);
    assert_rejected(prestate, member, "prestate drift");

    let mut operation = key;
    operation.operation = DynamicOperationV1::Lock(LockOperationV1::LocalAcquire);
    assert_rejected(operation, member, "operation drift");

    let mut phase = key;
    phase.phase = PhaseV1::RequestValidation;
    assert_rejected(phase, member, "phase drift");

    let mut timing = key;
    timing.timing = TimingV1::Natural;
    assert_rejected(timing, member, "timing drift");

    let mut occurrence = key;
    occurrence.occurrence = OccurrenceV1::Exact(1);
    assert_rejected(occurrence, member, "occurrence drift");

    let mut recipe = key;
    recipe.recipe.fixture = FixtureV1::ManagedWalMainTwoConnections;
    assert_rejected(recipe, member, "fixture drift");
    let mut recipe = key;
    recipe.recipe.callback = CallbackV1::XShmMap;
    assert_rejected(recipe, member, "callback drift");
    let mut recipe = key;
    recipe.recipe.fault_seam = FaultSeamV1::Natural;
    assert_rejected(recipe, member, "fault-seam drift");
    let mut recipe = key;
    recipe.recipe.observer = ObserverV1::CustodyAndCleanup;
    assert_rejected(recipe, member, "observer drift");
    let mut recipe = key;
    recipe.recipe.cleanup = CleanupV1::RetainUnsafeCustodyThenParentCleanup;
    assert_rejected(recipe, member, "cleanup drift");

    let mut axes = key;
    let DynamicAxesV1::Lock(value) = &mut axes.axes else {
        unreachable!()
    };
    value.initialization = ReachabilityV1::Reached(InitializationProfileV1::NodeLive);
    assert_rejected(axes, member, "initialization-axis drift");
    let mut axes = key;
    let DynamicAxesV1::Lock(value) = &mut axes.axes else {
        unreachable!()
    };
    value.held_shared_mask = ReachabilityV1::Reached(0);
    assert_rejected(axes, member, "held-mask reachability drift");
    let mut axes = key;
    let DynamicAxesV1::Lock(value) = &mut axes.axes else {
        unreachable!()
    };
    value.completion = ReachabilityV1::Reached(LockCompletionV1::Direct);
    assert_rejected(axes, member, "family/completion drift");

    let mut expected = key;
    expected.expected.lock_effect = LockEffectV1::Unchanged;
    assert_rejected(expected, member, "expected lock-effect drift");
    let mut expected = key;
    expected.expected.route = CustodyStateV1::Quarantined;
    assert_rejected(expected, member, "expected route drift");
    let mut expected = key;
    expected.expected.counts.callback_complete = 0;
    assert_rejected(expected, member, "expected count-vector drift");
}

#[test]
fn q9_direct_families_keep_their_distinct_exact_expected_vectors() {
    for (family, wrong_disposition) in [
        (
            LockPreManagedCallbackRejectionFamilyV1::AdmissionRouteUnknownDirect,
            TerminalDispositionV1::Quarantined,
        ),
        (
            LockPreManagedCallbackRejectionFamilyV1::AdmissionCounterOverflowDirect,
            TerminalDispositionV1::Returned,
        ),
    ] {
        let case = frozen_lock_pre_managed_callback_rejection_leaves_v1()
            .keys()
            .copied()
            .find(|case| case.family == family)
            .expect("q9 direct representative");
        let (mut key, member) = supported_key_and_member(case);
        key.expected.disposition = wrong_disposition;
        assert_rejected(key, member, "direct-family disposition substitution");
    }
}
