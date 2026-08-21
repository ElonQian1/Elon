//! Incomplete candidate branch atoms found in the authority-listed production owners.
//!
//! This table is explicitly incomplete and is not a terminal-leaf source universe. The Cartesian
//! products keep operation/branch reachability questions visible while source tracing continues.

use super::case_key::{LockAction, MapMode, Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Invoker {
    MapObserve,
    MapExtend,
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
}

impl Invoker {
    pub(super) const fn path(self) -> Path {
        match self {
            Self::MapObserve | Self::MapExtend => Path::Map,
            Self::LockShared | Self::LockExclusive | Self::UnlockShared | Self::UnlockExclusive => {
                Path::Lock
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AbiMapBranch {
    InvalidRegion,
    InvalidRegionSize,
    InvalidExtendFlag,
    NullOutput,
    RawFileRejected,
    ReturnedRegionMismatch,
    ReturnedLengthMismatch,
    ReturnedNullPointer,
    ManagedNotPresent,
    ManagedMapped,
    ManagedFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AbiLockBranch {
    InvalidOffset,
    ZeroCount,
    InvalidFlags,
    RawFileRejected,
    ManagedRangeOverflow,
    ManagedRangeInvalid,
    ManagedRangeOutOfRange,
    ManagedSharedMulti,
    ManagedAcquired,
    ManagedBusy,
    ManagedFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CallbackBranch {
    AdmissionRejected,
    UnsupportedFileRole,
    ShmDetached,
    CompletionRejected,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RouteBridgeBranch {
    PlanClaimRejected,
    WalPromotionRejected,
    FaultScriptPreparationRejected,
    FaultProbeRecordingRejected,
    Prepared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum InitializationBranch {
    ExistingPoisoned,
    ExactOpenFaultBefore,
    ExactOpenFaultAfterKnown,
    ExactOpenFaultAfterUncertain,
    ExactOpenNativeFailure,
    ExactOpenCleanupCloseFailure,
    DmsExclusiveFaultBefore,
    DmsExclusiveFaultAfterKnown,
    DmsExclusiveFaultAfterUncertain,
    DmsExclusiveNativeContendedPath,
    DmsExclusiveNativeIo,
    DmsExclusiveNativeUnsupported,
    DmsExclusiveCleanupCloseFailure,
    DmsTruncateFaultBefore,
    DmsTruncateFaultAfterKnown,
    DmsTruncateFaultAfterUncertain,
    DmsTruncateNativeReleaseSucceeded,
    DmsTruncateNativeReleaseFailed,
    DmsTruncateCleanupCloseFailure,
    DmsExclusiveReleaseFaultBefore,
    DmsExclusiveReleaseFaultAfterKnown,
    DmsExclusiveReleaseFaultAfterUncertain,
    DmsExclusiveReleaseNativeFailure,
    DmsSharedFaultBefore,
    DmsSharedFaultAfterKnown,
    DmsSharedFaultAfterUncertain,
    DmsSharedNativeBusy,
    DmsSharedNativeIo,
    DmsSharedNativeUnsupported,
    DmsSharedCleanupCloseFailure,
    FirstProcessInitialized,
    JoinerInitialized,
    NodeMissingAfterOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapBranch {
    RegionSizeBudgetRejected,
    LogicalEndBudgetRejected,
    AllocationGranularityIo,
    AllocationGranularityUnsupported,
    AllocationGranularityZero,
    MutexPoisoned,
    ConnectionMissing,
    DomainAlreadyPoisoned,
    PinnedConnectionInactive,
    RegionSizeChanged,
    FileSizeFaultBefore,
    FileSizeFaultAfterKnown,
    FileSizeFaultAfterUncertain,
    FileSizeNativeIo,
    FileSizeNativeUnsupported,
    ExistingSizeBudgetRejected,
    ObserveNotPresent,
    FileGrowFaultBefore,
    FileGrowFaultAfterKnown,
    FileGrowFaultAfterUncertain,
    FileGrowNativeFailure,
    MappingBudgetRejected,
    MappingCreateFaultBefore,
    MappingCreateFaultAfterKnown,
    MappingCreateFaultAfterUncertain,
    MappingCreateNativeIo,
    MappingCreateNativeUnsupported,
    ViewMapFaultBeforeCleanupSucceeded,
    ViewMapFaultBeforeCleanupFailed,
    ViewMapFaultAfterKnown,
    ViewMapFaultAfterUncertain,
    ViewMapNativeIoCleanupSucceeded,
    ViewMapNativeIoCleanupFailed,
    ViewMapNativeUnsupported,
    ViewMapNativeNull,
    MappingCreated,
    MappingReused,
    RegionCustodyMissing,
    ArithmeticOrNodeInvariant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockBranch {
    CoordinatorPoisoned,
    ConnectionMissing,
    PinnedConnectionInactive,
    TransitionNotUnlocked,
    SharedSiblingExclusiveContention,
    SharedLocalCoalescing,
    ExclusiveSiblingContention,
    SharedUnlockNotHeld,
    SharedLocalRelease,
    ExclusiveUnlockNotHeld,
    ExclusiveRangeMismatch,
    ExclusiveSiblingOverlap,
    AcquireFaultBefore,
    AcquireFaultAfterKnown,
    AcquireFaultAfterUncertain,
    AcquireNativeBusy,
    AcquireNativeIo,
    AcquireNativeUnsupported,
    AcquireNativeSuccess,
    ReleaseFaultBefore,
    ReleaseFaultAfterKnown,
    ReleaseFaultAfterUncertain,
    ReleaseNativeFailure,
    ReleaseNativeSuccess,
    ActionChanged,
    NodeMissing,
    ConnectionDisappearedAfterAction,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FaultControllerBranch {
    SelectorPhaseRejected,
    SelectorOccurrenceRejected,
    BeforeTriggerFailed,
    AfterKnownTriggerFailed,
    AfterUncertainTriggerFailed,
    PendingSelectorAtCompletion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawSourceBranchAtomId {
    AbiMap(AbiMapBranch),
    AbiLock(AbiLockBranch),
    Callback {
        invoker: Invoker,
        branch: CallbackBranch,
    },
    RouteBridge(RouteBridgeBranch),
    Initialization {
        invoker: Invoker,
        branch: InitializationBranch,
    },
    Map {
        mode: MapMode,
        branch: MapBranch,
    },
    Lock {
        action: LockAction,
        branch: LockBranch,
    },
    FaultController(FaultControllerBranch),
}

pub(super) const ALL_ABI_MAP_BRANCHES: &[AbiMapBranch] = &[
    AbiMapBranch::InvalidRegion,
    AbiMapBranch::InvalidRegionSize,
    AbiMapBranch::InvalidExtendFlag,
    AbiMapBranch::NullOutput,
    AbiMapBranch::RawFileRejected,
    AbiMapBranch::ReturnedRegionMismatch,
    AbiMapBranch::ReturnedLengthMismatch,
    AbiMapBranch::ReturnedNullPointer,
    AbiMapBranch::ManagedNotPresent,
    AbiMapBranch::ManagedMapped,
    AbiMapBranch::ManagedFailure,
];

pub(super) const ALL_ABI_LOCK_BRANCHES: &[AbiLockBranch] = &[
    AbiLockBranch::InvalidOffset,
    AbiLockBranch::ZeroCount,
    AbiLockBranch::InvalidFlags,
    AbiLockBranch::RawFileRejected,
    AbiLockBranch::ManagedRangeOverflow,
    AbiLockBranch::ManagedRangeInvalid,
    AbiLockBranch::ManagedRangeOutOfRange,
    AbiLockBranch::ManagedSharedMulti,
    AbiLockBranch::ManagedAcquired,
    AbiLockBranch::ManagedBusy,
    AbiLockBranch::ManagedFailure,
];

pub(super) const ALL_CALLBACK_BRANCHES: &[CallbackBranch] = &[
    CallbackBranch::AdmissionRejected,
    CallbackBranch::UnsupportedFileRole,
    CallbackBranch::ShmDetached,
    CallbackBranch::CompletionRejected,
    CallbackBranch::Completed,
];

pub(super) const ALL_INVOKERS: &[Invoker] = &[
    Invoker::MapObserve,
    Invoker::MapExtend,
    Invoker::LockShared,
    Invoker::LockExclusive,
    Invoker::UnlockShared,
    Invoker::UnlockExclusive,
];

pub(super) const INITIALIZATION_INVOKERS: &[Invoker] = &[
    Invoker::MapObserve,
    Invoker::MapExtend,
    Invoker::LockShared,
    Invoker::LockExclusive,
];

pub(super) const ALL_ROUTE_BRIDGE_BRANCHES: &[RouteBridgeBranch] = &[
    RouteBridgeBranch::PlanClaimRejected,
    RouteBridgeBranch::WalPromotionRejected,
    RouteBridgeBranch::FaultScriptPreparationRejected,
    RouteBridgeBranch::FaultProbeRecordingRejected,
    RouteBridgeBranch::Prepared,
];

pub(super) const ALL_INITIALIZATION_BRANCHES: &[InitializationBranch] = &[
    InitializationBranch::ExistingPoisoned,
    InitializationBranch::ExactOpenFaultBefore,
    InitializationBranch::ExactOpenFaultAfterKnown,
    InitializationBranch::ExactOpenFaultAfterUncertain,
    InitializationBranch::ExactOpenNativeFailure,
    InitializationBranch::ExactOpenCleanupCloseFailure,
    InitializationBranch::DmsExclusiveFaultBefore,
    InitializationBranch::DmsExclusiveFaultAfterKnown,
    InitializationBranch::DmsExclusiveFaultAfterUncertain,
    InitializationBranch::DmsExclusiveNativeContendedPath,
    InitializationBranch::DmsExclusiveNativeIo,
    InitializationBranch::DmsExclusiveNativeUnsupported,
    InitializationBranch::DmsExclusiveCleanupCloseFailure,
    InitializationBranch::DmsTruncateFaultBefore,
    InitializationBranch::DmsTruncateFaultAfterKnown,
    InitializationBranch::DmsTruncateFaultAfterUncertain,
    InitializationBranch::DmsTruncateNativeReleaseSucceeded,
    InitializationBranch::DmsTruncateNativeReleaseFailed,
    InitializationBranch::DmsTruncateCleanupCloseFailure,
    InitializationBranch::DmsExclusiveReleaseFaultBefore,
    InitializationBranch::DmsExclusiveReleaseFaultAfterKnown,
    InitializationBranch::DmsExclusiveReleaseFaultAfterUncertain,
    InitializationBranch::DmsExclusiveReleaseNativeFailure,
    InitializationBranch::DmsSharedFaultBefore,
    InitializationBranch::DmsSharedFaultAfterKnown,
    InitializationBranch::DmsSharedFaultAfterUncertain,
    InitializationBranch::DmsSharedNativeBusy,
    InitializationBranch::DmsSharedNativeIo,
    InitializationBranch::DmsSharedNativeUnsupported,
    InitializationBranch::DmsSharedCleanupCloseFailure,
    InitializationBranch::FirstProcessInitialized,
    InitializationBranch::JoinerInitialized,
    InitializationBranch::NodeMissingAfterOpen,
];

pub(super) const ALL_MAP_BRANCHES: &[MapBranch] = &[
    MapBranch::RegionSizeBudgetRejected,
    MapBranch::LogicalEndBudgetRejected,
    MapBranch::AllocationGranularityIo,
    MapBranch::AllocationGranularityUnsupported,
    MapBranch::AllocationGranularityZero,
    MapBranch::MutexPoisoned,
    MapBranch::ConnectionMissing,
    MapBranch::DomainAlreadyPoisoned,
    MapBranch::PinnedConnectionInactive,
    MapBranch::RegionSizeChanged,
    MapBranch::FileSizeFaultBefore,
    MapBranch::FileSizeFaultAfterKnown,
    MapBranch::FileSizeFaultAfterUncertain,
    MapBranch::FileSizeNativeIo,
    MapBranch::FileSizeNativeUnsupported,
    MapBranch::ExistingSizeBudgetRejected,
    MapBranch::ObserveNotPresent,
    MapBranch::FileGrowFaultBefore,
    MapBranch::FileGrowFaultAfterKnown,
    MapBranch::FileGrowFaultAfterUncertain,
    MapBranch::FileGrowNativeFailure,
    MapBranch::MappingBudgetRejected,
    MapBranch::MappingCreateFaultBefore,
    MapBranch::MappingCreateFaultAfterKnown,
    MapBranch::MappingCreateFaultAfterUncertain,
    MapBranch::MappingCreateNativeIo,
    MapBranch::MappingCreateNativeUnsupported,
    MapBranch::ViewMapFaultBeforeCleanupSucceeded,
    MapBranch::ViewMapFaultBeforeCleanupFailed,
    MapBranch::ViewMapFaultAfterKnown,
    MapBranch::ViewMapFaultAfterUncertain,
    MapBranch::ViewMapNativeIoCleanupSucceeded,
    MapBranch::ViewMapNativeIoCleanupFailed,
    MapBranch::ViewMapNativeUnsupported,
    MapBranch::ViewMapNativeNull,
    MapBranch::MappingCreated,
    MapBranch::MappingReused,
    MapBranch::RegionCustodyMissing,
    MapBranch::ArithmeticOrNodeInvariant,
];

pub(super) const ALL_LOCK_BRANCHES: &[LockBranch] = &[
    LockBranch::CoordinatorPoisoned,
    LockBranch::ConnectionMissing,
    LockBranch::PinnedConnectionInactive,
    LockBranch::TransitionNotUnlocked,
    LockBranch::SharedSiblingExclusiveContention,
    LockBranch::SharedLocalCoalescing,
    LockBranch::ExclusiveSiblingContention,
    LockBranch::SharedUnlockNotHeld,
    LockBranch::SharedLocalRelease,
    LockBranch::ExclusiveUnlockNotHeld,
    LockBranch::ExclusiveRangeMismatch,
    LockBranch::ExclusiveSiblingOverlap,
    LockBranch::AcquireFaultBefore,
    LockBranch::AcquireFaultAfterKnown,
    LockBranch::AcquireFaultAfterUncertain,
    LockBranch::AcquireNativeBusy,
    LockBranch::AcquireNativeIo,
    LockBranch::AcquireNativeUnsupported,
    LockBranch::AcquireNativeSuccess,
    LockBranch::ReleaseFaultBefore,
    LockBranch::ReleaseFaultAfterKnown,
    LockBranch::ReleaseFaultAfterUncertain,
    LockBranch::ReleaseNativeFailure,
    LockBranch::ReleaseNativeSuccess,
    LockBranch::ActionChanged,
    LockBranch::NodeMissing,
    LockBranch::ConnectionDisappearedAfterAction,
    LockBranch::Completed,
];

pub(super) const ALL_FAULT_CONTROLLER_BRANCHES: &[FaultControllerBranch] = &[
    FaultControllerBranch::SelectorPhaseRejected,
    FaultControllerBranch::SelectorOccurrenceRejected,
    FaultControllerBranch::BeforeTriggerFailed,
    FaultControllerBranch::AfterKnownTriggerFailed,
    FaultControllerBranch::AfterUncertainTriggerFailed,
    FaultControllerBranch::PendingSelectorAtCompletion,
];

/// Materializes this review table. Its length is neither a source-universe nor denominator count.
pub(super) fn all_candidate_branch_atoms() -> Vec<RawSourceBranchAtomId> {
    let mut all = Vec::new();
    all.extend(
        ALL_ABI_MAP_BRANCHES
            .iter()
            .copied()
            .map(RawSourceBranchAtomId::AbiMap),
    );
    all.extend(
        ALL_ABI_LOCK_BRANCHES
            .iter()
            .copied()
            .map(RawSourceBranchAtomId::AbiLock),
    );
    for invoker in ALL_INVOKERS.iter().copied() {
        for branch in ALL_CALLBACK_BRANCHES.iter().copied() {
            all.push(RawSourceBranchAtomId::Callback { invoker, branch });
        }
    }
    all.extend(
        ALL_ROUTE_BRIDGE_BRANCHES
            .iter()
            .copied()
            .map(RawSourceBranchAtomId::RouteBridge),
    );
    for invoker in INITIALIZATION_INVOKERS.iter().copied() {
        for branch in ALL_INITIALIZATION_BRANCHES.iter().copied() {
            all.push(RawSourceBranchAtomId::Initialization { invoker, branch });
        }
    }
    for mode in [MapMode::Observe, MapMode::Extend] {
        for branch in ALL_MAP_BRANCHES.iter().copied() {
            all.push(RawSourceBranchAtomId::Map { mode, branch });
        }
    }
    for action in [
        LockAction::LockShared,
        LockAction::LockExclusive,
        LockAction::UnlockShared,
        LockAction::UnlockExclusive,
    ] {
        for branch in ALL_LOCK_BRANCHES.iter().copied() {
            all.push(RawSourceBranchAtomId::Lock { action, branch });
        }
    }
    all.extend(
        ALL_FAULT_CONTROLLER_BRANCHES
            .iter()
            .copied()
            .map(RawSourceBranchAtomId::FaultController),
    );
    all
}
