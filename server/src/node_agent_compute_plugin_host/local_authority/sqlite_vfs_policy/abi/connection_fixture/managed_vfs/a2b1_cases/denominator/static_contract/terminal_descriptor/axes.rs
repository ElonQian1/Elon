#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ReachabilityV1<T> {
    NotReached,
    Reached(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PresenceV1 {
    Absent,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ValidityV1 {
    Invalid,
    Valid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SourceSiteV1 {
    MapAbiBoundary,
    LockAbiBoundary,
    RawStateAdmission,
    RawStateAbandon,
    AdapterDispatch,
    RegistryCallbackAdmission,
    ManagedRequestValidation,
    InitializationOpen,
    InitializationDms,
    CoordinatorState,
    MapFileSize,
    MapFileGrow,
    MapMappingCreate,
    MapViewMap,
    MapMappingClose,
    LockLocalState,
    LockNativeAcquire,
    LockNativeRelease,
    FailureCustody,
    CallbackCompletion,
    Quarantine,
    AbiProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PhaseV1 {
    AbiValidation,
    RawAdmission,
    Adapter,
    CallbackAdmission,
    RequestValidation,
    Gate,
    ExactSiblingOpen,
    ExactSiblingDelete,
    DmsExclusiveAcquire,
    DmsTruncate,
    DmsExclusiveRelease,
    DmsSharedAcquire,
    DmsSharedRelease,
    FileClose,
    FileSize,
    FileGrow,
    MappingCreate,
    ViewMap,
    MappingClose,
    ViewUnmap,
    ConnectionDetach,
    DeleteAuthorization,
    LockAcquire,
    LockRelease,
    CallbackCompletion,
    Success,
}

impl PhaseV1 {
    pub(crate) const fn static_name(self) -> &'static str {
        match self {
            Self::AbiValidation => "AbiValidation",
            Self::RawAdmission => "RawAdmission",
            Self::Adapter => "Adapter",
            Self::CallbackAdmission => "CallbackAdmission",
            Self::RequestValidation => "RequestValidation",
            Self::Gate => "Gate",
            Self::ExactSiblingOpen => "ExactSiblingOpen",
            Self::ExactSiblingDelete => "ExactSiblingDelete",
            Self::DmsExclusiveAcquire => "DmsExclusiveAcquire",
            Self::DmsTruncate => "DmsTruncate",
            Self::DmsExclusiveRelease => "DmsExclusiveRelease",
            Self::DmsSharedAcquire => "DmsSharedAcquire",
            Self::DmsSharedRelease => "DmsSharedRelease",
            Self::FileClose => "FileClose",
            Self::FileSize => "FileSize",
            Self::FileGrow => "FileGrow",
            Self::MappingCreate => "MappingCreate",
            Self::ViewMap => "ViewMap",
            Self::MappingClose => "MappingClose",
            Self::ViewUnmap => "ViewUnmap",
            Self::ConnectionDetach => "ConnectionDetach",
            Self::DeleteAuthorization => "DeleteAuthorization",
            Self::LockAcquire => "LockAcquire",
            Self::LockRelease => "LockRelease",
            Self::CallbackCompletion => "CallbackCompletion",
            Self::Success => "Success",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TimingV1 {
    NotReached,
    Natural,
    BeforeCall,
    AtCall,
    AfterSuccess,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OccurrenceV1 {
    NotReached,
    Natural,
    Exact(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapOperationV1 {
    AbiValidation,
    RawAdmission,
    RawAbandon,
    AdapterDispatch,
    CallbackAdmission,
    ManagedRequest,
    Initialization,
    FileSize,
    FileGrow,
    MappingCreate,
    ViewMap,
    MappingClose,
    SuccessProjection,
    CallbackCompletion,
    Quarantine,
    AbiProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LockOperationV1 {
    AbiValidation,
    RawAdmission,
    RawAbandon,
    AdapterDispatch,
    CallbackAdmission,
    ManagedRequest,
    Initialization,
    LocalAcquire,
    LocalRelease,
    NativeAcquire,
    NativeRelease,
    CallbackCompletion,
    Quarantine,
    AbiProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MapAbiScalarV1 {
    pub(crate) output: PresenceV1,
    pub(crate) region: ValidityV1,
    pub(crate) region_size: ValidityV1,
    pub(crate) extend: ValidityV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LockAbiScalarV1 {
    pub(crate) offset: ValidityV1,
    pub(crate) count: ValidityV1,
    pub(crate) flags: ValidityV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RawStateV1 {
    NullFile,
    Uninstalled,
    MethodsNullStatePresent,
    ForeignMethodsStateNull,
    ForeignMethodsStatePresent,
    ExactMethodsStateNull,
    OtherTypePayloadMissing,
    OtherTypePayloadPresent,
    ExpectedTypePayloadMissing,
    HandleBoundFileMissing,
    DropCompleted,
    DropUnwindCaught,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapManagedStimulusV1 {
    CallbackRouteUnknownPriorQuarantine,
    CallbackCounterOverflow,
    CallbackUnsupportedFileRole,
    CallbackShmDetached,
    RegionSizeBudget,
    RegionCountBudget,
    LogicalSizeBudget,
    AllocationGranularity,
    StoredPoison,
    Initialization,
    RegionSize,
    FileSize,
    FileGrow,
    MappingCreate,
    ViewMap,
    MappingClose,
    RegionLoop,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LockManagedStimulusV1 {
    Callback,
    AdmissionRouteUnknown,
    AdmissionCounterOverflow,
    RangeOverflow,
    EndPastEight,
    SharedMultiSlot,
    UnsupportedFileRole,
    ShmDetached,
    StoredPoison,
    Initialization,
    LocalState,
    NativeAcquire,
    NativeRelease,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InitializationFaultSiteV1 {
    ParentValidationBeforeOpen,
    ParentHandle,
    PlatformOpen,
    OpenCompletionValidation,
    OpenFileValidation,
    ParentValidationAfterOpen,
    DmsExclusiveAcquire,
    DmsTruncate,
    DmsExclusiveRelease,
    DmsSharedAcquire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InitializationPathV1 {
    NotReached,
    Opening,
    Created,
    Existing,
    CreatedFirst,
    ExistingFirst,
    CreatedJoiner,
    ExistingJoiner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InitializationStimulusV1 {
    pub(crate) fault_site: InitializationFaultSiteV1,
    pub(crate) path: InitializationPathV1,
    pub(crate) cleanup_rewrite: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StimulusV1 {
    MapAbi(MapAbiScalarV1),
    LockAbi(LockAbiScalarV1),
    MapRaw(RawStateV1),
    LockRaw(RawStateV1),
    MapManaged(MapManagedStimulusV1),
    LockManaged(LockManagedStimulusV1),
    Initialization(InitializationStimulusV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapPrestateV1 {
    NotReached,
    NodeAbsent,
    RegionsEmpty,
    TargetMissing,
    TargetMapped,
    StoredPoison(MapStoredPoisonPrestateV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapStoredPoisonPrestateV1 {
    NoNode,
    LiveNodeRegionsEmpty,
    LiveNodeCompleteRegions,
    NodeAbsentFileQuarantinedNoRegions,
    NodeAbsentFileQuarantinedRegionsReleased,
    NodeAbsentFileReleasedNoRegions,
    NodeAbsentFileAndRegionsReleased,
    LiveNodeMappingOnlyViewNotCreated,
    LiveNodeMappingOnlyViewReleased,
    LiveNodeMappingOnlyWithRetainedView,
    LiveNodeViewUnmapPartialRetained,
    LiveNodeRegionsReleased,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LockPrestateV1 {
    NotReached,
    NoHeldLocks,
    OwnOverlap,
    OwnSharedHeld,
    OwnExclusiveHeld,
    ExclusiveRangeMismatch,
    SiblingExclusiveContention,
    SiblingAnyContention,
    SiblingSharedCoalesced,
    StoredPoison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PrestateV1 {
    Map(MapPrestateV1),
    Lock(LockPrestateV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapModeV1 {
    Observe,
    Extend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InitializationProfileV1 {
    NodeLive,
    CreatedFirstShared,
    CreatedJoinerShared,
    ExistingFirstShared,
    ExistingJoinerShared,
}

impl InitializationProfileV1 {
    pub(crate) const fn static_label(self) -> &'static str {
        match self {
            Self::NodeLive => "node-live",
            Self::CreatedFirstShared => "created-first-shared",
            Self::CreatedJoinerShared => "created-joiner-shared",
            Self::ExistingFirstShared => "existing-first-shared",
            Self::ExistingJoinerShared => "existing-joiner-shared",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapRegionPrestateV1 {
    Empty,
    NonemptyTargetMissing,
    Reuse,
    ObserveNotPresent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapRegionSizeArmV1 {
    NotReached,
    Changed,
    Same,
    UnsetAssigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapFilePathV1 {
    NotReached,
    GrowAttempted,
    ObserveNotPresent,
    SizeSufficient,
    GrowSucceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MapProfileV1 {
    pub(crate) mode: MapModeV1,
    pub(crate) initialization: InitializationProfileV1,
    pub(crate) prestate: MapRegionPrestateV1,
    pub(crate) region_size_arm: MapRegionSizeArmV1,
    pub(crate) file_path: MapFilePathV1,
    pub(crate) prior_mutation: bool,
    pub(crate) preexisting_mapping: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MapAxesV1 {
    pub(crate) mode: ReachabilityV1<MapModeV1>,
    pub(crate) profile: ReachabilityV1<MapProfileV1>,
    pub(crate) ordinal: ReachabilityV1<u16>,
    pub(crate) regions_to_create: ReachabilityV1<u16>,
    pub(crate) completion: ReachabilityV1<MapCompletionV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapCompletionV1 {
    Direct,
    Completed,
    RouteUnknown,
    UnsafeRetentionSucceededThenRouteUnknown,
    UnsafeRetentionRouteUnknownThenRouteUnknown,
    RawDropCompleted,
    RawDropUnwindCaught,
}

impl MapAxesV1 {
    pub(crate) const NOT_REACHED: Self = Self {
        mode: ReachabilityV1::NotReached,
        profile: ReachabilityV1::NotReached,
        ordinal: ReachabilityV1::NotReached,
        regions_to_create: ReachabilityV1::NotReached,
        completion: ReachabilityV1::NotReached,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LockActionV1 {
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LockCompletionV1 {
    Direct,
    Completed,
    RouteUnknown,
    UnsafeRetentionSucceededThenRouteUnknown,
    UnsafeRetentionRouteUnknownThenRouteUnknown,
    RawDropCompleted,
    RawDropUnwindCaught,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LockAxesV1 {
    pub(crate) action: ReachabilityV1<LockActionV1>,
    pub(crate) first: ReachabilityV1<u8>,
    pub(crate) count: ReachabilityV1<u8>,
    pub(crate) mask: ReachabilityV1<u8>,
    pub(crate) initialization: ReachabilityV1<InitializationProfileV1>,
    pub(crate) held_shared_mask: ReachabilityV1<u8>,
    pub(crate) held_exclusive_mask: ReachabilityV1<u8>,
    pub(crate) sibling_shared_mask: ReachabilityV1<u8>,
    pub(crate) sibling_exclusive_mask: ReachabilityV1<u8>,
    pub(crate) completion: ReachabilityV1<LockCompletionV1>,
}

impl LockAxesV1 {
    pub(crate) const NOT_REACHED: Self = Self {
        action: ReachabilityV1::NotReached,
        first: ReachabilityV1::NotReached,
        count: ReachabilityV1::NotReached,
        mask: ReachabilityV1::NotReached,
        initialization: ReachabilityV1::NotReached,
        held_shared_mask: ReachabilityV1::NotReached,
        held_exclusive_mask: ReachabilityV1::NotReached,
        sibling_shared_mask: ReachabilityV1::NotReached,
        sibling_exclusive_mask: ReachabilityV1::NotReached,
        completion: ReachabilityV1::NotReached,
    };
}
