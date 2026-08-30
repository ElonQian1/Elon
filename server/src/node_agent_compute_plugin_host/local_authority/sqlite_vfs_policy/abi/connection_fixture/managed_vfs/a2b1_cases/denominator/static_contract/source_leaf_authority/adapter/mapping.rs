use super::super::super::{model as graph, source};
use super::super::{
    expected::{
        CustodyStateV1, DmsLockCustodyV1, ExpectedV1, FailureClassV1, LockEffectV1, LockModeV1,
        MutationStateV1, ObservableCountsV1, SqliteResultV1, TerminalDispositionV1,
    },
    model::{
        DecisionStageV1, DecisionV1, ExclusionKindV1, ExclusionProofV1, RootOperationV1,
        SourceWitnessV1,
    },
};

pub(super) const fn root(value: graph::RootOperation) -> RootOperationV1 {
    match value {
        graph::RootOperation::Map => RootOperationV1::Map,
        graph::RootOperation::Lock => RootOperationV1::Lock,
    }
}

pub(super) fn decision(value: &graph::Decision) -> DecisionV1 {
    DecisionV1 {
        stage: decision_stage(value.stage),
        branch: value.branch.clone(),
    }
}

const fn decision_stage(value: graph::DecisionStage) -> DecisionStageV1 {
    match value {
        graph::DecisionStage::AbiValidation => DecisionStageV1::AbiValidation,
        graph::DecisionStage::RawAdmission => DecisionStageV1::RawAdmission,
        graph::DecisionStage::RawAbandon => DecisionStageV1::RawAbandon,
        graph::DecisionStage::Adapter => DecisionStageV1::Adapter,
        graph::DecisionStage::CallbackAdmission => DecisionStageV1::CallbackAdmission,
        graph::DecisionStage::ManagedRequest => DecisionStageV1::ManagedRequest,
        graph::DecisionStage::Initialization => DecisionStageV1::Initialization,
        graph::DecisionStage::Coordination => DecisionStageV1::Coordination,
        graph::DecisionStage::NativeCall => DecisionStageV1::NativeCall,
        graph::DecisionStage::Cleanup => DecisionStageV1::Cleanup,
        graph::DecisionStage::Quarantine => DecisionStageV1::Quarantine,
        graph::DecisionStage::CallbackCompletion => DecisionStageV1::CallbackCompletion,
        graph::DecisionStage::AbiProjection => DecisionStageV1::AbiProjection,
    }
}

pub(super) fn expected(value: graph::Expected) -> ExpectedV1 {
    ExpectedV1 {
        sqlite: sqlite(value.sqlite),
        disposition: disposition(value.disposition),
        phase: value.phase.to_owned(),
        failure: failure(value.failure),
        mutation: mutation(value.mutation),
        lock_outcome_uncertain: value.lock_outcome_uncertain,
        lock_effect: lock_effect(value.lock_effect),
        dms_lock: dms_lock(value.dms_lock),
        raw_slots: custody(value.raw_slots),
        route: custody(value.route),
        callback: custody(value.callback),
        file: custody(value.file),
        mapping: custody(value.mapping),
        view: custody(value.view),
        payload: custody(value.payload),
        counts: ObservableCountsV1 {
            callback_begin: value.counts.callback_begin,
            callback_complete: value.counts.callback_complete,
            native_lock: value.counts.native_lock,
            native_unlock: value.counts.native_unlock,
            file_grow: value.counts.file_grow,
            mapping_create: value.counts.mapping_create,
            view_map: value.counts.view_map,
        },
    }
}

const fn sqlite(value: graph::SqliteResult) -> SqliteResultV1 {
    match value {
        graph::SqliteResult::Ok => SqliteResultV1::Ok,
        graph::SqliteResult::Busy => SqliteResultV1::Busy,
        graph::SqliteResult::MapUnavailable => SqliteResultV1::MapUnavailable,
        graph::SqliteResult::LockUnavailable => SqliteResultV1::LockUnavailable,
    }
}

const fn disposition(value: graph::TerminalDisposition) -> TerminalDispositionV1 {
    match value {
        graph::TerminalDisposition::Returned => TerminalDispositionV1::Returned,
        graph::TerminalDisposition::Abandoned => TerminalDispositionV1::Abandoned,
        graph::TerminalDisposition::Quarantined => TerminalDispositionV1::Quarantined,
        graph::TerminalDisposition::CleanupRewritten => TerminalDispositionV1::CleanupRewritten,
    }
}

const fn failure(value: graph::FailureClass) -> FailureClassV1 {
    match value {
        graph::FailureClass::None => FailureClassV1::None,
        graph::FailureClass::ProtocolViolation => FailureClassV1::ProtocolViolation,
        graph::FailureClass::RegistryRejected => FailureClassV1::RegistryRejected,
        graph::FailureClass::BusyNoMutation => FailureClassV1::BusyNoMutation,
        graph::FailureClass::BusyAfterKnownMutation => FailureClassV1::BusyAfterKnownMutation,
        graph::FailureClass::NotPresent => FailureClassV1::NotPresent,
        graph::FailureClass::IoBeforeMutation => FailureClassV1::IoBeforeMutation,
        graph::FailureClass::MutatedButKnown => FailureClassV1::MutatedButKnown,
        graph::FailureClass::OutcomeUncertainPoisoned => FailureClassV1::OutcomeUncertainPoisoned,
        graph::FailureClass::PlatformUnsupported => FailureClassV1::PlatformUnsupported,
    }
}

const fn mutation(value: graph::MutationState) -> MutationStateV1 {
    match value {
        graph::MutationState::None => MutationStateV1::None,
        graph::MutationState::Known => MutationStateV1::Known,
        graph::MutationState::Uncertain => MutationStateV1::Uncertain,
    }
}

const fn custody(value: graph::CustodyState) -> CustodyStateV1 {
    match value {
        graph::CustodyState::NotReached => CustodyStateV1::NotReached,
        graph::CustodyState::Unchanged => CustodyStateV1::Unchanged,
        graph::CustodyState::Released => CustodyStateV1::Released,
        graph::CustodyState::Retained => CustodyStateV1::Retained,
        graph::CustodyState::Quarantined => CustodyStateV1::Quarantined,
        graph::CustodyState::Cleared => CustodyStateV1::Cleared,
    }
}

const fn lock_mode(value: graph::LockMode) -> LockModeV1 {
    match value {
        graph::LockMode::Shared => LockModeV1::Shared,
        graph::LockMode::Exclusive => LockModeV1::Exclusive,
    }
}

const fn lock_effect(value: graph::LockEffect) -> LockEffectV1 {
    match value {
        graph::LockEffect::NotReached => LockEffectV1::NotReached,
        graph::LockEffect::Unchanged => LockEffectV1::Unchanged,
        graph::LockEffect::Acquired { mode, mask, native } => LockEffectV1::Acquired {
            mode: lock_mode(mode),
            mask,
            native,
        },
        graph::LockEffect::Released { mode, mask, native } => LockEffectV1::Released {
            mode: lock_mode(mode),
            mask,
            native,
        },
        graph::LockEffect::OutcomeUncertain { mode, mask } => LockEffectV1::OutcomeUncertain {
            mode: lock_mode(mode),
            mask,
        },
    }
}

const fn dms_lock(value: graph::DmsLockCustody) -> DmsLockCustodyV1 {
    match value {
        graph::DmsLockCustody::NotReached => DmsLockCustodyV1::NotReached,
        graph::DmsLockCustody::UnknownRetained => DmsLockCustodyV1::UnknownRetained,
        graph::DmsLockCustody::UnobservedRetained => DmsLockCustodyV1::UnobservedRetained,
        graph::DmsLockCustody::ExistingShared => DmsLockCustodyV1::ExistingShared,
        graph::DmsLockCustody::AcquiredShared => DmsLockCustodyV1::AcquiredShared,
        graph::DmsLockCustody::Released => DmsLockCustodyV1::Released,
        graph::DmsLockCustody::ExclusiveKnown => DmsLockCustodyV1::ExclusiveKnown,
        graph::DmsLockCustody::ExclusiveOutcomeUncertain => {
            DmsLockCustodyV1::ExclusiveOutcomeUncertain
        }
    }
}

pub(super) fn exclusion(value: &graph::ExclusionProof) -> ExclusionProofV1 {
    let (kind, reason) = match value {
        graph::ExclusionProof::TypeInvariant(reason) => (ExclusionKindV1::TypeInvariant, *reason),
        graph::ExclusionProof::ControlFlow(reason) => (ExclusionKindV1::ControlFlow, *reason),
        graph::ExclusionProof::SafetyPremise(reason) => (ExclusionKindV1::SafetyPremise, *reason),
    };
    ExclusionProofV1 {
        kind,
        reason: reason.to_owned(),
    }
}

pub(super) fn witness(value: source::SourceWitness) -> SourceWitnessV1 {
    SourceWitnessV1 {
        owner_id: owner_id(value.owner).to_owned(),
        symbol: value.symbol.to_owned(),
        needle: value.needle.to_owned(),
        occurrence: value.occurrence,
    }
}

const fn owner_id(value: source::ProductionOwner) -> &'static str {
    match value {
        source::ProductionOwner::SqliteVfsAbiTable => "sqlite-vfs-abi-table",
        source::ProductionOwner::AbiBoundary => "abi-boundary",
        source::ProductionOwner::AbiIoShm => "abi-io-shm",
        source::ProductionOwner::AbiFileState => "abi-file-state",
        source::ProductionOwner::AbiRawState => "abi-raw-state",
        source::ProductionOwner::AbiResultCodes => "abi-result-codes",
        source::ProductionOwner::RegistryAbiFile => "registry-abi-file",
        source::ProductionOwner::RegistryOperations => "registry-operations",
        source::ProductionOwner::RegistryFileCustody => "registry-file-custody",
        source::ProductionOwner::RegistryProcessOwner => "registry-process-owner",
        source::ProductionOwner::RegistryProcessLifecycle => "registry-process-lifecycle",
        source::ProductionOwner::RegistryOwner => "registry-owner",
        source::ProductionOwner::RegistryOwnerLifecycle => "registry-owner-lifecycle",
        source::ProductionOwner::RegistryState => "registry-state",
        source::ProductionOwner::ManagedNamespace => "managed-namespace",
        source::ProductionOwner::ManagedNamespaceTypes => "managed-namespace-types",
        source::ProductionOwner::ManagedFsRoot => "managed-fs-root",
        source::ProductionOwner::ManagedWindowsPlatform => "managed-windows-platform",
        source::ProductionOwner::ManagedShmRoot => "managed-shm-root",
        source::ProductionOwner::ManagedCoordinator => "managed-coordinator",
        source::ProductionOwner::ManagedTypes => "managed-types",
        source::ProductionOwner::ManagedInitialization => "managed-initialization",
        source::ProductionOwner::ManagedFailureCustody => "managed-failure-custody",
        source::ProductionOwner::ManagedMapping => "managed-mapping",
        source::ProductionOwner::ManagedLocking => "managed-locking",
        source::ProductionOwner::ManagedNamespaceIo => "managed-namespace-io",
        source::ProductionOwner::ManagedNamespaceClose => "managed-namespace-close",
        source::ProductionOwner::WindowsShm => "windows-shm",
        source::ProductionOwner::WindowsLocking => "windows-locking",
    }
}
