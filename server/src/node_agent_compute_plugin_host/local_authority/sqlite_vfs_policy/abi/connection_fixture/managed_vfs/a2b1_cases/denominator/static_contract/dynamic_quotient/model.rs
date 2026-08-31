use super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
        ObservableCountsV1, RootOperationV1, SqliteResultV1, TerminalDispositionV1,
    },
    terminal_descriptor::{
        ExecutionRecipeV1, LockAxesV1, LockOperationV1, MapAxesV1, MapOperationV1, OccurrenceV1,
        PhaseV1, PrestateV1, SourceSiteV1, StimulusV1, TimingV1,
    },
};

pub(crate) const DYNAMIC_PROJECTOR_SCHEMA_V1: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StaticMemberSealV1 {
    pub(crate) case_key_sha256: Digest32,
    pub(crate) full_record_sha256: Digest32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DynamicExpectedV1 {
    pub(crate) sqlite: SqliteResultV1,
    pub(crate) disposition: TerminalDispositionV1,
    pub(crate) phase: PhaseV1,
    pub(crate) failure: FailureClassV1,
    pub(crate) mutation: MutationStateV1,
    pub(crate) lock_outcome_uncertain: bool,
    pub(crate) lock_effect: LockEffectV1,
    pub(crate) dms_lock: DmsLockCustodyV1,
    pub(crate) raw_slots: CustodyStateV1,
    pub(crate) route: CustodyStateV1,
    pub(crate) callback: CustodyStateV1,
    pub(crate) file: CustodyStateV1,
    pub(crate) mapping: CustodyStateV1,
    pub(crate) view: CustodyStateV1,
    pub(crate) payload: CustodyStateV1,
    pub(crate) counts: ObservableCountsV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DynamicOperationV1 {
    Map(MapOperationV1),
    Lock(LockOperationV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DynamicAxesV1 {
    Map(MapAxesV1),
    Lock(LockAxesV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DynamicClassKeyV1 {
    pub(crate) schema_version: u16,
    pub(crate) root: RootOperationV1,
    pub(crate) source_site: SourceSiteV1,
    pub(crate) stimulus: StimulusV1,
    pub(crate) prestate: PrestateV1,
    pub(crate) operation: DynamicOperationV1,
    pub(crate) phase: PhaseV1,
    pub(crate) timing: TimingV1,
    pub(crate) occurrence: OccurrenceV1,
    pub(crate) recipe: ExecutionRecipeV1,
    pub(crate) axes: DynamicAxesV1,
    pub(crate) expected: DynamicExpectedV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DynamicProjectionV1 {
    pub(crate) key: DynamicClassKeyV1,
    pub(crate) class_key_sha256: Digest32,
    pub(crate) member: StaticMemberSealV1,
}
