//! Typed Lock terminal semantics produced beside the source graph.
//!
//! The constructors in this module accept only typed source facts. They intentionally cannot
//! inspect a leaf id, node id, branch label, debug value, or source display string.

use super::{
    super::terminal_descriptor::{
        CallbackV1, CapabilityGapV1, CleanupV1, ExecutionRecipeV1, FaultSeamV1, FixtureV1,
        InitializationFaultSiteV1, InitializationProfileV1, InitializationStimulusV1, LockActionV1,
        LockAxesV1, LockCompletionV1, LockManagedStimulusV1, LockOperationV1, LockPrestateV1,
        ObserverV1, OccurrenceV1, PhaseV1, PrestateV1, ReachabilityV1, RunnerCapabilityV1,
        SourceSiteV1, StimulusV1, TerminalDescriptorV1, TimingV1,
    },
    range::{Action, RangeCell},
};

#[derive(Debug, Clone, Copy)]
pub(super) struct SeedV1 {
    source_site: SourceSiteV1,
    stimulus: StimulusV1,
    prestate: LockPrestateV1,
    operation: LockOperationV1,
    phase: PhaseV1,
    timing: TimingV1,
    occurrence: OccurrenceV1,
    fixture: FixtureV1,
    callback: CallbackV1,
    fault_seam: FaultSeamV1,
    observer: ObserverV1,
    axes: LockAxesV1,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TerminalPathV1 {
    Direct,
    Completed,
    RouteUnknown,
    UnsafeRetentionSucceededThenRouteUnknown,
    UnsafeRetentionRouteUnknownThenRouteUnknown,
    RawDropCompleted,
    RawDropUnwindCaught,
}

impl TerminalPathV1 {
    const fn completion(self) -> LockCompletionV1 {
        match self {
            Self::Direct => LockCompletionV1::Direct,
            Self::Completed => LockCompletionV1::Completed,
            Self::RouteUnknown => LockCompletionV1::RouteUnknown,
            Self::UnsafeRetentionSucceededThenRouteUnknown => {
                LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown
            }
            Self::UnsafeRetentionRouteUnknownThenRouteUnknown => {
                LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown
            }
            Self::RawDropCompleted => LockCompletionV1::RawDropCompleted,
            Self::RawDropUnwindCaught => LockCompletionV1::RawDropUnwindCaught,
        }
    }

    const fn cleanup(self) -> CleanupV1 {
        match self {
            Self::UnsafeRetentionSucceededThenRouteUnknown
            | Self::UnsafeRetentionRouteUnknownThenRouteUnknown => {
                CleanupV1::RetainUnsafeCustodyThenParentCleanup
            }
            Self::Direct
            | Self::Completed
            | Self::RouteUnknown
            | Self::RawDropCompleted
            | Self::RawDropUnwindCaught => CleanupV1::ParentOwnedRoot,
        }
    }
}

impl SeedV1 {
    #[allow(clippy::too_many_arguments)]
    pub(super) const fn early(
        source_site: SourceSiteV1,
        stimulus: StimulusV1,
        operation: LockOperationV1,
        phase: PhaseV1,
        timing: TimingV1,
        occurrence: OccurrenceV1,
        fault_seam: FaultSeamV1,
        observer: ObserverV1,
    ) -> Self {
        Self {
            source_site,
            stimulus,
            prestate: LockPrestateV1::NotReached,
            operation,
            phase,
            timing,
            occurrence,
            fixture: FixtureV1::AbiRawOnly,
            callback: CallbackV1::XShmLock,
            fault_seam,
            observer,
            axes: LockAxesV1::NOT_REACHED,
        }
    }

    pub(super) const fn request_rejection(action: Action, stimulus: LockManagedStimulusV1) -> Self {
        Self {
            source_site: SourceSiteV1::ManagedRequestValidation,
            stimulus: StimulusV1::LockManaged(stimulus),
            prestate: LockPrestateV1::NotReached,
            operation: LockOperationV1::ManagedRequest,
            phase: PhaseV1::RequestValidation,
            timing: TimingV1::BeforeCall,
            occurrence: OccurrenceV1::Natural,
            fixture: FixtureV1::ManagedWalMainSingleConnection,
            callback: CallbackV1::XShmLock,
            fault_seam: FaultSeamV1::ManagedRequest,
            observer: ObserverV1::LockCallbackAndSnapshot,
            axes: request_axes(action),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn managed(
        action: Action,
        range: RangeCell,
        source_site: SourceSiteV1,
        stimulus: LockManagedStimulusV1,
        prestate: LockPrestateV1,
        operation: LockOperationV1,
        phase: PhaseV1,
        timing: TimingV1,
        fault_seam: FaultSeamV1,
    ) -> Self {
        Self {
            source_site,
            stimulus: StimulusV1::LockManaged(stimulus),
            prestate,
            operation,
            phase,
            timing,
            occurrence: OccurrenceV1::Natural,
            fixture: fixture(prestate),
            callback: CallbackV1::XShmLock,
            fault_seam,
            observer: ObserverV1::LockCallbackAndSnapshot,
            axes: managed_axes(action, range, prestate),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn initialization_failure(
        action: Action,
        range: RangeCell,
        stimulus: InitializationStimulusV1,
        phase: PhaseV1,
        timing: TimingV1,
        occurrence: OccurrenceV1,
    ) -> Self {
        Self {
            source_site: initialization_source(stimulus.fault_site),
            stimulus: StimulusV1::Initialization(stimulus),
            prestate: LockPrestateV1::NoHeldLocks,
            operation: LockOperationV1::Initialization,
            phase,
            timing,
            occurrence,
            fixture: FixtureV1::ManagedWalMainSingleConnection,
            callback: CallbackV1::XShmLock,
            fault_seam: FaultSeamV1::Initialization,
            observer: ObserverV1::LockCallbackAndSnapshot,
            axes: managed_axes(action, range, LockPrestateV1::NoHeldLocks),
        }
    }

    pub(super) const fn with_initialization(mut self, profile: InitializationProfileV1) -> Self {
        self.axes.initialization = ReachabilityV1::Reached(profile);
        self
    }

    pub(super) const fn terminal(mut self, path: TerminalPathV1) -> TerminalDescriptorV1 {
        self.axes.completion = ReachabilityV1::Reached(path.completion());
        TerminalDescriptorV1::lock(
            self.source_site,
            self.stimulus,
            PrestateV1::Lock(self.prestate),
            self.operation,
            self.phase,
            self.timing,
            self.occurrence,
            ExecutionRecipeV1::new(
                self.fixture,
                self.callback,
                self.fault_seam,
                self.observer,
                path.cleanup(),
                RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
            ),
            self.axes,
        )
    }
}

const fn action(action: Action) -> LockActionV1 {
    match action {
        Action::LockShared => LockActionV1::LockShared,
        Action::LockExclusive => LockActionV1::LockExclusive,
        Action::UnlockShared => LockActionV1::UnlockShared,
        Action::UnlockExclusive => LockActionV1::UnlockExclusive,
    }
}

const fn request_axes(action_value: Action) -> LockAxesV1 {
    LockAxesV1 {
        action: ReachabilityV1::Reached(action(action_value)),
        ..LockAxesV1::NOT_REACHED
    }
}

fn managed_axes(action_value: Action, range: RangeCell, prestate: LockPrestateV1) -> LockAxesV1 {
    let mask = range.mask();
    let mut axes = LockAxesV1 {
        action: ReachabilityV1::Reached(action(action_value)),
        first: ReachabilityV1::Reached(range.first),
        count: ReachabilityV1::Reached(range.count),
        mask: ReachabilityV1::Reached(mask),
        ..LockAxesV1::NOT_REACHED
    };
    let reached = |value| ReachabilityV1::Reached(value);
    match prestate {
        LockPrestateV1::NotReached | LockPrestateV1::StoredPoison => {}
        LockPrestateV1::NoHeldLocks => {
            axes.held_shared_mask = reached(0);
            axes.held_exclusive_mask = reached(0);
            axes.sibling_shared_mask = reached(0);
            axes.sibling_exclusive_mask = reached(0);
        }
        LockPrestateV1::OwnOverlap => {
            axes.held_shared_mask = reached(if action_value.is_shared() { mask } else { 0 });
            axes.held_exclusive_mask = reached(if action_value.is_shared() { 0 } else { mask });
            axes.sibling_shared_mask = reached(0);
            axes.sibling_exclusive_mask = reached(0);
        }
        LockPrestateV1::OwnSharedHeld => {
            axes.held_shared_mask = reached(mask);
            axes.held_exclusive_mask = reached(0);
            axes.sibling_shared_mask = reached(0);
            axes.sibling_exclusive_mask = reached(0);
        }
        LockPrestateV1::OwnExclusiveHeld | LockPrestateV1::ExclusiveRangeMismatch => {
            axes.held_shared_mask = reached(0);
            axes.held_exclusive_mask = reached(mask);
            axes.sibling_shared_mask = reached(0);
            axes.sibling_exclusive_mask = reached(0);
        }
        LockPrestateV1::SiblingExclusiveContention => {
            axes.held_shared_mask = reached(0);
            axes.held_exclusive_mask = reached(0);
            axes.sibling_shared_mask = reached(0);
            axes.sibling_exclusive_mask = reached(mask);
        }
        LockPrestateV1::SiblingAnyContention => {
            axes.held_shared_mask = reached(0);
            axes.held_exclusive_mask = reached(0);
            axes.sibling_shared_mask = reached(mask);
            axes.sibling_exclusive_mask = reached(0);
        }
        LockPrestateV1::SiblingSharedCoalesced => {
            axes.held_shared_mask = reached(if matches!(action_value, Action::UnlockShared) {
                mask
            } else {
                0
            });
            axes.held_exclusive_mask = reached(0);
            axes.sibling_shared_mask = reached(mask);
            axes.sibling_exclusive_mask = reached(0);
        }
    }
    axes
}

const fn fixture(prestate: LockPrestateV1) -> FixtureV1 {
    match prestate {
        LockPrestateV1::SiblingExclusiveContention
        | LockPrestateV1::SiblingAnyContention
        | LockPrestateV1::SiblingSharedCoalesced => FixtureV1::ManagedWalMainTwoConnections,
        LockPrestateV1::NotReached
        | LockPrestateV1::NoHeldLocks
        | LockPrestateV1::OwnOverlap
        | LockPrestateV1::OwnSharedHeld
        | LockPrestateV1::OwnExclusiveHeld
        | LockPrestateV1::ExclusiveRangeMismatch
        | LockPrestateV1::StoredPoison => FixtureV1::ManagedWalMainSingleConnection,
    }
}

const fn initialization_source(fault_site: InitializationFaultSiteV1) -> SourceSiteV1 {
    match fault_site {
        InitializationFaultSiteV1::DmsExclusiveAcquire
        | InitializationFaultSiteV1::DmsTruncate
        | InitializationFaultSiteV1::DmsExclusiveRelease
        | InitializationFaultSiteV1::DmsSharedAcquire => SourceSiteV1::InitializationDms,
        InitializationFaultSiteV1::ParentValidationBeforeOpen
        | InitializationFaultSiteV1::ParentHandle
        | InitializationFaultSiteV1::PlatformOpen
        | InitializationFaultSiteV1::OpenCompletionValidation
        | InitializationFaultSiteV1::OpenFileValidation
        | InitializationFaultSiteV1::ParentValidationAfterOpen => SourceSiteV1::InitializationOpen,
    }
}
