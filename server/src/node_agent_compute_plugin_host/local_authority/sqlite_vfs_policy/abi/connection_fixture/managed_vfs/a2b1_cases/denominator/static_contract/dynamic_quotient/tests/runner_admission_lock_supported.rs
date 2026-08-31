use super::super::runner_admission::{
    compile_for_test, run_lock_isolated_for_test, tamper_lock_implementation_digest_for_test,
    LockRunnerIsolatedOutcomeV1, RunnerAdmissionDecisionV1,
};
use super::*;

const EXACT_TEST_PREFIX: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_lock_supported::";

#[derive(Clone, Copy)]
struct LockProgramCaseV1 {
    label: &'static str,
    action: LockActionV1,
    stimulus: LockManagedStimulusV1,
}

const EXACT_LOCK_PROGRAMS_V1: [LockProgramCaseV1; 10] = [
    LockProgramCaseV1 {
        label: "lock-shared-range-overflow",
        action: LockActionV1::LockShared,
        stimulus: LockManagedStimulusV1::RangeOverflow,
    },
    LockProgramCaseV1 {
        label: "lock-shared-end-past-eight",
        action: LockActionV1::LockShared,
        stimulus: LockManagedStimulusV1::EndPastEight,
    },
    LockProgramCaseV1 {
        label: "lock-shared-shared-multi-slot",
        action: LockActionV1::LockShared,
        stimulus: LockManagedStimulusV1::SharedMultiSlot,
    },
    LockProgramCaseV1 {
        label: "lock-exclusive-range-overflow",
        action: LockActionV1::LockExclusive,
        stimulus: LockManagedStimulusV1::RangeOverflow,
    },
    LockProgramCaseV1 {
        label: "lock-exclusive-end-past-eight",
        action: LockActionV1::LockExclusive,
        stimulus: LockManagedStimulusV1::EndPastEight,
    },
    LockProgramCaseV1 {
        label: "unlock-shared-range-overflow",
        action: LockActionV1::UnlockShared,
        stimulus: LockManagedStimulusV1::RangeOverflow,
    },
    LockProgramCaseV1 {
        label: "unlock-shared-end-past-eight",
        action: LockActionV1::UnlockShared,
        stimulus: LockManagedStimulusV1::EndPastEight,
    },
    LockProgramCaseV1 {
        label: "unlock-shared-shared-multi-slot",
        action: LockActionV1::UnlockShared,
        stimulus: LockManagedStimulusV1::SharedMultiSlot,
    },
    LockProgramCaseV1 {
        label: "unlock-exclusive-range-overflow",
        action: LockActionV1::UnlockExclusive,
        stimulus: LockManagedStimulusV1::RangeOverflow,
    },
    LockProgramCaseV1 {
        label: "unlock-exclusive-end-past-eight",
        action: LockActionV1::UnlockExclusive,
        stimulus: LockManagedStimulusV1::EndPastEight,
    },
];

fn lock_request_record(case: LockProgramCaseV1) -> LeafRecordV1 {
    let leaf = format!("lock-request-{}-supported", case.label);
    let mut value = record(&leaf, case.label);
    value.key.identity.root = RootOperationV1::Lock;
    let LeafOutcomeV1::Terminal(expected) = &mut value.outcome else {
        unreachable!()
    };
    expected.sqlite = SqliteResultV1::LockUnavailable;
    expected.phase = "RequestValidation".to_owned();
    expected.failure = FailureClassV1::ProtocolViolation;
    expected.mutation = MutationStateV1::None;
    expected.lock_effect = LockEffectV1::Unchanged;
    expected.raw_slots = CustodyStateV1::Unchanged;
    expected.file = CustodyStateV1::Unchanged;
    value
}

fn lock_request_descriptor(
    case: LockProgramCaseV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    TerminalDescriptorV1::lock(
        SourceSiteV1::ManagedRequestValidation,
        StimulusV1::LockManaged(case.stimulus),
        PrestateV1::Lock(LockPrestateV1::NotReached),
        LockOperationV1::ManagedRequest,
        PhaseV1::RequestValidation,
        TimingV1::BeforeCall,
        OccurrenceV1::Natural,
        ExecutionRecipeV1::new(
            FixtureV1::ManagedWalMainSingleConnection,
            CallbackV1::XShmLock,
            FaultSeamV1::ManagedRequest,
            ObserverV1::LockCallbackAndSnapshot,
            CleanupV1::ParentOwnedRoot,
            capability,
        ),
        LockAxesV1 {
            action: ReachabilityV1::Reached(case.action),
            completion: ReachabilityV1::Reached(LockCompletionV1::Direct),
            ..LockAxesV1::NOT_REACHED
        },
    )
}

fn supported_key_and_member(case: LockProgramCaseV1) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let record = lock_request_record(case);
    let descriptor = lock_request_descriptor(
        case,
        RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
    );
    let validated = project_validated_dynamic_terminal_v1(&record, &descriptor).unwrap();
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, validated.descriptor_binding.member)
}

#[test]
fn exact_lock_program_table_rejects_naked_supported_declarations() {
    assert_eq!(EXACT_LOCK_PROGRAMS_V1.len(), 10);
    for case in EXACT_LOCK_PROGRAMS_V1 {
        let record = lock_request_record(case);
        let descriptor = lock_request_descriptor(case, RunnerCapabilityV1::Supported);
        assert_eq!(
            project_dynamic_class_v1(&record, &descriptor),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
            "{} must require a private execution receipt",
            case.label,
        );
    }
}

#[test]
fn shared_multi_slot_never_widens_to_exclusive_actions() {
    for action in [LockActionV1::LockExclusive, LockActionV1::UnlockExclusive] {
        let case = LockProgramCaseV1 {
            label: "exclusive-shared-multi-slot-must-not-match",
            action,
            stimulus: LockManagedStimulusV1::SharedMultiSlot,
        };
        assert_eq!(
            project_dynamic_class_v1(
                &lock_request_record(case),
                &lock_request_descriptor(
                    case,
                    RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
                ),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::LockProducerTupleMismatch,
            )),
        );
    }
}

fn exercise_supported_projection(case: LockProgramCaseV1, exact_test: &str) -> anyhow::Result<()> {
    let record = lock_request_record(case);
    let descriptor = lock_request_descriptor(case, RunnerCapabilityV1::Supported);
    let (key, member) = supported_key_and_member(case);
    let plan = compile_for_test(&key);
    let execution = match run_lock_isolated_for_test(exact_test, &key, member, plan)? {
        LockRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        LockRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    let validated =
        project_validated_dynamic_terminal_with_lock_execution_v1(&record, &descriptor, execution)
            .map_err(|error| anyhow::anyhow!("supported Lock projection failed: {error:?}"))?;
    assert!(validated.projection.is_ok());
    assert!(matches!(
        validated.runner_admission.decision(),
        RunnerAdmissionDecisionV1::Supported { .. }
    ));
    Ok(())
}

macro_rules! exact_lock_program_test {
    ($name:ident, $index:expr) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            exercise_supported_projection(
                EXACT_LOCK_PROGRAMS_V1[$index],
                &format!("{EXACT_TEST_PREFIX}{}", stringify!($name)),
            )
        }
    };
}

exact_lock_program_test!(
    isolated_lock_shared_range_overflow_receipt_authorizes_exact_projection,
    0
);
exact_lock_program_test!(
    isolated_lock_shared_end_past_eight_receipt_authorizes_exact_projection,
    1
);
exact_lock_program_test!(
    isolated_lock_shared_shared_multi_slot_receipt_authorizes_exact_projection,
    2
);
exact_lock_program_test!(
    isolated_lock_exclusive_range_overflow_receipt_authorizes_exact_projection,
    3
);
exact_lock_program_test!(
    isolated_lock_exclusive_end_past_eight_receipt_authorizes_exact_projection,
    4
);
exact_lock_program_test!(
    isolated_unlock_shared_range_overflow_receipt_authorizes_exact_projection,
    5
);
exact_lock_program_test!(
    isolated_unlock_shared_end_past_eight_receipt_authorizes_exact_projection,
    6
);
exact_lock_program_test!(
    isolated_unlock_shared_shared_multi_slot_receipt_authorizes_exact_projection,
    7
);
exact_lock_program_test!(
    isolated_unlock_exclusive_range_overflow_receipt_authorizes_exact_projection,
    8
);
exact_lock_program_test!(
    isolated_unlock_exclusive_end_past_eight_receipt_authorizes_exact_projection,
    9
);

#[test]
fn isolated_lock_receipt_rejects_member_replay() -> anyhow::Result<()> {
    let case = EXACT_LOCK_PROGRAMS_V1[0];
    let source_record = lock_request_record(case);
    let descriptor = lock_request_descriptor(case, RunnerCapabilityV1::Supported);
    let (key, member) = supported_key_and_member(case);
    let plan = compile_for_test(&key);
    let exact_test = format!("{EXACT_TEST_PREFIX}isolated_lock_receipt_rejects_member_replay");
    let execution = match run_lock_isolated_for_test(&exact_test, &key, member, plan)? {
        LockRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        LockRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    let mut replayed_record = source_record;
    replayed_record
        .key
        .identity
        .leaf_id
        .push_str("-other-member");
    assert!(project_validated_dynamic_terminal_with_lock_execution_v1(
        &replayed_record,
        &descriptor,
        execution,
    )
    .is_err());
    Ok(())
}

#[test]
fn isolated_lock_receipt_rejects_implementation_digest_tamper() -> anyhow::Result<()> {
    let case = EXACT_LOCK_PROGRAMS_V1[0];
    let record = lock_request_record(case);
    let descriptor = lock_request_descriptor(case, RunnerCapabilityV1::Supported);
    let (key, member) = supported_key_and_member(case);
    let plan = compile_for_test(&key);
    let exact_test =
        format!("{EXACT_TEST_PREFIX}isolated_lock_receipt_rejects_implementation_digest_tamper");
    let mut execution = match run_lock_isolated_for_test(&exact_test, &key, member, plan)? {
        LockRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        LockRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    tamper_lock_implementation_digest_for_test(&mut execution, Digest32([0x6d; 32]));
    assert!(project_validated_dynamic_terminal_with_lock_execution_v1(
        &record,
        &descriptor,
        execution,
    )
    .is_err());
    Ok(())
}

#[test]
fn isolated_lock_receipt_rejects_cross_stimulus_replay() -> anyhow::Result<()> {
    let source = EXACT_LOCK_PROGRAMS_V1[0];
    let target = EXACT_LOCK_PROGRAMS_V1[1];
    let record = lock_request_record(source);
    let (key, member) = supported_key_and_member(source);
    let plan = compile_for_test(&key);
    let exact_test =
        format!("{EXACT_TEST_PREFIX}isolated_lock_receipt_rejects_cross_stimulus_replay");
    let execution = match run_lock_isolated_for_test(&exact_test, &key, member, plan)? {
        LockRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        LockRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    assert!(project_validated_dynamic_terminal_with_lock_execution_v1(
        &record,
        &lock_request_descriptor(target, RunnerCapabilityV1::Supported),
        execution,
    )
    .is_err());
    Ok(())
}

#[test]
fn isolated_lock_receipt_rejects_cross_action_replay() -> anyhow::Result<()> {
    let source = EXACT_LOCK_PROGRAMS_V1[0];
    let target = EXACT_LOCK_PROGRAMS_V1[5];
    let record = lock_request_record(source);
    let (key, member) = supported_key_and_member(source);
    let plan = compile_for_test(&key);
    let exact_test =
        format!("{EXACT_TEST_PREFIX}isolated_lock_receipt_rejects_cross_action_replay");
    let execution = match run_lock_isolated_for_test(&exact_test, &key, member, plan)? {
        LockRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        LockRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    assert!(project_validated_dynamic_terminal_with_lock_execution_v1(
        &record,
        &lock_request_descriptor(target, RunnerCapabilityV1::Supported),
        execution,
    )
    .is_err());
    Ok(())
}
