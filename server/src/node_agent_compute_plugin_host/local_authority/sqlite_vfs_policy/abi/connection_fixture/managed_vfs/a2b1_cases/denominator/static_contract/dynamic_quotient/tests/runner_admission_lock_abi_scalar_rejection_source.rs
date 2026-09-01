//! Source-only admission contracts for all seven q10 Lock ABI scalar rejections.

use super::super::runner_admission::{
    abi_scalar_rejection_catalog_row_count_for_test, compile_for_test,
    validate_lock_program_for_test,
};
use super::lock_abi_scalar_rejection_cases::{
    frozen_lock_abi_scalar_rejection_leaves_v1, lock_abi_scalar_rejection_descriptor_v1,
    LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT,
};
use super::*;

fn supported_key_and_member(scalar: LockAbiScalarV1) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = &frozen_lock_abi_scalar_rejection_leaves_v1()[&scalar];
    let descriptor = lock_abi_scalar_rejection_descriptor_v1(
        scalar,
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
        "q10 ABI scalar admission accepted {mutation}"
    );
}

#[test]
fn all_seven_exact_q10_descriptors_and_catalog_seals_are_source_present() {
    let leaves = frozen_lock_abi_scalar_rejection_leaves_v1();
    assert_eq!(leaves.len(), LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT);
    assert_eq!(
        abi_scalar_rejection_catalog_row_count_for_test(),
        LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT
    );
    for &scalar in leaves.keys() {
        let (key, member) = supported_key_and_member(scalar);
        validate_lock_program_for_test(&key, member, compile_for_test(&key))
            .unwrap_or_else(|error| panic!("exact q10 member {scalar:?} was rejected: {error:?}"));
    }
}

#[test]
fn all_seven_q10_programs_are_inventory_present_without_granting_supported() {
    for (&scalar, leaf) in frozen_lock_abi_scalar_rejection_leaves_v1() {
        let prepared = prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            super::super::runner_admission::ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            project_dynamic_class_v1(
                &leaf.record,
                &lock_abi_scalar_rejection_descriptor_v1(scalar, RunnerCapabilityV1::Supported,),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
        );
    }
}

#[test]
fn every_q10_key_rejects_a_sibling_frozen_seal() {
    let leaves = frozen_lock_abi_scalar_rejection_leaves_v1();
    let members = leaves.values().map(|leaf| leaf.member).collect::<Vec<_>>();
    for (index, &scalar) in leaves.keys().enumerate() {
        let (key, member) = supported_key_and_member(scalar);
        let sibling = members[(index + 1) % members.len()];
        assert_ne!(member, sibling);
        assert_rejected(key, sibling, "a sibling frozen member seal");
    }
}

#[test]
fn q10_matcher_rejects_typed_identity_recipe_and_every_lock_axis_drift() {
    let scalar = LockAbiScalarV1 {
        offset: ValidityV1::Valid,
        count: ValidityV1::Valid,
        flags: ValidityV1::Invalid,
    };
    let (key, member) = supported_key_and_member(scalar);

    let mut schema = key;
    schema.schema_version = schema.schema_version.wrapping_add(1);
    assert_rejected(schema, member, "schema drift");
    let mut root = key;
    root.root = RootOperationV1::Map;
    assert_rejected(root, member, "root drift");
    let mut source = key;
    source.source_site = SourceSiteV1::RawStateAdmission;
    assert_rejected(source, member, "source-site drift");
    let mut stimulus = key;
    stimulus.stimulus = StimulusV1::LockAbi(LockAbiScalarV1 {
        flags: ValidityV1::Valid,
        ..scalar
    });
    assert_rejected(stimulus, member, "the all-valid scalar profile");
    let mut prestate = key;
    prestate.prestate = PrestateV1::Lock(LockPrestateV1::NoHeldLocks);
    assert_rejected(prestate, member, "prestate drift");
    let mut operation = key;
    operation.operation = DynamicOperationV1::Lock(LockOperationV1::RawAdmission);
    assert_rejected(operation, member, "operation drift");
    let mut phase = key;
    phase.phase = PhaseV1::RawAdmission;
    assert_rejected(phase, member, "phase drift");
    let mut timing = key;
    timing.timing = TimingV1::Natural;
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
    observer.recipe.observer = ObserverV1::CustodyAndCleanup;
    assert_rejected(observer, member, "observer drift");
    let mut cleanup = key;
    cleanup.recipe.cleanup = CleanupV1::RetainUnsafeCustodyThenParentCleanup;
    assert_rejected(cleanup, member, "cleanup drift");
    let mut capability = key;
    capability.recipe.capability =
        RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated);
    assert_rejected(capability, member, "capability drift");

    let axis_mutations: [(&str, fn(&mut LockAxesV1)); 10] = [
        ("action", |axes| {
            axes.action = ReachabilityV1::Reached(LockActionV1::LockShared)
        }),
        ("first", |axes| axes.first = ReachabilityV1::Reached(0)),
        ("count", |axes| axes.count = ReachabilityV1::Reached(1)),
        ("mask", |axes| axes.mask = ReachabilityV1::Reached(1)),
        ("initialization", |axes| {
            axes.initialization = ReachabilityV1::Reached(InitializationProfileV1::NodeLive)
        }),
        ("held-shared", |axes| {
            axes.held_shared_mask = ReachabilityV1::Reached(0)
        }),
        ("held-exclusive", |axes| {
            axes.held_exclusive_mask = ReachabilityV1::Reached(0)
        }),
        ("sibling-shared", |axes| {
            axes.sibling_shared_mask = ReachabilityV1::Reached(0)
        }),
        ("sibling-exclusive", |axes| {
            axes.sibling_exclusive_mask = ReachabilityV1::Reached(0)
        }),
        ("completion", |axes| {
            axes.completion = ReachabilityV1::Reached(LockCompletionV1::Completed)
        }),
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
fn q10_matcher_rejects_every_expected_field_drift() {
    let scalar = LockAbiScalarV1 {
        offset: ValidityV1::Invalid,
        count: ValidityV1::Invalid,
        flags: ValidityV1::Invalid,
    };
    let (key, member) = supported_key_and_member(scalar);
    let mut candidates = Vec::new();

    let mut value = key;
    value.expected.sqlite = SqliteResultV1::Busy;
    candidates.push(("sqlite", value));
    let mut value = key;
    value.expected.disposition = TerminalDispositionV1::Quarantined;
    candidates.push(("disposition", value));
    let mut value = key;
    value.expected.phase = PhaseV1::RawAdmission;
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
    value.expected.raw_slots = CustodyStateV1::Unchanged;
    candidates.push(("raw-slots", value));
    let mut value = key;
    value.expected.route = CustodyStateV1::Unchanged;
    candidates.push(("route", value));
    let mut value = key;
    value.expected.callback = CustodyStateV1::Released;
    candidates.push(("callback", value));
    let mut value = key;
    value.expected.file = CustodyStateV1::Unchanged;
    candidates.push(("file", value));
    let mut value = key;
    value.expected.mapping = CustodyStateV1::Unchanged;
    candidates.push(("mapping", value));
    let mut value = key;
    value.expected.view = CustodyStateV1::Unchanged;
    candidates.push(("view", value));
    let mut value = key;
    value.expected.payload = CustodyStateV1::Unchanged;
    candidates.push(("payload", value));
    let mut value = key;
    value.expected.counts.callback_begin = 1;
    candidates.push(("counts", value));

    for (name, candidate) in candidates {
        assert_rejected(candidate, member, &format!("expected {name} drift"));
    }
}
