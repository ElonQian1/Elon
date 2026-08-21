//! Review-stage raw-source projection.
//!
//! This module intentionally does not construct `CaseKey` or `Expected`. One raw source branch may
//! still expand into several keys after initialization, prefix mutation, operation pre-state,
//! occurrence and cleanup-terminal review. Treating this projection as StaticContract is a type
//! error.

use std::num::NonZeroU32;

use super::{
    branch_atoms::{
        AbiLockBranch, AbiMapBranch, CallbackBranch, InitializationBranch, Invoker, LockBranch,
        MapBranch, RawSourceBranchAtomId, RouteBridgeBranch,
    },
    case_key::{
        BranchGroup, CleanupCause, FailureClass, InitializationPath, LockAction,
        LockOwnershipShape, LockRange, MapMode, NodePrecondition, Operation, Path, Phase,
        PrefixMutation, RegionPrecondition, RegionShape, Timing,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CandidateAxis<T> {
    Exact(T),
    SplitRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ExpectedStatus {
    PendingSourceAndRedTeamReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CandidatePrestate {
    Map {
        node: NodePrecondition,
        region: RegionPrecondition,
        region_shape: RegionShape,
    },
    Lock {
        own: LockOwnershipShape,
        sibling: LockOwnershipShape,
    },
}

/// A lossy review aid, not `CaseKey`. Exact values are retained only where the raw source owner
/// fixes them without requiring Expected-equivalence decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CandidateKeyProjection {
    pub(super) raw: RawSourceBranchAtomId,
    pub(super) path: CandidateAxis<Path>,
    pub(super) invoker: Option<Invoker>,
    pub(super) operation: CandidateAxis<Operation>,
    pub(super) prestate: CandidateAxis<CandidatePrestate>,
    pub(super) range: CandidateAxis<LockRange>,
    pub(super) branch_group: CandidateAxis<BranchGroup>,
    pub(super) cause_phase: CandidateAxis<Phase>,
    pub(super) terminal_phase: CandidateAxis<Phase>,
    pub(super) timing: CandidateAxis<Timing>,
    pub(super) prefix_mutation: CandidateAxis<PrefixMutation>,
    pub(super) initialization_path: CandidateAxis<InitializationPath>,
    pub(super) failure: CandidateAxis<FailureClass>,
    pub(super) cleanup: CandidateAxis<CleanupCause>,
    pub(super) occurrence: CandidateAxis<Option<NonZeroU32>>,
    pub(super) expected: ExpectedStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ExclusionReason {
    OutsideSupportedWindowsQuotient,
    ExactFixtureInvariant,
    PriorTerminalStateOutsideCasePrecondition,
    RejectedByOperationControlFlow,
    DefensiveCorruptionBranch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExcludedDefensiveBranch {
    pub(super) raw: RawSourceBranchAtomId,
    pub(super) reason: ExclusionReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawDisposition {
    Included(CandidateKeyProjection),
    Excluded(ExcludedDefensiveBranch),
}

pub(super) fn disposition(raw: RawSourceBranchAtomId) -> RawDisposition {
    if let Some(reason) = exclusion_reason(raw) {
        return RawDisposition::Excluded(ExcludedDefensiveBranch { raw, reason });
    }
    RawDisposition::Included(candidate(raw))
}

fn candidate(raw: RawSourceBranchAtomId) -> CandidateKeyProjection {
    let invoker = invoker(raw);
    let (branch_group, cause_phase) = fixed_axes(raw);
    CandidateKeyProjection {
        raw,
        path: invoker
            .map(|value| CandidateAxis::Exact(value.path()))
            .unwrap_or_else(|| raw_path(raw)),
        invoker,
        operation: CandidateAxis::SplitRequired,
        prestate: CandidateAxis::SplitRequired,
        range: CandidateAxis::SplitRequired,
        branch_group,
        cause_phase,
        terminal_phase: CandidateAxis::SplitRequired,
        timing: CandidateAxis::SplitRequired,
        prefix_mutation: prefix_axis(raw),
        initialization_path: initialization_axis(raw),
        failure: failure_axis(raw),
        cleanup: CandidateAxis::SplitRequired,
        occurrence: CandidateAxis::SplitRequired,
        expected: ExpectedStatus::PendingSourceAndRedTeamReview,
    }
}

fn fixed_axes(raw: RawSourceBranchAtomId) -> (CandidateAxis<BranchGroup>, CandidateAxis<Phase>) {
    use CandidateAxis::{Exact, SplitRequired};
    match raw {
        RawSourceBranchAtomId::AbiMap(
            AbiMapBranch::InvalidRegion
            | AbiMapBranch::InvalidRegionSize
            | AbiMapBranch::InvalidExtendFlag
            | AbiMapBranch::NullOutput
            | AbiMapBranch::RawFileRejected,
        )
        | RawSourceBranchAtomId::AbiLock(
            AbiLockBranch::InvalidOffset
            | AbiLockBranch::ZeroCount
            | AbiLockBranch::InvalidFlags
            | AbiLockBranch::RawFileRejected,
        ) => (
            Exact(BranchGroup::AbiValidation),
            Exact(Phase::AbiValidation),
        ),
        RawSourceBranchAtomId::AbiMap(
            AbiMapBranch::ReturnedRegionMismatch
            | AbiMapBranch::ReturnedLengthMismatch
            | AbiMapBranch::ReturnedNullPointer,
        )
        | RawSourceBranchAtomId::AbiLock(
            AbiLockBranch::ManagedRangeOverflow
            | AbiLockBranch::ManagedRangeInvalid
            | AbiLockBranch::ManagedRangeOutOfRange
            | AbiLockBranch::ManagedSharedMulti,
        ) => (
            Exact(BranchGroup::ManagedValidation),
            Exact(Phase::RequestValidation),
        ),
        RawSourceBranchAtomId::AbiMap(
            AbiMapBranch::ManagedNotPresent
            | AbiMapBranch::ManagedMapped
            | AbiMapBranch::ManagedFailure,
        )
        | RawSourceBranchAtomId::AbiLock(
            AbiLockBranch::ManagedAcquired
            | AbiLockBranch::ManagedBusy
            | AbiLockBranch::ManagedFailure,
        ) => (SplitRequired, SplitRequired),
        RawSourceBranchAtomId::Callback { .. } => {
            (Exact(BranchGroup::RegistryCallback), SplitRequired)
        }
        RawSourceBranchAtomId::RouteBridge(_) => {
            (Exact(BranchGroup::ManagedValidation), SplitRequired)
        }
        RawSourceBranchAtomId::Initialization { .. } => {
            (Exact(BranchGroup::Initialization), SplitRequired)
        }
        RawSourceBranchAtomId::Map { .. } | RawSourceBranchAtomId::Lock { .. } => {
            (SplitRequired, SplitRequired)
        }
        RawSourceBranchAtomId::FaultController(_) => {
            (Exact(BranchGroup::InjectedFault), SplitRequired)
        }
    }
}

fn invoker(raw: RawSourceBranchAtomId) -> Option<Invoker> {
    match raw {
        RawSourceBranchAtomId::Callback { invoker, .. }
        | RawSourceBranchAtomId::Initialization { invoker, .. } => Some(invoker),
        RawSourceBranchAtomId::Map { mode, .. } => Some(match mode {
            MapMode::Observe => Invoker::MapObserve,
            MapMode::Extend => Invoker::MapExtend,
            MapMode::Invalid => return None,
        }),
        RawSourceBranchAtomId::Lock { action, .. } => Some(match action {
            LockAction::LockShared => Invoker::LockShared,
            LockAction::LockExclusive => Invoker::LockExclusive,
            LockAction::UnlockShared => Invoker::UnlockShared,
            LockAction::UnlockExclusive => Invoker::UnlockExclusive,
            LockAction::Invalid => return None,
        }),
        RawSourceBranchAtomId::AbiMap(_)
        | RawSourceBranchAtomId::AbiLock(_)
        | RawSourceBranchAtomId::RouteBridge(_)
        | RawSourceBranchAtomId::FaultController(_) => None,
    }
}

fn raw_path(raw: RawSourceBranchAtomId) -> CandidateAxis<Path> {
    match raw {
        RawSourceBranchAtomId::AbiLock(_) | RawSourceBranchAtomId::Lock { .. } => {
            CandidateAxis::Exact(Path::Lock)
        }
        RawSourceBranchAtomId::AbiMap(_)
        | RawSourceBranchAtomId::Map { .. }
        | RawSourceBranchAtomId::RouteBridge(_) => CandidateAxis::Exact(Path::Map),
        RawSourceBranchAtomId::Callback { invoker, .. }
        | RawSourceBranchAtomId::Initialization { invoker, .. } => {
            CandidateAxis::Exact(invoker.path())
        }
        RawSourceBranchAtomId::FaultController(_) => CandidateAxis::SplitRequired,
    }
}

fn prefix_axis(raw: RawSourceBranchAtomId) -> CandidateAxis<PrefixMutation> {
    if rejects_before_managed_action(raw) {
        CandidateAxis::Exact(PrefixMutation::NotReached)
    } else {
        CandidateAxis::SplitRequired
    }
}

fn initialization_axis(raw: RawSourceBranchAtomId) -> CandidateAxis<InitializationPath> {
    if rejects_before_managed_action(raw) {
        CandidateAxis::Exact(InitializationPath::NotReached)
    } else {
        CandidateAxis::SplitRequired
    }
}

fn failure_axis(raw: RawSourceBranchAtomId) -> CandidateAxis<FailureClass> {
    match raw {
        RawSourceBranchAtomId::AbiMap(
            AbiMapBranch::InvalidRegion
            | AbiMapBranch::InvalidRegionSize
            | AbiMapBranch::InvalidExtendFlag
            | AbiMapBranch::NullOutput,
        )
        | RawSourceBranchAtomId::AbiLock(
            AbiLockBranch::InvalidOffset
            | AbiLockBranch::ZeroCount
            | AbiLockBranch::InvalidFlags
            | AbiLockBranch::ManagedRangeOverflow
            | AbiLockBranch::ManagedRangeInvalid
            | AbiLockBranch::ManagedRangeOutOfRange
            | AbiLockBranch::ManagedSharedMulti,
        ) => CandidateAxis::Exact(FailureClass::ProtocolViolation),
        RawSourceBranchAtomId::Callback {
            branch: CallbackBranch::AdmissionRejected | CallbackBranch::CompletionRejected,
            ..
        } => CandidateAxis::Exact(FailureClass::RegistryRejected),
        _ => CandidateAxis::SplitRequired,
    }
}

fn rejects_before_managed_action(raw: RawSourceBranchAtomId) -> bool {
    matches!(
        raw,
        RawSourceBranchAtomId::AbiMap(
            AbiMapBranch::InvalidRegion
                | AbiMapBranch::InvalidRegionSize
                | AbiMapBranch::InvalidExtendFlag
                | AbiMapBranch::NullOutput
                | AbiMapBranch::RawFileRejected
        ) | RawSourceBranchAtomId::AbiLock(
            AbiLockBranch::InvalidOffset
                | AbiLockBranch::ZeroCount
                | AbiLockBranch::InvalidFlags
                | AbiLockBranch::RawFileRejected
                | AbiLockBranch::ManagedRangeOverflow
                | AbiLockBranch::ManagedRangeInvalid
                | AbiLockBranch::ManagedRangeOutOfRange
                | AbiLockBranch::ManagedSharedMulti
        ) | RawSourceBranchAtomId::Callback {
            branch: CallbackBranch::AdmissionRejected,
            ..
        }
    )
}

fn exclusion_reason(raw: RawSourceBranchAtomId) -> Option<ExclusionReason> {
    use ExclusionReason::*;
    match raw {
        RawSourceBranchAtomId::RouteBridge(RouteBridgeBranch::Prepared) => None,
        RawSourceBranchAtomId::RouteBridge(_) => Some(ExactFixtureInvariant),
        RawSourceBranchAtomId::AbiMap(
            AbiMapBranch::RawFileRejected
            | AbiMapBranch::ReturnedRegionMismatch
            | AbiMapBranch::ReturnedLengthMismatch
            | AbiMapBranch::ReturnedNullPointer,
        )
        | RawSourceBranchAtomId::AbiLock(AbiLockBranch::RawFileRejected)
        | RawSourceBranchAtomId::Callback {
            branch: CallbackBranch::UnsupportedFileRole | CallbackBranch::ShmDetached,
            ..
        }
        | RawSourceBranchAtomId::Map {
            branch: MapBranch::PinnedConnectionInactive,
            ..
        }
        | RawSourceBranchAtomId::Lock {
            branch: LockBranch::PinnedConnectionInactive,
            ..
        } => Some(ExactFixtureInvariant),
        RawSourceBranchAtomId::Initialization {
            branch:
                InitializationBranch::DmsExclusiveNativeUnsupported
                | InitializationBranch::DmsSharedNativeUnsupported,
            ..
        }
        | RawSourceBranchAtomId::Map {
            branch:
                MapBranch::AllocationGranularityUnsupported
                | MapBranch::FileSizeNativeUnsupported
                | MapBranch::MappingCreateNativeUnsupported
                | MapBranch::ViewMapNativeUnsupported,
            ..
        }
        | RawSourceBranchAtomId::Lock {
            branch: LockBranch::AcquireNativeUnsupported,
            ..
        } => Some(OutsideSupportedWindowsQuotient),
        RawSourceBranchAtomId::Initialization {
            branch: InitializationBranch::ExistingPoisoned,
            ..
        }
        | RawSourceBranchAtomId::Map {
            branch: MapBranch::DomainAlreadyPoisoned,
            ..
        } => Some(PriorTerminalStateOutsideCasePrecondition),
        RawSourceBranchAtomId::Initialization {
            branch: InitializationBranch::NodeMissingAfterOpen,
            ..
        }
        | RawSourceBranchAtomId::Map {
            branch:
                MapBranch::AllocationGranularityZero
                | MapBranch::MutexPoisoned
                | MapBranch::ConnectionMissing
                | MapBranch::ViewMapNativeNull
                | MapBranch::RegionCustodyMissing
                | MapBranch::ArithmeticOrNodeInvariant,
            ..
        }
        | RawSourceBranchAtomId::Lock {
            branch:
                LockBranch::CoordinatorPoisoned
                | LockBranch::ConnectionMissing
                | LockBranch::ActionChanged
                | LockBranch::NodeMissing
                | LockBranch::ConnectionDisappearedAfterAction,
            ..
        } => Some(DefensiveCorruptionBranch),
        RawSourceBranchAtomId::Map { mode, branch } if !map_reachable(mode, branch) => {
            Some(RejectedByOperationControlFlow)
        }
        RawSourceBranchAtomId::Lock { action, branch } if !lock_reachable(action, branch) => {
            Some(RejectedByOperationControlFlow)
        }
        _ => None,
    }
}

fn map_reachable(mode: MapMode, branch: MapBranch) -> bool {
    match branch {
        // The production site calls `finish_test_fault`, but the admitted controller phase policy
        // does not expose FileSize after-success selectors.
        MapBranch::FileSizeFaultAfterKnown | MapBranch::FileSizeFaultAfterUncertain => false,
        MapBranch::ObserveNotPresent => mode == MapMode::Observe,
        MapBranch::FileGrowFaultBefore
        | MapBranch::FileGrowFaultAfterKnown
        | MapBranch::FileGrowFaultAfterUncertain
        | MapBranch::FileGrowNativeFailure => mode == MapMode::Extend,
        _ => true,
    }
}

fn lock_reachable(action: LockAction, branch: LockBranch) -> bool {
    use LockAction::{LockExclusive, LockShared, UnlockExclusive, UnlockShared};
    match branch {
        LockBranch::AcquireFaultBefore
        | LockBranch::AcquireFaultAfterKnown
        | LockBranch::AcquireFaultAfterUncertain
        | LockBranch::AcquireNativeBusy
        | LockBranch::AcquireNativeIo
        | LockBranch::AcquireNativeUnsupported
        | LockBranch::AcquireNativeSuccess => matches!(action, LockShared | LockExclusive),
        LockBranch::ReleaseFaultBefore
        | LockBranch::ReleaseFaultAfterKnown
        | LockBranch::ReleaseFaultAfterUncertain
        | LockBranch::ReleaseNativeFailure
        | LockBranch::ReleaseNativeSuccess => matches!(action, UnlockShared | UnlockExclusive),
        LockBranch::SharedSiblingExclusiveContention | LockBranch::SharedLocalCoalescing => {
            action == LockShared
        }
        LockBranch::ExclusiveSiblingContention => action == LockExclusive,
        LockBranch::SharedUnlockNotHeld | LockBranch::SharedLocalRelease => action == UnlockShared,
        LockBranch::ExclusiveUnlockNotHeld
        | LockBranch::ExclusiveRangeMismatch
        | LockBranch::ExclusiveSiblingOverlap => action == UnlockExclusive,
        LockBranch::TransitionNotUnlocked => matches!(action, LockShared | LockExclusive),
        _ => true,
    }
}
