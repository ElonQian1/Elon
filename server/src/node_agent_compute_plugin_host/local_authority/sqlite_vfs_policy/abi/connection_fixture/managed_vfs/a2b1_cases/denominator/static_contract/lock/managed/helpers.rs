use super::super::super::terminal_descriptor::{
    FaultSeamV1, LockManagedStimulusV1, LockOperationV1, LockPrestateV1, PhaseV1, SourceSiteV1,
    TimingV1,
};
use super::super::{
    super::{
        model::{
            CustodyState, DecisionStage, DmsLockCustody, ExclusionProof, FailureClass, LockEffect,
            MutationState, ObservableCounts, TerminalDisposition,
        },
        source::{witness, ProductionOwner, SourceWitness},
    },
    builder::Builder,
    dynamic::SeedV1,
    input::ValidRequest,
    outcome::{self, Shape},
};

pub(super) fn safe_branch(
    builder: &mut Builder,
    from: &str,
    prefix: &str,
    stage: DecisionStage,
    branch: &str,
    source: SourceWitness,
    shape: Shape,
) {
    let cause = builder.continuation(
        format!("{prefix}.cause"),
        "registry operation-error precedence",
        source,
    );
    builder.edge(from, &cause, stage, branch);
    outcome::complete(builder, &cause, prefix, shape);
}

pub(super) fn unsafe_branch(
    builder: &mut Builder,
    from: &str,
    prefix: &str,
    stage: DecisionStage,
    branch: &str,
    source: SourceWitness,
    shape: Shape,
) {
    let cause = builder.continuation(
        format!("{prefix}.cause"),
        "unsafe SHM failure retention",
        source,
    );
    builder.edge(from, &cause, stage, branch);
    outcome::unsafe_failure(builder, &cause, prefix, shape);
}

pub(super) fn protocol(descriptor: SeedV1, phase: &'static str, lock: u16, unlock: u16) -> Shape {
    Shape::failure(
        descriptor,
        phase,
        FailureClass::ProtocolViolation,
        MutationState::None,
        false,
        lock,
        unlock,
    )
    .with_dms_lock(DmsLockCustody::ExistingShared)
}

pub(super) fn add_admission_failures(
    builder: &mut Builder,
    admission: &str,
    request: &ValidRequest,
) {
    let prefix = &request.prefix;
    let owner_poisoned = builder.excluded(
        format!("{prefix}.admission-rejected.excluded.owner-poisoned"),
        super::super::super::poison::owner_mutex_poison_proof(),
        witness(
            ProductionOwner::RegistryProcessOwner,
            "fn lock_routes",
            "ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned",
            1,
        ),
    );
    builder.edge(
        admission,
        &owner_poisoned,
        DecisionStage::CallbackAdmission,
        "owner-poisoned",
    );
    for (branch, stimulus, route, disposition, source) in [
        (
            "route-unknown-prior-quarantine",
            LockManagedStimulusV1::AdmissionRouteUnknown,
            CustodyState::Quarantined,
            TerminalDisposition::Returned,
            witness(
                ProductionOwner::RegistryOwner,
                "fn exact_entry_mut",
                "return Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired);",
                1,
            ),
        ),
        (
            "counter-overflow",
            LockManagedStimulusV1::AdmissionCounterOverflow,
            CustodyState::Quarantined,
            TerminalDisposition::Quarantined,
            witness(
                ProductionOwner::RegistryState,
                "pub(super) fn begin_callback",
                "self.enter_terminal(ManagedSqliteRegistryTerminalReason::CallbackCounterOverflow);",
                1,
            ),
        ),
    ] {
        let mut expected = outcome::unavailable("CallbackAdmission");
        expected.failure = FailureClass::RegistryRejected;
        expected.raw_slots = CustodyState::Unchanged;
        expected.lock_effect = LockEffect::Unchanged;
        expected.route = route;
        expected.file = CustodyState::Unchanged;
        expected.disposition = disposition;
        expected.counts = ObservableCounts {
            callback_begin: 1,
            ..ObservableCounts::default()
        };
        outcome::managed_direct(
            builder,
            admission,
            &format!("{prefix}.admission-rejected.{branch}"),
            DecisionStage::CallbackAdmission,
            branch,
            expected,
            request.descriptor(
                SourceSiteV1::RegistryCallbackAdmission,
                stimulus,
                LockPrestateV1::NotReached,
                LockOperationV1::CallbackAdmission,
                PhaseV1::CallbackAdmission,
                TimingV1::BeforeCall,
                FaultSeamV1::RegistryAdmission,
            ),
            source,
        );
    }

    for (branch, proof, source) in [
        (
            "identity-mismatch",
            ExclusionProof::TypeInvariant(
                "route tokens are never reused and a pinned file retains the immutable handle minted for its exact live route",
            ),
            witness(
                ProductionOwner::RegistryOwner,
                "fn exact_entry_mut",
                "return Err(ManagedSqliteRegistryRouteRejection::IdentityMismatch);",
                1,
            ),
        ),
        (
            "state-shape-invalid",
            ExclusionProof::TypeInvariant(
                "registry session fields are private and every production transition preserves shape before a pinned-file callback can enter",
            ),
            witness(
                ProductionOwner::RegistryState,
                "fn ensure_shape",
                "Err(ManagedSqliteRegistryTransitionRejection::StateInvariantViolated)",
                1,
            ),
        ),
        (
            "terminal-phase",
            ExclusionProof::ControlFlow(
                "the process-owner apply_route error path removes and permanently retains a route as soon as its state enters TerminalQuarantine",
            ),
            witness(
                ProductionOwner::RegistryState,
                "pub(super) fn begin_callback",
                "ManagedSqliteRegistryTransitionRejection::Terminal",
                1,
            ),
        ),
        (
            "wrong-phase",
            ExclusionProof::TypeInvariant(
                "a callable pinned WAL-main file exists only in Opening, Active or Closing, and all three phases admit Shm callbacks",
            ),
            witness(
                ProductionOwner::RegistryState,
                "pub(super) fn begin_callback",
                "ManagedSqliteRegistryTransitionRejection::WrongPhase",
                1,
            ),
        ),
    ] {
        let excluded = builder.excluded(
            format!("{prefix}.admission-rejected.excluded.{branch}"),
            proof,
            source,
        );
        builder.edge(
            admission,
            &excluded,
            DecisionStage::CallbackAdmission,
            branch,
        );
    }
}

pub(super) fn locking_witness(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::ManagedLocking, symbol, needle, 1)
}

pub(super) fn registry_operations(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::RegistryOperations, symbol, needle, 1)
}
