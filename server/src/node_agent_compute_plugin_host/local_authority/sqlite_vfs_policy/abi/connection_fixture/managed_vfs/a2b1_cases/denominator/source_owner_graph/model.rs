#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum PathOp {
    MapObserve,
    MapExtend,
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
}

pub(super) const MAP_OPS: &[PathOp] = &[PathOp::MapObserve, PathOp::MapExtend];
pub(super) const MAP_EXTEND_OPS: &[PathOp] = &[PathOp::MapExtend];
pub(super) const LOCK_OPS: &[PathOp] = &[
    PathOp::LockShared,
    PathOp::LockExclusive,
    PathOp::UnlockShared,
    PathOp::UnlockExclusive,
];
pub(super) const ACQUIRE_OPS: &[PathOp] = &[PathOp::LockShared, PathOp::LockExclusive];
pub(super) const UNLOCK_OPS: &[PathOp] = &[PathOp::UnlockShared, PathOp::UnlockExclusive];
pub(super) const LOCK_SHARED_OPS: &[PathOp] = &[PathOp::LockShared];
pub(super) const LOCK_EXCLUSIVE_OPS: &[PathOp] = &[PathOp::LockExclusive];
pub(super) const UNLOCK_SHARED_OPS: &[PathOp] = &[PathOp::UnlockShared];
pub(super) const UNLOCK_EXCLUSIVE_OPS: &[PathOp] = &[PathOp::UnlockExclusive];
pub(super) const SHARED_LOCK_OPS: &[PathOp] = &[PathOp::LockShared, PathOp::UnlockShared];
pub(super) const EXCLUSIVE_LOCK_OPS: &[PathOp] = &[PathOp::LockExclusive, PathOp::UnlockExclusive];
pub(super) const INITIALIZING_OPS: &[PathOp] = &[
    PathOp::MapObserve,
    PathOp::MapExtend,
    PathOp::LockShared,
    PathOp::LockExclusive,
];
pub(super) const ALL_OPS: &[PathOp] = &[
    PathOp::MapObserve,
    PathOp::MapExtend,
    PathOp::LockShared,
    PathOp::LockExclusive,
    PathOp::UnlockShared,
    PathOp::UnlockExclusive,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Epoch {
    AbiInput,
    MapRoutePreparation,
    FirstMapBootstrap,
    WalMainSteady,
    ColdNodeAcquirePrefix,
    PlatformBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum NodeRole {
    Entry,
    Adapter,
    RawStateGate,
    FaultWrapper,
    RoutePromotion,
    CallbackOwner,
    CustodyAdapter,
    ManagedValidation,
    Initialization,
    ManagedOperation,
    FaultController,
    PlatformSeam,
    CleanupOwner,
    StateWitness,
    AbiProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Boundary {
    Expanded,
    TypedOutcomeSeam,
    PendingExpansion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum EdgeKind {
    Call,
    ConditionalCall,
    Continuation,
    TerminalReturn,
    CleanupRewrite,
    Quarantine,
    Abandon,
    StatePrerequisite,
    CallbackCompletion,
    ErrorPrecedence,
    MutationBeforeContinuation,
    LoopBack,
    ResultProjection,
    UnwindRetention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Reachability {
    Required,
    Conditional,
    DefensivePending,
    ScopePending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SourceEffect {
    None,
    OutputNull,
    OutputPointer,
    CallbackLease,
    CustodyMutation,
    LocalMaskMutation,
    PlatformMutation,
    Poison,
    RetainCustody,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum StateWitness {
    WalMainPromotedNodeAbsentAfterEarlyMapReturn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SourceOwnerId {
    SqliteVfsAbiTable,
    AbiBoundary,
    AbiIoShm,
    AbiFileState,
    AbiRawState,
    AbiResultCodes,
    FixtureFaultFile,
    FixtureFaultController,
    FixtureRouteFile,
    FixtureFaultPlan,
    RegistryTestBridge,
    RegistryAbiFile,
    RegistryPromotion,
    RegistryOperations,
    RegistryFileCustody,
    RegistryFileFaults,
    RegistryProcessOwner,
    RegistryProcessLifecycle,
    RegistryOwner,
    RegistryOwnerLifecycle,
    RegistryState,
    ManagedNamespace,
    ManagedFsRoot,
    ManagedWindowsPlatform,
    ManagedShmRoot,
    ManagedCoordinator,
    ManagedTypes,
    ManagedInitialization,
    ManagedFailureCustody,
    ManagedMapping,
    ManagedLocking,
    ManagedFaultApi,
    ManagedFaultController,
    ManagedFaultOperation,
    ManagedFaultMapping,
    ManagedNamespaceIo,
    ManagedNamespaceClose,
    WindowsShm,
    WindowsLocking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SourceNodeId {
    AbiMapSlot,
    AbiMapEntry,
    AbiMapNullOutput,
    AbiMapValidation,
    AbiLockSlot,
    AbiLockEntry,
    AbiLockValidation,
    AbiRawRun,
    AbiMapRawGate,
    AbiLockRawGate,
    AbiRawAbandon,
    AbiMapRawStateAbandon,
    AbiLockRawStateAbandon,
    FileStateMap,
    FileStateLock,
    FixtureMapFault,
    FixtureLockFault,
    FixtureFaultController,
    RouteMapPreparation,
    RoutePlanClaim,
    RoutePromotionDelegate,
    RouteNoPlanPromotionDelegate,
    PromotionCallbackBegin,
    PromotionProcessBegin,
    PromotionOwnerBegin,
    PromotionStateBegin,
    PromotionCallbackComplete,
    PromotionProcessComplete,
    PromotionOwnerFinish,
    PromotionStateFinish,
    RegistryPromotionAdapter,
    RegistryPromotionOwner,
    PromotionClaimShmProcess,
    PromotionClaimShmApplyRoute,
    PromotionClaimShmOwner,
    PromotionClaimShmState,
    RuntimeBindMain,
    CoordinatorAttach,
    RegistryAbiFaultInstall,
    ManagedFaultInstall,
    ManagedFaultControllerInstall,
    RegistryFaultInstall,
    RoutePlanRecord,
    RouteMapDelegate,
    RouteLockDelegate,
    TestBridgeMap,
    TestBridgeLock,
    RegistryMapAdapter,
    RegistryLockAdapter,
    ManagedLockRequest,
    RegistryMapOperation,
    RegistryLockOperation,
    RegistryCallbackBegin,
    RegistryProcessBegin,
    RegistryOwnerBegin,
    RegistryStateBegin,
    RegistryShmCustodyGate,
    RegistryUnsafeRetention,
    RegistryRetainTerminal,
    RegistryQuarantineApplyRoute,
    RegistryOwnerQuarantine,
    RegistryStateQuarantine,
    RegistryCallbackComplete,
    RegistryProcessComplete,
    RegistryOwnerFinish,
    RegistryStateFinish,
    RegistryCallbackUnwind,
    RegistryPinnedDrop,
    ManagedConnectionMap,
    ManagedConnectionLock,
    ManagedShmPlatformModuleSelect,
    ManagedPlatformModuleSelect,
    ManagedWindowsLockingExport,
    ManagedMapCoordinator,
    ManagedLockCoordinator,
    ManagedLockLocalGate,
    ManagedLockAcquireGate,
    ManagedLockSharedReleaseGate,
    ManagedLockExclusiveReleaseGate,
    ManagedLockAcquire,
    ManagedLockRelease,
    ManagedEnsureNode,
    ManagedOpenNode,
    ManagedOpenShm,
    ManagedOpenExact,
    ManagedPinnedClose,
    ManagedConsumeOpenFailure,
    ManagedRetainFailureHandleCustody,
    ManagedOpenCleanup,
    ManagedDmsInitialization,
    ManagedFaultBegin,
    ManagedFaultObserve,
    ManagedFaultControllerObserve,
    ManagedFaultTriggerBefore,
    ManagedFaultActivateBefore,
    ManagedFaultFinish,
    ManagedFaultActivateAfter,
    ManagedFaultTriggerAfter,
    ManagedFaultTerminalize,
    ManagedRegionSizeValidation,
    ManagedLogicalEndValidation,
    ManagedExistingSizeValidation,
    ManagedMappedTotalValidation,
    ManagedFileSize,
    ManagedFileGrow,
    ManagedRegionLoop,
    ManagedRegionSelect,
    ManagedMappingCleanup,
    ManagedNativeMappingCleanup,
    ManagedMappingCustodyRetain,
    ManagedInlineRegionCustody,
    ManagedPoison,
    WindowsAllocationGranularity,
    WindowsCreateMapping,
    WindowsMapView,
    WindowsByteLock,
    WindowsByteUnlock,
    ManagedLockLocalSharedMasks,
    ManagedLockPlatformSharedMasks,
    ManagedLockPlatformExclusiveMasks,
    ManagedLockExclusiveRanges,
    WalMainColdNodeWitness,
    RegistryMapProjection,
    AbiMapFallbackProjection,
    RegistryLockProjection,
    RegistryLockBusyProjection,
    AbiMapProjection,
    AbiMapUnavailableCode,
    AbiLockProjection,
    AbiLockBusyProjection,
    AbiLockFallbackProjection,
    AbiLockUnavailableCode,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct OwnerSnapshot {
    pub(super) id: SourceOwnerId,
    pub(super) path: &'static str,
    pub(super) blob_oid: &'static str,
    pub(super) normalized_sha256: &'static str,
    pub(super) symbols: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SourceNode {
    pub(super) id: SourceNodeId,
    pub(super) owner: SourceOwnerId,
    pub(super) symbol: &'static str,
    pub(super) role: NodeRole,
    pub(super) ops: &'static [PathOp],
    pub(super) epoch: Epoch,
    pub(super) boundary: Boundary,
    pub(super) state_witness: Option<StateWitness>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SourceEdge {
    pub(super) id: &'static str,
    pub(super) from: SourceNodeId,
    pub(super) to: SourceNodeId,
    pub(super) kind: EdgeKind,
    pub(super) ops: &'static [PathOp],
    pub(super) epoch: Epoch,
    pub(super) reachability: Reachability,
    pub(super) effect: SourceEffect,
}
