use super::super::{
    source_leaf_authority::{
        CaseKeyV1, CoordinateV1, CustodyStateV1, Digest32, DmsLockCustodyV1, ExpectedV1,
        FailureClassV1, FrozenStaticBindingV1, LeafIdentityV1, LeafOutcomeV1, LeafRecordV1,
        LeafSealOutcomeV1, LeafSealV1, LockEffectV1, LockModeV1, ManifestContextV1,
        MutationStateV1, ObservableCountsV1, RootOperationV1, SourceWitnessV1, SqliteResultV1,
        StreamedLeafV1, TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CapabilityGapV1, CleanupV1, ExecutionRecipeV1, FaultSeamV1, FixtureV1,
        InitializationProfileV1, LockActionV1, LockAxesV1, LockCompletionV1, LockManagedStimulusV1,
        LockOperationV1, LockPrestateV1, MapAbiScalarV1, MapAxesV1, MapCompletionV1,
        MapManagedStimulusV1, MapModeV1, MapOperationV1, MapPrestateV1, ObserverV1, OccurrenceV1,
        PhaseV1, PresenceV1, PrestateV1, ReachabilityV1, RunnerCapabilityV1, SourceSiteV1,
        StimulusV1, TerminalDescriptorV1, TimingV1, ValidityV1,
    },
};
use super::*;

mod descriptor_binding;
mod manifest_validation;
mod map_validation;
mod producer_coherence;
mod program_admission;
mod program_inventory;
mod runner_admission;
#[cfg(windows)]
mod runner_admission_lock_lifecycle_supported;
#[cfg(windows)]
mod runner_admission_lock_supported;
#[cfg(windows)]
mod runner_admission_supported;
mod runner_state;

fn expected(phase: &str) -> ExpectedV1 {
    ExpectedV1 {
        sqlite: SqliteResultV1::MapUnavailable,
        disposition: TerminalDispositionV1::Returned,
        phase: phase.to_owned(),
        failure: FailureClassV1::ProtocolViolation,
        mutation: MutationStateV1::None,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::NotReached,
        dms_lock: DmsLockCustodyV1::NotReached,
        raw_slots: CustodyStateV1::NotReached,
        route: CustodyStateV1::NotReached,
        callback: CustodyStateV1::NotReached,
        file: CustodyStateV1::NotReached,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1::default(),
    }
}

fn record(leaf: &str, branch: &str) -> LeafRecordV1 {
    LeafRecordV1 {
        key: CaseKeyV1 {
            identity: LeafIdentityV1 {
                root: RootOperationV1::Map,
                leaf_id: leaf.to_owned(),
                family_id: format!("family-{leaf}"),
                coordinates: vec![CoordinateV1 {
                    name: "identity-only".to_owned(),
                    value: leaf.to_owned(),
                }],
            },
            decisions: vec![super::super::source_leaf_authority::DecisionV1 {
                stage: super::super::source_leaf_authority::DecisionStageV1::AbiValidation,
                branch: branch.to_owned(),
            }],
        },
        source_branch: vec![SourceWitnessV1 {
            owner_id: "identity-only-owner".to_owned(),
            symbol: leaf.to_owned(),
            needle: branch.to_owned(),
            occurrence: 1,
        }],
        outcome: LeafOutcomeV1::Terminal(expected("AbiValidation")),
    }
}

fn descriptor(capability: RunnerCapabilityV1) -> TerminalDescriptorV1 {
    TerminalDescriptorV1::map(
        SourceSiteV1::MapAbiBoundary,
        StimulusV1::MapAbi(MapAbiScalarV1 {
            output: PresenceV1::Present,
            region: ValidityV1::Invalid,
            region_size: ValidityV1::Valid,
            extend: ValidityV1::Valid,
        }),
        PrestateV1::Map(MapPrestateV1::NotReached),
        MapOperationV1::AbiValidation,
        PhaseV1::AbiValidation,
        TimingV1::BeforeCall,
        OccurrenceV1::Natural,
        ExecutionRecipeV1::new(
            FixtureV1::AbiRawOnly,
            CallbackV1::XShmMap,
            FaultSeamV1::AbiBoundary,
            ObserverV1::MapCallbackAndSnapshot,
            CleanupV1::ParentOwnedRoot,
            capability,
        ),
        MapAxesV1 {
            completion: ReachabilityV1::Reached(MapCompletionV1::Direct),
            ..MapAxesV1::NOT_REACHED
        },
    )
}

fn lock_request_rejection_descriptor(
    stimulus: LockManagedStimulusV1,
    callback: CallbackV1,
) -> TerminalDescriptorV1 {
    TerminalDescriptorV1::lock(
        SourceSiteV1::ManagedRequestValidation,
        StimulusV1::LockManaged(stimulus),
        PrestateV1::Lock(LockPrestateV1::NotReached),
        LockOperationV1::ManagedRequest,
        PhaseV1::RequestValidation,
        TimingV1::BeforeCall,
        OccurrenceV1::Natural,
        ExecutionRecipeV1::new(
            FixtureV1::ManagedWalMainSingleConnection,
            callback,
            FaultSeamV1::ManagedRequest,
            ObserverV1::LockCallbackAndSnapshot,
            CleanupV1::ParentOwnedRoot,
            RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        ),
        LockAxesV1 {
            action: ReachabilityV1::Reached(LockActionV1::LockShared),
            completion: ReachabilityV1::Reached(LockCompletionV1::Direct),
            ..LockAxesV1::NOT_REACHED
        },
    )
}

fn lock_request_record() -> LeafRecordV1 {
    let mut value = record("lock-request", "range-overflow");
    value.key.identity.root = RootOperationV1::Lock;
    let LeafOutcomeV1::Terminal(expected) = &mut value.outcome else {
        unreachable!()
    };
    expected.sqlite = SqliteResultV1::LockUnavailable;
    expected.phase = "RequestValidation".to_owned();
    value
}

fn with_source_site(
    mut descriptor: TerminalDescriptorV1,
    source_site: SourceSiteV1,
) -> TerminalDescriptorV1 {
    match &mut descriptor {
        TerminalDescriptorV1::Map(value) => value.source_site = source_site,
        TerminalDescriptorV1::Lock(value) => value.source_site = source_site,
    }
    descriptor
}

fn seal(record: &LeafRecordV1) -> LeafSealV1 {
    LeafSealV1 {
        root: RootOperationV1::Map,
        leaf_id: record.key.identity.leaf_id.clone(),
        outcome: LeafSealOutcomeV1::Terminal,
        shard: 0,
        source_leaf_identity_sha256: super::super::source_leaf_authority::Digest32::ZERO,
        case_key_sha256: super::super::source_leaf_authority::digest_case_key(&record.key),
        source_branch_sha256: super::super::source_leaf_authority::Digest32::ZERO,
        expected_sha256: Some(super::super::source_leaf_authority::Digest32::ZERO),
        exclusion_sha256: None,
        full_record_sha256: super::super::source_leaf_authority::digest_full_record(record),
    }
}

fn synthetic_projection_for_test(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> DynamicProjectionV1 {
    let validated = project_validated_dynamic_terminal_v1(record, descriptor).unwrap();
    DynamicProjectionV1 {
        key: validated.semantic_key,
        class_key_sha256: digest_dynamic_class_key_v1(&validated.semantic_key),
        member: validated.descriptor_binding.member,
    }
}

fn observe_synthetic_for_test(
    builder: &mut DynamicCatalogBuilderV1,
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) {
    builder
        .observe_synthetic_projection_for_test(record, descriptor, &seal(record))
        .unwrap();
}

#[test]
fn semantic_projection_erases_leaf_and_case_identity_only() {
    let left = record("left", "left-branch");
    let right = record("right", "right-branch");
    let descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let left_projection = synthetic_projection_for_test(&left, &descriptor);
    let right_projection = synthetic_projection_for_test(&right, &descriptor);

    assert_eq!(left_projection.key, right_projection.key);
    assert_eq!(
        left_projection.class_key_sha256,
        right_projection.class_key_sha256
    );
    assert_ne!(left_projection.member, right_projection.member);
    assert_ne!(
        digest_dynamic_expected_v1(&left_projection.key.expected),
        left_projection.class_key_sha256
    );
}

#[test]
fn catalog_forms_one_exact_class_and_manifest_commits_both_members() {
    let left = record("left", "left-branch");
    let right = record("right", "right-branch");
    let descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let mut builder = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    observe_synthetic_for_test(&mut builder, &right, &descriptor);
    observe_synthetic_for_test(&mut builder, &left, &descriptor);
    let catalog = builder.finish().unwrap();
    assert_eq!(catalog.member_count(), 2);
    assert_eq!(catalog.classes().len(), 1);
    assert_eq!(catalog.classes()[0].members().len(), 2);
    assert_eq!(
        catalog.classes()[0].representative(),
        *catalog.classes()[0].members().iter().min().unwrap()
    );

    let binding = FrozenStaticBindingV1 {
        context: ManifestContextV1 {
            schema: "static-v1".to_owned(),
            root: RootOperationV1::Map,
            target_scope: "windows-x64".to_owned(),
            source_baseline_commit_sha1: "a".repeat(40),
            source_scope_sha256: super::super::source_leaf_authority::Digest32([1; 32]),
            ledger_sha256: super::super::source_leaf_authority::Digest32([2; 32]),
            map_profile_set_sha256: Some(super::super::source_leaf_authority::Digest32([3; 32])),
            map_ordinal_domain_sha256: Some(super::super::source_leaf_authority::Digest32([4; 32])),
            lock_range_set_sha256: None,
            lock_range_count: None,
        },
        included_count: 2,
        excluded_count: 0,
        source_universe_count: 2,
        static_manifest_sha256: super::super::source_leaf_authority::Digest32([5; 32]),
        included_member_pair_set_sha256: catalog.member_pair_set_sha256(),
    };
    let bundle = build_dynamic_manifest_v1(&binding, &catalog).unwrap();
    assert_eq!(bundle.manifest.class_count, 1);
    assert_eq!(bundle.manifest.member_count, 2);
    assert_eq!(bundle.reverse_index.len(), 2);
    assert_ne!(
        bundle.manifest.manifest_sha256,
        super::super::source_leaf_authority::Digest32::ZERO
    );
}

#[test]
fn missing_runner_capability_blocks_the_whole_catalog() {
    let record = record("blocked", "blocked-branch");
    let supported_seal = seal(&record);
    let descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let mut builder = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    builder
        .observe(StreamedLeafV1::Terminal {
            record: &record,
            descriptor: &descriptor,
            seal: &supported_seal,
        })
        .unwrap();
    assert!(matches!(
        builder.finish(),
        Err(CatalogErrorV1::RunnerCapabilityMissing {
            count: 1,
            gap: CapabilityGapV1::QuotientRunnerNotIntegrated,
            ..
        })
    ));
}

#[test]
fn mixed_runner_capability_gaps_block_the_whole_catalog() {
    let first_record = record("first-gap", "first-gap-branch");
    let second_record = record("second-gap", "second-gap-branch");
    let first_seal = seal(&first_record);
    let second_seal = seal(&second_record);
    let first_descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let mut builder = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    builder
        .observe(StreamedLeafV1::Terminal {
            record: &first_record,
            descriptor: &first_descriptor,
            seal: &first_seal,
        })
        .unwrap();
    builder
        .inject_runner_gap_for_test(&second_seal, CapabilityGapV1::LockObservationIncomplete)
        .unwrap();
    assert!(matches!(
        builder.finish(),
        Err(CatalogErrorV1::MixedRunnerCapabilityGaps {
            count: 2,
            first: ProjectionFailureV1 {
                error: ProjectionErrorV1::RunnerCapabilityMissing(
                    CapabilityGapV1::QuotientRunnerNotIntegrated
                ),
                ..
            },
            conflicting: ProjectionFailureV1 {
                error: ProjectionErrorV1::RunnerCapabilityMissing(
                    CapabilityGapV1::LockObservationIncomplete
                ),
                ..
            },
        })
    ));
}

#[test]
fn typed_phase_must_match_the_static_expected() {
    let mut record = record("phase", "branch");
    let LeafOutcomeV1::Terminal(expected) = &mut record.outcome else {
        unreachable!()
    };
    expected.phase = "Success".to_owned();
    assert!(matches!(
        project_dynamic_class_v1(&record, &descriptor(RunnerCapabilityV1::Supported)),
        Err(ProjectionErrorV1::StaticPhaseMismatch {
            typed: PhaseV1::AbiValidation
        })
    ));
}

#[test]
fn lock_action_only_axes_are_reserved_for_exact_request_rejections() {
    let record = lock_request_record();
    assert!(matches!(
        project_dynamic_class_v1(
            &record,
            &lock_request_rejection_descriptor(
                LockManagedStimulusV1::RangeOverflow,
                CallbackV1::XShmLock,
            ),
        ),
        Err(ProjectionErrorV1::RunnerCapabilityMissing(
            CapabilityGapV1::LockObservationIncomplete
        ))
    ));
    assert!(matches!(
        project_dynamic_class_v1(
            &record,
            &lock_request_rejection_descriptor(
                LockManagedStimulusV1::LocalState,
                CallbackV1::XShmLock,
            ),
        ),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::LockRequestRejectionDescriptorMismatch
        ))
    ));
}

#[test]
fn missing_capability_does_not_mask_an_invalid_recipe_root() {
    let record = lock_request_record();
    assert!(matches!(
        project_dynamic_class_v1(
            &record,
            &lock_request_rejection_descriptor(
                LockManagedStimulusV1::RangeOverflow,
                CallbackV1::XShmMap,
            ),
        ),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::CallbackRoot
        ))
    ));
}

#[test]
fn missing_capability_does_not_mask_a_cross_root_observer() {
    let record = record("map-cross-root-observer", "invalid-observer");
    let mut descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!()
    };
    value.recipe.observer = ObserverV1::LockCallbackAndSnapshot;
    assert!(matches!(
        project_dynamic_class_v1(&record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::ObserverRoot
        ))
    ));

    let record = lock_request_record();
    let mut descriptor = lock_request_rejection_descriptor(
        LockManagedStimulusV1::RangeOverflow,
        CallbackV1::XShmLock,
    );
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!()
    };
    value.recipe.observer = ObserverV1::MapCallbackAndSnapshot;
    assert!(matches!(
        project_dynamic_class_v1(&record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::ObserverRoot
        ))
    ));
}

#[test]
fn lock_completion_must_be_explicit_before_missing_capability() {
    let record = lock_request_record();
    let mut descriptor = lock_request_rejection_descriptor(
        LockManagedStimulusV1::RangeOverflow,
        CallbackV1::XShmLock,
    );
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!()
    };
    value.axes.completion = ReachabilityV1::NotReached;
    assert!(matches!(
        project_dynamic_class_v1(&record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::LockCompletionNotReached
        ))
    ));
}

#[test]
fn map_rejects_a_lock_specific_source_site_before_missing_capability() {
    let record = record("map-cross-root-source", "invalid-region");
    let descriptor = with_source_site(
        descriptor(RunnerCapabilityV1::Missing(
            CapabilityGapV1::QuotientRunnerNotIntegrated,
        )),
        SourceSiteV1::LockNativeAcquire,
    );
    assert!(matches!(
        project_dynamic_class_v1(&record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::SourceSiteRoot
        ))
    ));
}

#[test]
fn lock_rejects_a_map_specific_source_site_before_missing_capability() {
    let record = lock_request_record();
    let descriptor = with_source_site(
        lock_request_rejection_descriptor(
            LockManagedStimulusV1::RangeOverflow,
            CallbackV1::XShmLock,
        ),
        SourceSiteV1::MapFileGrow,
    );
    assert!(matches!(
        project_dynamic_class_v1(&record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::SourceSiteRoot
        ))
    ));
}
