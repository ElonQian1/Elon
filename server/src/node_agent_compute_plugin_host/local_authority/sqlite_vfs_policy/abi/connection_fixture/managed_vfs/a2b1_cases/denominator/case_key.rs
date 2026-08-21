use std::num::{NonZeroU32, NonZeroU8};

/// Stable operation path. Runtime registration, route, generation and connection identifiers are
/// deliberately absent: they belong to a future dynamic actual record, not this semantic key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Path {
    Map,
    Lock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum PrefixMutation {
    NotReached,
    NoKnownMutation,
    KnownMutation,
}

/// The complete node-initialization trace cannot be represented by a cold/warm bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum InitializationPath {
    NotReached,
    NodeAlreadyLive,
    ExactOpenPending,
    OpenedCreated,
    OpenedExisting,
    DmsExclusiveHeldCreated,
    DmsExclusiveHeldExisting,
    DmsExclusiveOutcomeUncertain,
    DmsReleasedAwaitingShared,
    OpenedFirstProcess,
    OpenedJoiner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapMode {
    Observe,
    Extend,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapInput {
    Canonical,
    InvalidRegion,
    InvalidRegionSize,
    InvalidExtendFlag,
    NullOutput,
    RegionBudget,
    RegionSizeChanged,
    ExistingSizeBudget,
    MappingBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum NodePrecondition {
    NotReached,
    Absent,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RegionPrecondition {
    NotReached,
    MissingShortFile,
    MissingSizedFile,
    Mapped,
}

/// Normalized loop shape: mapping region zero, appending the next canonical region, and filling a
/// canonical gap perform different numbers of mapping/view actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RegionShape {
    NotReached,
    First,
    NextCanonical,
    SkipAheadCanonical { regions_to_create: NonZeroU32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MapOperation {
    pub(super) mode: MapMode,
    pub(super) input: MapInput,
    pub(super) node: NodePrecondition,
    pub(super) region: RegionPrecondition,
    pub(super) region_shape: RegionShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockAction {
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockRange {
    Single,
    ExclusiveMulti,
    InvalidOffset,
    ZeroCount,
    OutOfRange,
    SharedMulti,
    ExclusiveRangeMismatch,
}

/// Exact canonical masks. This preserves multi-slot and mixed sibling relations which a three-state
/// enum would collapse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LockMaskShape {
    pub(super) shared_mask: u8,
    pub(super) exclusive_mask: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LockRequestShape {
    pub(super) first: u8,
    pub(super) count: NonZeroU8,
    pub(super) mask: u8,
}

/// Exact canonical ownership metadata. Equal masks with different exclusive range tables are not
/// equivalent for unlock validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LockOwnershipShape {
    pub(super) masks: LockMaskShape,
    pub(super) exclusive_ranges: [u8; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LockOperation {
    pub(super) action: LockAction,
    pub(super) range: LockRange,
    pub(super) request: LockRequestShape,
    pub(super) own: LockOwnershipShape,
    pub(super) sibling: LockOwnershipShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Operation {
    Map(MapOperation),
    Lock(LockOperation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Phase {
    AbiValidation,
    CallbackAdmission,
    RequestValidation,
    AllocationGranularity,
    ExactSiblingOpen,
    DmsExclusiveAcquire,
    DmsTruncate,
    DmsExclusiveRelease,
    DmsSharedAcquire,
    FileSize,
    FileGrow,
    MappingCreate,
    ViewMap,
    LockAcquire,
    LockRelease,
    MappingClose,
    FileClose,
    CallbackCompletion,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Timing {
    Validation,
    BeforeCall,
    NativeRetryable,
    NativeUncertain,
    AfterSuccessKnown,
    AfterSuccessUncertain,
    LocalDeterministic,
    Succeeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FailureClass {
    None,
    ProtocolViolation,
    RegistryRejected,
    BusyNoMutation,
    BusyAfterKnownMutation,
    NotPresent,
    IoBeforeMutation,
    MutatedButKnown,
    OutcomeUncertainPoisoned,
    PlatformUnsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CleanupCause {
    None,
    ExactOpenHandleClose,
    DmsExclusiveAcquireFileClose,
    DmsTruncateUnlock,
    DmsTruncateFileClose,
    DmsSharedAcquireFileClose,
    ViewMapMappingClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum BranchGroup {
    AbiValidation,
    RegistryCallback,
    ManagedValidation,
    Initialization,
    InjectedFault,
    NativeFailure,
    CleanupRewrite,
    SemanticSuccess,
    LockProtocol,
    LockCoordination,
}

/// Candidate schema only. No value of this type is a StaticContract until the source projection and
/// complete Expected vector are independently reviewed and frozen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CaseKey {
    pub(super) path: Path,
    pub(super) branch_group: BranchGroup,
    pub(super) operation: Operation,
    pub(super) prefix_mutation: PrefixMutation,
    pub(super) initialization_path: InitializationPath,
    pub(super) cause_phase: Phase,
    pub(super) terminal_phase: Phase,
    pub(super) timing: Timing,
    pub(super) failure: FailureClass,
    pub(super) cleanup: CleanupCause,
    pub(super) occurrence: Option<NonZeroU32>,
}
