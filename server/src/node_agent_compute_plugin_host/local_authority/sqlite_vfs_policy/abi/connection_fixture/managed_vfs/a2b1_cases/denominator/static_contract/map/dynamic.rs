use super::{
    super::terminal_descriptor::{
        CallbackV1, CapabilityGapV1, CleanupV1, ExecutionRecipeV1, FaultSeamV1, FixtureV1,
        MapAxesV1, MapCompletionV1, MapModeV1, MapOperationV1, MapPrestateV1, MapProfileV1,
        ObserverV1, OccurrenceV1, PhaseV1, PrestateV1, ReachabilityV1, RunnerCapabilityV1,
        SourceSiteV1, StimulusV1, TerminalDescriptorV1, TimingV1,
    },
    MapMode,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct DescriptorSeedV1 {
    source_site: SourceSiteV1,
    stimulus: StimulusV1,
    prestate: MapPrestateV1,
    operation: MapOperationV1,
    phase: PhaseV1,
    timing: TimingV1,
    occurrence: OccurrenceV1,
    fault_seam: FaultSeamV1,
    axes: MapAxesV1,
}

impl DescriptorSeedV1 {
    #[allow(clippy::too_many_arguments)]
    pub(super) const fn new(
        source_site: SourceSiteV1,
        stimulus: StimulusV1,
        prestate: MapPrestateV1,
        operation: MapOperationV1,
        phase: PhaseV1,
        timing: TimingV1,
        occurrence: OccurrenceV1,
        fault_seam: FaultSeamV1,
        axes: MapAxesV1,
    ) -> Self {
        Self {
            source_site,
            stimulus,
            prestate,
            operation,
            phase,
            timing,
            occurrence,
            fault_seam,
            axes,
        }
    }

    pub(super) const fn direct(self) -> TerminalDescriptorV1 {
        self.finish(MapCompletionV1::Direct, false)
    }

    pub(super) const fn after_success_completion(mut self) -> Self {
        self.source_site = SourceSiteV1::CallbackCompletion;
        self.operation = MapOperationV1::CallbackCompletion;
        self.phase = PhaseV1::CallbackCompletion;
        self.timing = TimingV1::AfterSuccess;
        self.fault_seam = FaultSeamV1::CallbackCompletion;
        self
    }

    pub(super) const fn completed(self) -> TerminalDescriptorV1 {
        self.finish(MapCompletionV1::Completed, false)
    }

    pub(super) const fn route_unknown(self) -> TerminalDescriptorV1 {
        self.finish(MapCompletionV1::RouteUnknown, true)
    }

    pub(super) const fn unsafe_retention_succeeded(self) -> TerminalDescriptorV1 {
        self.finish(
            MapCompletionV1::UnsafeRetentionSucceededThenRouteUnknown,
            true,
        )
    }

    pub(super) const fn unsafe_retention_route_unknown(self) -> TerminalDescriptorV1 {
        self.finish(
            MapCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown,
            true,
        )
    }

    pub(super) const fn raw_drop_completed(self) -> TerminalDescriptorV1 {
        self.finish(MapCompletionV1::RawDropCompleted, false)
    }

    pub(super) const fn raw_drop_unwind(self) -> TerminalDescriptorV1 {
        self.finish(MapCompletionV1::RawDropUnwindCaught, true)
    }

    const fn finish(
        mut self,
        completion: MapCompletionV1,
        unsafe_custody: bool,
    ) -> TerminalDescriptorV1 {
        self.axes.completion = ReachabilityV1::Reached(completion);
        let managed = matches!(self.axes.mode, ReachabilityV1::Reached(_));
        TerminalDescriptorV1::map(
            self.source_site,
            self.stimulus,
            PrestateV1::Map(self.prestate),
            self.operation,
            self.phase,
            self.timing,
            self.occurrence,
            ExecutionRecipeV1::new(
                if managed {
                    FixtureV1::ManagedWalMainSingleConnection
                } else {
                    FixtureV1::AbiRawOnly
                },
                CallbackV1::XShmMap,
                self.fault_seam,
                if unsafe_custody {
                    ObserverV1::CustodyAndCleanup
                } else {
                    ObserverV1::MapCallbackAndSnapshot
                },
                if unsafe_custody {
                    CleanupV1::RetainUnsafeCustodyThenParentCleanup
                } else {
                    CleanupV1::ParentOwnedRoot
                },
                RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
            ),
            self.axes,
        )
    }
}

pub(super) const fn mode(mode: MapMode) -> MapModeV1 {
    match mode {
        MapMode::Observe => MapModeV1::Observe,
        MapMode::Extend => MapModeV1::Extend,
    }
}

pub(super) const fn mode_axes(mode: MapMode) -> MapAxesV1 {
    MapAxesV1 {
        mode: ReachabilityV1::Reached(self::mode(mode)),
        ..MapAxesV1::NOT_REACHED
    }
}

pub(super) const fn profile_axes(profile: MapProfileV1) -> MapAxesV1 {
    MapAxesV1 {
        mode: ReachabilityV1::Reached(profile.mode),
        profile: ReachabilityV1::Reached(profile),
        ordinal: ReachabilityV1::NotReached,
        regions_to_create: ReachabilityV1::NotReached,
        completion: ReachabilityV1::NotReached,
    }
}

pub(super) const fn ordinal_axes(profile: MapProfileV1, ordinal: u16) -> MapAxesV1 {
    MapAxesV1 {
        ordinal: ReachabilityV1::Reached(ordinal),
        regions_to_create: ReachabilityV1::Reached(ordinal),
        ..profile_axes(profile)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) const fn managed_seed(
    mode: MapMode,
    source_site: SourceSiteV1,
    stimulus: StimulusV1,
    prestate: MapPrestateV1,
    operation: MapOperationV1,
    phase: PhaseV1,
    timing: TimingV1,
    fault_seam: FaultSeamV1,
) -> DescriptorSeedV1 {
    DescriptorSeedV1::new(
        source_site,
        stimulus,
        prestate,
        operation,
        phase,
        timing,
        OccurrenceV1::Natural,
        fault_seam,
        mode_axes(mode),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) const fn profile_seed(
    profile: MapProfileV1,
    source_site: SourceSiteV1,
    stimulus: StimulusV1,
    prestate: MapPrestateV1,
    operation: MapOperationV1,
    phase: PhaseV1,
    timing: TimingV1,
    fault_seam: FaultSeamV1,
) -> DescriptorSeedV1 {
    DescriptorSeedV1::new(
        source_site,
        stimulus,
        prestate,
        operation,
        phase,
        timing,
        OccurrenceV1::Natural,
        fault_seam,
        profile_axes(profile),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) const fn ordinal_seed(
    profile: MapProfileV1,
    ordinal: u16,
    source_site: SourceSiteV1,
    stimulus: StimulusV1,
    prestate: MapPrestateV1,
    operation: MapOperationV1,
    phase: PhaseV1,
    timing: TimingV1,
    fault_seam: FaultSeamV1,
) -> DescriptorSeedV1 {
    DescriptorSeedV1::new(
        source_site,
        stimulus,
        prestate,
        operation,
        phase,
        timing,
        OccurrenceV1::Exact(ordinal),
        fault_seam,
        ordinal_axes(profile, ordinal),
    )
}
