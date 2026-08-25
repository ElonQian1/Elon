use super::super::model::{Epoch, PathOp, SourceEffect, SourceNodeId, SourceOwnerId};

pub(super) const MAP_OBSERVE: &[PathOp] = &[PathOp::MapObserve];
pub(super) const MAP_EXTEND: &[PathOp] = &[PathOp::MapExtend];
pub(super) const MAP_BOTH: &[PathOp] = &[PathOp::MapObserve, PathOp::MapExtend];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapSiteId {
    AbiInput,
    RawState,
    OuterFault,
    RoutePlan,
    Promotion,
    FaultInstall,
    OperationCallback,
    ManagedValidation,
    NodeInitialization,
    FileSize,
    FileGrow,
    MappingCreate,
    ViewMap,
    RegionSelection,
    AbiProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapPhase {
    AbiValidation,
    RawStateGate,
    TypedFileStateGate,
    OuterCallbackFault,
    RoutePreparation,
    PromotionCallbackAdmission,
    PromotionCustody,
    PromotionCallbackCompletion,
    FaultScriptInstall,
    OperationCallbackAdmission,
    RequestValidation,
    ExactSiblingOpen,
    DmsExclusiveAcquire,
    DmsTruncate,
    DmsExclusiveRelease,
    DmsSharedAcquire,
    FileSize,
    FileGrow,
    MappingCreate,
    ViewMap,
    MappingClose,
    FileClose,
    OperationCallbackCompletion,
    AbiProjection,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapTiming {
    Validation,
    BeforeCall,
    NativeResult,
    AfterSuccessKnown,
    AfterSuccessUncertain,
    Cleanup,
    LocalDeterministic,
    CallbackCompletion,
    Succeeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapExit {
    AbiUnavailableNull,
    AbiUnavailableNoSlot,
    AbiOkNotPresent,
    AbiOkMapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapMultiplicity {
    OncePerMapCall,
    LifetimeCallbackOccurrence,
    SymbolicPhaseOccurrence,
    SymbolicRegionLoop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapRetention {
    None,
    ExistingCustody,
    NodeCustody,
    FileCloseCustody,
    MappingCustody,
    ViewAndMappingCustody,
    RegistryMarkerBeforeCompletion,
    PrefixDependent,
    BranchDependent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapPhaseProjection {
    NotWritten,
    Cause,
    Returned,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapExclusionReason {
    OutsideSupportedWindows,
    ExactFixtureInvariant,
    DefensiveCorruption,
    RejectedByControlFlow,
    InvalidSemanticOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapPendingReason {
    AbiInputShapeSplit,
    RawAbandonSubbranch,
    RouteOrPlanPrecondition,
    PromotionCustodyVariant,
    CallbackOwnerVariant,
    PlatformTypedOutcome,
    PrefixMutationSplit,
    SymbolicLoopOrOccurrence,
    DynamicObservableMissing,
    ControllerInternalFailure,
    ManagedStateInvariant,
    PlatformCfgScope,
    SuccessPrestatePartition,
    CallbackLifetimeOccurrence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapValueFlow {
    None,
    AbiNullWriteConditional,
    OutputSlotAbsent,
    TypedPointerCreated,
    TypedPointerCarried,
    AbiPointerWritten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SourceAnchor {
    pub(super) owner: SourceOwnerId,
    pub(super) symbol: &'static str,
    pub(super) needle: &'static str,
    pub(super) occurrence: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MapTerminalTemplate {
    pub(super) cause: MapPhase,
    pub(super) returned_terminal: MapPhase,
    pub(super) stored_poison: MapPhaseProjection,
    pub(super) route_marker: MapPhaseProjection,
    pub(super) timing: MapTiming,
    pub(super) exit: MapExit,
    pub(super) retention: MapRetention,
    pub(super) multiplicity: MapMultiplicity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapStepKind {
    Continuation,
    StructuralJoin,
    Terminal(MapTerminalTemplate),
    CleanupRewrite(MapTerminalTemplate),
    Excluded(MapExclusionReason),
    Pending {
        terminal: Option<MapTerminalTemplate>,
        reason: MapPendingReason,
    },
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapSourceStepId {
    AbiNullFirst,
    AbiInputRejected,
    AbiNullOutputRejected,
    AbiRawDispatch,
    RawStateAccepted,
    RawStateRejectedOrPanicked,
    FileStateInnerMissing,
    RawAbandonEmpty,
    RawAbandonInstalled,
    RawAbandonRejected,
    OuterFaultControllerRejected,
    OuterFaultSelected,
    OuterInnerMissing,
    OuterFaultPass,
    RouteNonMainBypass,
    RoutePlanClaimRejected,
    RoutePlanned,
    RouteNoPlan,
    PromotionAdmissionRejected,
    PromotionAlreadyWalLive,
    PromotionWalDetached,
    PromotionSidecar,
    PromotionClaimShmRejected,
    PromotionBindSucceeded,
    PromotionBindFailedRetained,
    PromotionBindRetentionFailed,
    PromotionOperationErrorWins,
    PromotionCompletionRejected,
    PromotionCompleted,
    FaultInstallRejected,
    FaultProbeRecordRejected,
    FaultProbeRecorded,
    OperationAdmissionRejected,
    OperationUnsupportedRole,
    OperationShmDetached,
    OperationManagedFailure,
    OperationUnsafeRetain,
    OperationErrorWinsCompletion,
    OperationCompletionRejected,
    OperationCompleted,
    AdapterNotPresent,
    AdapterMapped,
    AdapterRegionMismatch,
    AdapterLengthMismatch,
    AdapterNullPointer,
    ManagedInactive,
    RegionSizeBudgetRejected,
    RegionCountBudgetRejected,
    LogicalEndOverflowRejected,
    LogicalEndBudgetRejected,
    AllocationGranularityFailure,
    AllocationGranularityZero,
    CoordinatorMutexPoisoned,
    ConnectionMissing,
    DomainAlreadyPoisoned,
    RegionSizeChanged,
    ExactOpenFaultBefore,
    ExactOpenFaultAfterKnown,
    ExactOpenFaultAfterUncertain,
    ExactOpenNativeFailure,
    ExactOpenCleanupSucceeded,
    ExactOpenCloseRewrite,
    DmsExclusiveFaultBefore,
    DmsExclusiveFaultAfterKnown,
    DmsExclusiveFaultAfterUncertain,
    DmsExclusiveContended,
    DmsExclusiveNativeFailure,
    DmsExclusiveFaultBeforeCloseSucceeded,
    DmsExclusiveFaultBeforeCloseRewrite,
    DmsTruncateFaultBeforeReleaseOk,
    DmsTruncateFaultBeforeReleaseFailed,
    DmsTruncateNativeReleaseOk,
    DmsTruncateNativeReleaseFailed,
    DmsTruncateFaultAfterKnown,
    DmsTruncateFaultAfterUncertain,
    DmsTruncateCloseSucceeded,
    DmsTruncateCloseRewrite,
    DmsExclusiveReleaseFaultBefore,
    DmsExclusiveReleaseNativeFailure,
    DmsExclusiveReleaseFaultAfterKnown,
    DmsExclusiveReleaseFaultAfterUncertain,
    DmsSharedFaultBefore,
    DmsSharedNativeBusy,
    DmsSharedNativeFailure,
    DmsSharedFaultAfterKnown,
    DmsSharedFaultAfterUncertain,
    DmsSharedFaultBeforeCloseSucceeded,
    DmsSharedFaultBeforeCloseRewrite,
    FirstProcessInitialized,
    SharedDmsInitialized,
    NodeMissingAfterOpen,
    FileSizeFaultBefore,
    FileSizeAfterSelectorRejected,
    FileSizeNativeFailure,
    ExistingSizeBudgetRejected,
    ObserveNotPresent,
    FileGrowFaultBefore,
    FileGrowNativeFailure,
    FileGrowFaultAfterKnown,
    FileGrowFaultAfterUncertain,
    MappingArithmeticRejected,
    RegionOffsetOverflowRejected,
    ViewShiftOverflowRejected,
    RegionLengthOverflowRejected,
    ViewLengthOverflowRejected,
    MappedTotalOverflowRejected,
    MappingBudgetRejected,
    MappingCreateFaultBefore,
    MappingCreateNativeFailure,
    MappingCreateRetained,
    MappingCreateFaultAfterKnown,
    MappingCreateFaultAfterUncertain,
    MappingCreateAfterMatchLostExcluded,
    MappingCreateAttempt,
    ViewMapFaultBeforeCleanupOk,
    ViewMapFaultBeforeCleanupFailed,
    ViewMapFaultBeforeUncertain,
    ViewMapNativeCleanupOk,
    ViewMapNativeCleanupFailed,
    ViewMapNullCustodyRetained,
    ViewMapNullPoisoned,
    ViewMapFaultAfterKnown,
    ViewMapFaultAfterUncertain,
    ViewMapSucceeded,
    RegionRecorded,
    RegionLoopContinues,
    RegionReuseCandidate,
    RegionCustodyMissing,
    ManagedMapped,
    AbiFailureProjection,
    AbiNotPresentProjection,
    AbiMappedProjection,
    Count,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MapSourceStep {
    pub(super) id: MapSourceStepId,
    pub(super) site: MapSiteId,
    pub(super) anchor: SourceAnchor,
    pub(super) call_context: Option<SourceAnchor>,
    pub(super) ops: &'static [PathOp],
    pub(super) epoch: Epoch,
    pub(super) effect: SourceEffect,
    pub(super) value_flow: MapValueFlow,
    pub(super) kind: MapStepKind,
}

pub(super) const fn step(
    id: MapSourceStepId,
    site: MapSiteId,
    owner: SourceOwnerId,
    symbol: &'static str,
    needle: &'static str,
    occurrence: u8,
    ops: &'static [PathOp],
    epoch: Epoch,
    effect: SourceEffect,
    kind: MapStepKind,
) -> MapSourceStep {
    MapSourceStep {
        id,
        site,
        anchor: SourceAnchor {
            owner,
            symbol,
            needle,
            occurrence,
        },
        call_context: None,
        ops,
        epoch,
        effect,
        value_flow: MapValueFlow::None,
        kind,
    }
}

pub(super) const fn with_value_flow(
    mut source_step: MapSourceStep,
    value_flow: MapValueFlow,
) -> MapSourceStep {
    source_step.value_flow = value_flow;
    source_step
}

pub(super) const fn source_anchor(
    owner: SourceOwnerId,
    symbol: &'static str,
    needle: &'static str,
    occurrence: u8,
) -> SourceAnchor {
    SourceAnchor {
        owner,
        symbol,
        needle,
        occurrence,
    }
}

pub(super) const fn with_call_context(
    mut source_step: MapSourceStep,
    call_context: SourceAnchor,
) -> MapSourceStep {
    source_step.call_context = Some(call_context);
    source_step
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapReviewGate {
    AbiInputShapeSplit,
    RawRejectionVsPanicSplit,
    RawStateExactFixtureExclusion,
    RouteAndPromotionExactFixtureExclusion,
    TypedPlatformOutcomeExpansion,
    PrefixMutationAndInitializationCrossProduct,
    SymbolicRegionLoopAndFaultOccurrence,
    CallbackAndCustodyProjectionClosure,
    DynamicTerminalRewriteObservation,
    PlatformCfgAndControllerInternals,
    ManagedDefensiveLeafExpansion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MapReviewGateRecord {
    pub(super) gate: MapReviewGate,
    pub(super) witnesses: &'static [MapSourceStepId],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MapOpenReviewBoundary {
    pub(super) gate: MapReviewGate,
    pub(super) anchors: &'static [SourceAnchor],
    pub(super) note: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapSuccessFamily {
    ExtendColdCreate,
    ExtendWarmCreate,
    ExtendReuse,
    ObserveWarmCreate,
    ObserveReuse,
    ObserveNotPresent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MapSuccessFamilyRecord {
    pub(super) family: MapSuccessFamily,
    pub(super) ops: &'static [PathOp],
    pub(super) witnesses: &'static [MapSourceStepId],
    pub(super) exit: MapExit,
    pub(super) unresolved_multiplicity: Option<MapMultiplicity>,
    pub(super) prestate_partition_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapBoundaryReviewStatus {
    AnchoredButGraphPending,
    BudgetOwnerGraphGap,
    FileSizeGrowGraphConflated,
    CrossLedgerStateWitnessPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MapPendingBoundaryRecord {
    pub(super) node: SourceNodeId,
    pub(super) status: MapBoundaryReviewStatus,
    pub(super) witnesses: &'static [MapSourceStepId],
}
