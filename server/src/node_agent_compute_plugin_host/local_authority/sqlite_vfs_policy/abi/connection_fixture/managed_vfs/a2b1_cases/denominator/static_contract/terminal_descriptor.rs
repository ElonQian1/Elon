//! Typed, identity-free terminal semantics for the Map/Lock dynamic quotient.
//!
//! These values are built beside graph terminals. They never enter the frozen static record or
//! its digest, and they never recover semantics from a leaf id, node id, branch label or test name.

mod axes;
mod recipe;

pub(crate) use axes::*;
pub(crate) use recipe::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TerminalDescriptorV1 {
    Map(MapTerminalDescriptorV1),
    Lock(LockTerminalDescriptorV1),
}

pub(crate) type TypedTerminalDescriptorV1 = TerminalDescriptorV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MapTerminalDescriptorV1 {
    pub(crate) source_site: SourceSiteV1,
    pub(crate) stimulus: StimulusV1,
    pub(crate) prestate: PrestateV1,
    pub(crate) operation: MapOperationV1,
    pub(crate) phase: PhaseV1,
    pub(crate) timing: TimingV1,
    pub(crate) occurrence: OccurrenceV1,
    pub(crate) recipe: ExecutionRecipeV1,
    pub(crate) axes: MapAxesV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LockTerminalDescriptorV1 {
    pub(crate) source_site: SourceSiteV1,
    pub(crate) stimulus: StimulusV1,
    pub(crate) prestate: PrestateV1,
    pub(crate) operation: LockOperationV1,
    pub(crate) phase: PhaseV1,
    pub(crate) timing: TimingV1,
    pub(crate) occurrence: OccurrenceV1,
    pub(crate) recipe: ExecutionRecipeV1,
    pub(crate) axes: LockAxesV1,
}

impl TerminalDescriptorV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn map(
        source_site: SourceSiteV1,
        stimulus: StimulusV1,
        prestate: PrestateV1,
        operation: MapOperationV1,
        phase: PhaseV1,
        timing: TimingV1,
        occurrence: OccurrenceV1,
        recipe: ExecutionRecipeV1,
        axes: MapAxesV1,
    ) -> Self {
        Self::Map(MapTerminalDescriptorV1 {
            source_site,
            stimulus,
            prestate,
            operation,
            phase,
            timing,
            occurrence,
            recipe,
            axes,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn lock(
        source_site: SourceSiteV1,
        stimulus: StimulusV1,
        prestate: PrestateV1,
        operation: LockOperationV1,
        phase: PhaseV1,
        timing: TimingV1,
        occurrence: OccurrenceV1,
        recipe: ExecutionRecipeV1,
        axes: LockAxesV1,
    ) -> Self {
        Self::Lock(LockTerminalDescriptorV1 {
            source_site,
            stimulus,
            prestate,
            operation,
            phase,
            timing,
            occurrence,
            recipe,
            axes,
        })
    }
}
