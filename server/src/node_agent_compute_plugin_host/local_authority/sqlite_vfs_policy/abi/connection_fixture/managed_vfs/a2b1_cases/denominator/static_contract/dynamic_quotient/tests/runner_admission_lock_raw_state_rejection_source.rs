//! Source-only admission contracts for all eleven q11 Lock raw-state rejections.

use super::super::runner_admission::{
    compile_for_test, raw_state_rejection_catalog_row_count_for_test,
    validate_lock_program_for_test,
};
use super::lock_raw_state_rejection_cases::{
    frozen_lock_raw_state_rejection_leaves_v1, lock_raw_state_rejection_descriptor_v1,
    FrozenLockRawStateRejectionCaseV1, LOCK_RAW_STATE_REJECTION_MEMBER_COUNT,
};
use super::super::super::terminal_descriptor::RawStateV1;
use super::*;

fn supported_key_and_member(
    case: FrozenLockRawStateRejectionCaseV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = &frozen_lock_raw_state_rejection_leaves_v1()[&case];
    let descriptor = lock_raw_state_rejection_descriptor_v1(
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
        "q11 raw-state admission accepted {mutation}"
    );
}

#[test]
fn all_eleven_exact_q11_descriptors_and_catalog_seals_are_source_present() {
    let leaves = frozen_lock_raw_state_rejection_leaves_v1();
    assert_eq!(leaves.len(), LOCK_RAW_STATE_REJECTION_MEMBER_COUNT);
    assert_eq!(
        raw_state_rejection_catalog_row_count_for_test(),
        LOCK_RAW_STATE_REJECTION_MEMBER_COUNT
    );
    for &case in leaves.keys() {
        let (key, member) = supported_key_and_member(case);
        validate_lock_program_for_test(&key, member, compile_for_test(&key))
            .unwrap_or_else(|error| panic!("exact q11 member {case:?} was rejected: {error:?}"));
    }
}

#[test]
fn all_eleven_q11_programs_are_inventory_present_without_granting_supported() {
    for (&case, leaf) in frozen_lock_raw_state_rejection_leaves_v1() {
        let prepared = prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            super::super::runner_admission::ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            project_dynamic_class_v1(
                &leaf.record,
                &lock_raw_state_rejection_descriptor_v1(case, RunnerCapabilityV1::Supported),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
        );
    }
}

#[test]
fn every_q11_key_rejects_a_sibling_frozen_seal() {
    let leaves = frozen_lock_raw_state_rejection_leaves_v1();
    let members = leaves.values().map(|leaf| leaf.member).collect::<Vec<_>>();
    for (index, &case) in leaves.keys().enumerate() {
        let (key, member) = supported_key_and_member(case);
        let sibling = members[(index + 1) % members.len()];
        assert_ne!(member, sibling);
        assert_rejected(key, sibling, "a sibling frozen member seal");
    }
}

#[test]
fn q11_matcher_rejects_typed_identity_recipe_and_every_lock_axis_drift() {
    let case = FrozenLockRawStateRejectionCaseV1::NullFileDirect;
    let (key, member) = supported_key_and_member(case);

    let mut schema = key;
    schema.schema_version = schema.schema_version.wrapping_add(1);
    assert_rejected(schema, member, "schema drift");
    let mut root = key;
    root.root = RootOperationV1::Map;
    assert_rejected(root, member, "root drift");
    let mut source = key;
    source.source_site = SourceSiteV1::AdapterDispatch;
    assert_rejected(source, member, "source-site drift");
    let mut stimulus = key;
    stimulus.stimulus = StimulusV1::LockRaw(RawStateV1::Uninstalled);
    assert_rejected(stimulus, member, "raw-state drift");
    let mut sentinel = key;
    sentinel.stimulus = StimulusV1::LockRaw(RawStateV1::DropCompleted);
    assert_rejected(sentinel, member, "sentinel raw-state substitution");
    let mut prestate = key;
    prestate.prestate = PrestateV1::Lock(LockPrestateV1::NoHeldLocks);
    assert_rejected(prestate, member, "prestate drift");
    let mut operation = key;
    operation.operation = DynamicOperationV1::Lock(LockOperationV1::AdapterDispatch);
    assert_rejected(operation, member, "operation drift");
    let mut phase = key;
    phase.phase = PhaseV1::Adapter;
    assert_rejected(phase, member, "phase drift");
    let mut timing = key;
    timing.timing = TimingV1::BeforeCall;
    assert_rejected(timing, member, "timing drift");
    let mut occurrence = key;
    occurrence.occurrence = OccurrenceV1::Exact(1);
    assert_rejected(occurrence, member, "occurrence drift");

    let mut fixture = key;
    fixture.recipe.fixture = FixtureV1::ManagedWalMainSingleConnection;
    assert_rejected(fixture, member, "fixture drift");
    let mut callback = key;
    callback.recipe.callback = CallbackV1::XShmMap;
    assert_rejected(callback, member, "callback drift");
    let mut seam = key;
    seam.recipe.fault_seam = FaultSeamV1::Natural;
    assert_rejected(seam, member, "fault-seam drift");
    let mut observer = key;
    observer.recipe.observer = ObserverV1::LockCallbackAndSnapshot;
    assert_rejected(observer, member, "observer drift");
    let mut cleanup = key;
    cleanup.recipe.cleanup = CleanupV1::RetainUnsafeCustodyThenParentCleanup;
    assert_rejected(cleanup, member, "cleanup drift");
    let mut capability = key;
    capability.recipe.capability =
        RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated);
    assert_rejected(capability, member, "capability drift");

    let axis_mutations: [(&str, fn(&mut LockAxesV1)); 10] = [
        ("action", |axes| axes.action = ReachabilityV1::Reached(LockActionV1::LockShared)),
        ("first", |axes| axes.first = ReachabilityV1::Reached(0)),
        ("count", |axes| axes.count = ReachabilityV1::Reached(1)),
        ("mask", |axes| axes.mask = ReachabilityV1::Reached(1)),
        ("initialization", |axes| axes.initialization = ReachabilityV1::Reached(InitializationProfileV1::NodeLive)),
        ("held-shared", |axes| axes.held_shared_mask = ReachabilityV1::Reached(0)),
        ("held-exclusive", |axes| axes.held_exclusive_mask = ReachabilityV1::Reached(0)),
        ("sibling-shared", |axes| axes.sibling_shared_mask = ReachabilityV1::Reached(0)),
        ("sibling-exclusive", |axes| axes.sibling_exclusive_mask = ReachabilityV1::Reached(0)),
        ("completion", |axes| axes.completion = ReachabilityV1::Reached(LockCompletionV1::Completed)),
    ];
    for (name, mutate) in axis_mutations {
        let mut candidate = key;
        let DynamicAxesV1::Lock(axes) = &mut candidate.axes else {
            unreachable!()
        };
        mutate(axes);
        assert_rejected(candidate, member, &format!("{name} axis drift"));
    }
}

#[test]
fn q11_matcher_rejects_every_expected_field_drift() {
    let (key, member) = supported_key_and_member(FrozenLockRawStateRejectionCaseV1::NullFileDirect);
    let mut candidates = Vec::new();

    let mut value = key;
    value.expected.sqlite = SqliteResultV1::Busy;
    candidates.push(("sqlite", value));
    let mut value = key;
    value.expected.disposition = TerminalDispositionV1::Abandoned;
    candidates.push(("disposition", value));
    let mut value = key;
    value.expected.phase = PhaseV1::Adapter;
    candidates.push(("phase", value));
    let mut value = key;
    value.expected.failure = FailureClassV1::RegistryRejected;
    candidates.push(("failure", value));
    let mut value = key;
    value.expected.mutation = MutationStateV1::Known;
    candidates.push(("mutation", value));
    let mut value = key;
    value.expected.lock_outcome_uncertain = true;
    candidates.push(("lock-outcome-uncertain", value));
    let mut value = key;
    value.expected.lock_effect = LockEffectV1::Unchanged;
    candidates.push(("lock-effect", value));
    let mut value = key;
    value.expected.dms_lock = DmsLockCustodyV1::ExistingShared;
    candidates.push(("dms-lock", value));
    let mut value = key;
    value.expected.raw_slots = CustodyStateV1::Cleared;
    candidates.push(("raw-slots", value));
    let mut value = key;
    value.expected.route = CustodyStateV1::Unchanged;
    candidates.push(("route", value));
    let mut value = key;
    value.expected.callback = CustodyStateV1::Released;
    candidates.push(("callback", value));
    let mut value = key;
    value.expected.file = CustodyStateV1::Cleared;
    candidates.push(("file", value));
    let mut value = key;
    value.expected.mapping = CustodyStateV1::Unchanged;
    candidates.push(("mapping", value));
    let mut value = key;
    value.expected.view = CustodyStateV1::Unchanged;
    candidates.push(("view", value));
    let mut value = key;
    value.expected.payload = CustodyStateV1::Retained;
    candidates.push(("payload", value));
    let mut value = key;
    value.expected.counts.callback_begin = 1;
    candidates.push(("counts", value));

    for (name, candidate) in candidates {
        assert_rejected(candidate, member, &format!("expected {name} drift"));
    }
}

#[test]
fn q11_direct_cleanup_and_adapter_profiles_are_not_interchangeable() {
    for case in [
        FrozenLockRawStateRejectionCaseV1::UninstalledDirect,
        FrozenLockRawStateRejectionCaseV1::MethodsNullStatePresentDirect,
        FrozenLockRawStateRejectionCaseV1::OtherTypePayloadPresentDropCompleted,
        FrozenLockRawStateRejectionCaseV1::OtherTypePayloadPresentDropUnwindCaught,
        FrozenLockRawStateRejectionCaseV1::HandleBoundFileMissingDirect,
    ] {
        let (mut key, member) = supported_key_and_member(case);
        key.expected.raw_slots = CustodyStateV1::NotReached;
        key.expected.payload = CustodyStateV1::NotReached;
        key.expected.file = CustodyStateV1::NotReached;
        key.expected.disposition = TerminalDispositionV1::Returned;
        assert_rejected(key, member, "a generic raw-state Expected vector");
    }
}
