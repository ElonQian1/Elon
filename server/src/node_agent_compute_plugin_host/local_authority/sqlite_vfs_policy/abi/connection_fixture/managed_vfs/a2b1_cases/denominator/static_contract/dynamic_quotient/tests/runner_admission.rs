use super::super::manifest_canonical::digest_dynamic_manifest_body_v1;
use super::super::runner_admission::{
    compile_for_test, resolve_v1, resolve_with_plan_for_test, RunnerAdmissionViolationV1,
};
use super::*;

fn map_key_and_member() -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let record = record("runner-admission-map", "runner-admission-map-branch");
    let descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let validated = project_validated_dynamic_terminal_v1(&record, &descriptor).unwrap();
    (validated.semantic_key, validated.descriptor_binding.member)
}

fn lock_key_and_member() -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let record = lock_request_record();
    let descriptor = lock_request_rejection_descriptor(
        LockManagedStimulusV1::RangeOverflow,
        CallbackV1::XShmLock,
    );
    let validated = project_validated_dynamic_terminal_v1(&record, &descriptor).unwrap();
    (validated.semantic_key, validated.descriptor_binding.member)
}

#[test]
fn naked_supported_claim_is_not_a_runner_permit() {
    let (mut key, member) = map_key_and_member();
    key.recipe.capability = RunnerCapabilityV1::Supported;
    assert_eq!(
        resolve_v1(&key, member),
        Err(RunnerAdmissionViolationV1::UnsealedSupportedClaim)
    );
}

#[test]
fn root_plans_preserve_the_exact_map_and_lock_gaps() {
    let (map_key, map_member) = map_key_and_member();
    let map_receipt = resolve_v1(&map_key, map_member).unwrap();
    assert_eq!(map_receipt.member(), map_member);
    assert_eq!(
        map_receipt.exact_missing_gap(),
        Some(CapabilityGapV1::QuotientRunnerNotIntegrated)
    );
    assert_ne!(map_receipt.plan_sha256(), Digest32::ZERO);

    let mut wrong_map_gap = map_key;
    wrong_map_gap.recipe.capability =
        RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete);
    assert_eq!(
        resolve_v1(&wrong_map_gap, map_member),
        Err(RunnerAdmissionViolationV1::DeclaredGapMismatch {
            expected: CapabilityGapV1::QuotientRunnerNotIntegrated,
            actual: CapabilityGapV1::LockObservationIncomplete,
        })
    );

    let (lock_key, lock_member) = lock_key_and_member();
    let lock_receipt = resolve_v1(&lock_key, lock_member).unwrap();
    assert_eq!(lock_receipt.member(), lock_member);
    assert_eq!(
        lock_receipt.exact_missing_gap(),
        Some(CapabilityGapV1::LockObservationIncomplete)
    );
    assert_ne!(lock_receipt.plan_sha256(), Digest32::ZERO);
}

#[test]
fn plan_normalizes_only_capability_readiness_and_binds_other_semantics() {
    let (missing_key, _) = map_key_and_member();
    let missing_plan = compile_for_test(&missing_key);

    let mut supported_key = missing_key;
    supported_key.recipe.capability = RunnerCapabilityV1::Supported;
    assert_eq!(compile_for_test(&supported_key), missing_plan);

    let mut changed_semantics = missing_key;
    changed_semantics.expected.lock_outcome_uncertain =
        !changed_semantics.expected.lock_outcome_uncertain;
    assert_ne!(compile_for_test(&changed_semantics), missing_plan);
}

#[test]
fn cross_root_and_same_root_plan_swaps_are_rejected() {
    let (map_key, map_member) = map_key_and_member();
    let map_plan = compile_for_test(&map_key);
    let (lock_key, lock_member) = lock_key_and_member();
    assert_eq!(
        resolve_with_plan_for_test(&lock_key, lock_member, map_plan),
        Err(RunnerAdmissionViolationV1::PlanBindingMismatch)
    );

    let mut changed_map_key = map_key;
    changed_map_key.expected.lock_outcome_uncertain =
        !changed_map_key.expected.lock_outcome_uncertain;
    assert_eq!(
        resolve_with_plan_for_test(&changed_map_key, map_member, map_plan),
        Err(RunnerAdmissionViolationV1::PlanBindingMismatch)
    );
}

#[test]
fn catalog_blocker_and_future_manifest_both_commit_runner_admission() {
    let blocked = record(
        "runner-admission-blocked",
        "runner-admission-blocked-branch",
    );
    let blocked_descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let mut blocked_builder = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    blocked_builder
        .observe(StreamedLeafV1::Terminal {
            record: &blocked,
            descriptor: &blocked_descriptor,
            seal: &seal(&blocked),
        })
        .unwrap();
    let blocker_digest = match blocked_builder.finish() {
        Err(CatalogErrorV1::RunnerCapabilityMissing {
            runner_admission_binding_sha256,
            ..
        }) => runner_admission_binding_sha256,
        other => panic!("unexpected catalog result: {other:?}"),
    };
    assert_ne!(blocker_digest, Digest32::ZERO);

    let first = record("runner-admission-first", "runner-admission-first-branch");
    let second = record("runner-admission-second", "runner-admission-second-branch");
    let mut builder = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    observe_synthetic_for_test(&mut builder, &first, &blocked_descriptor);
    observe_synthetic_for_test(&mut builder, &second, &blocked_descriptor);
    let catalog = builder.finish().unwrap();
    let binding = FrozenStaticBindingV1 {
        context: ManifestContextV1 {
            schema: "static-v1".to_owned(),
            root: RootOperationV1::Map,
            target_scope: "windows-x64".to_owned(),
            source_baseline_commit_sha1: "a".repeat(40),
            source_scope_sha256: Digest32([1; 32]),
            ledger_sha256: Digest32([2; 32]),
            map_profile_set_sha256: Some(Digest32([3; 32])),
            map_ordinal_domain_sha256: Some(Digest32([4; 32])),
            lock_range_set_sha256: None,
            lock_range_count: None,
        },
        included_count: 2,
        excluded_count: 0,
        source_universe_count: 2,
        static_manifest_sha256: Digest32([5; 32]),
        included_member_pair_set_sha256: catalog.member_pair_set_sha256(),
    };
    let bundle = build_dynamic_manifest_v1(&binding, &catalog).unwrap();
    assert_eq!(
        bundle.manifest.context.runner_admission_binding_sha256,
        catalog.runner_admission_binding_sha256()
    );
    assert_ne!(
        bundle.manifest.context.runner_admission_binding_sha256,
        Digest32::ZERO
    );
    let baseline = digest_dynamic_manifest_body_v1(&bundle.manifest);
    let mut tampered = bundle.manifest;
    tampered.context.runner_admission_binding_sha256 = Digest32::ZERO;
    assert_ne!(digest_dynamic_manifest_body_v1(&tampered), baseline);
}
