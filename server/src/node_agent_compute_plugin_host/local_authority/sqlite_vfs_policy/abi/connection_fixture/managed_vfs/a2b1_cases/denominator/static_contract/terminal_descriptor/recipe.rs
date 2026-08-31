#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FixtureV1 {
    NotReached,
    AbiRawOnly,
    ManagedWalMainSingleConnection,
    ManagedWalMainTwoConnections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CallbackV1 {
    NotReached,
    XShmMap,
    XShmLock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FaultSeamV1 {
    NotReached,
    Natural,
    AbiBoundary,
    RawState,
    RegistryAdmission,
    ManagedRequest,
    Initialization,
    NativeOperation,
    CallbackCompletion,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ObserverV1 {
    NotReached,
    MapCallbackAndSnapshot,
    LockCallbackAndSnapshot,
    CustodyAndCleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CleanupV1 {
    NotReached,
    ParentOwnedRoot,
    RetainUnsafeCustodyThenParentCleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CapabilityGapV1 {
    QuotientRunnerNotIntegrated,
    CallbackAfterSuccessUnavailable,
    LockObservationIncomplete,
    TerminalRecipeMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RunnerCapabilityV1 {
    Supported,
    Missing(CapabilityGapV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExecutionRecipeV1 {
    pub(crate) fixture: FixtureV1,
    pub(crate) callback: CallbackV1,
    pub(crate) fault_seam: FaultSeamV1,
    pub(crate) observer: ObserverV1,
    pub(crate) cleanup: CleanupV1,
    pub(crate) capability: RunnerCapabilityV1,
}

impl ExecutionRecipeV1 {
    pub(crate) const fn new(
        fixture: FixtureV1,
        callback: CallbackV1,
        fault_seam: FaultSeamV1,
        observer: ObserverV1,
        cleanup: CleanupV1,
        capability: RunnerCapabilityV1,
    ) -> Self {
        Self {
            fixture,
            callback,
            fault_seam,
            observer,
            cleanup,
            capability,
        }
    }
}
