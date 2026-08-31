use super::super::super::{
    model::{CustodyState, MutationState},
    poison,
    terminal_descriptor::MapStoredPoisonPrestateV1,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct StoredPoisonPrestate {
    pub(super) label: &'static str,
    pub(super) file: CustodyState,
    pub(super) mapping: CustodyState,
    pub(super) view: CustodyState,
    pub(super) typed: MapStoredPoisonPrestateV1,
}

const fn stored(
    label: &'static str,
    file: CustodyState,
    mapping: CustodyState,
    view: CustodyState,
    typed: MapStoredPoisonPrestateV1,
) -> StoredPoisonPrestate {
    StoredPoisonPrestate {
        label,
        file,
        mapping,
        view,
        typed,
    }
}

const NO_NODE: StoredPoisonPrestate = stored(
    "no-node",
    CustodyState::NotReached,
    CustodyState::NotReached,
    CustodyState::NotReached,
    MapStoredPoisonPrestateV1::NoNode,
);
const LIVE_EMPTY: StoredPoisonPrestate = stored(
    "live-node-regions-empty",
    CustodyState::Retained,
    CustodyState::NotReached,
    CustodyState::NotReached,
    MapStoredPoisonPrestateV1::LiveNodeRegionsEmpty,
);
const LIVE_COMPLETE: StoredPoisonPrestate = stored(
    "live-node-complete-regions",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::Retained,
    MapStoredPoisonPrestateV1::LiveNodeCompleteRegions,
);
const QUARANTINED_EMPTY: StoredPoisonPrestate = stored(
    "node-absent-file-quarantined-no-regions",
    CustodyState::Quarantined,
    CustodyState::NotReached,
    CustodyState::NotReached,
    MapStoredPoisonPrestateV1::NodeAbsentFileQuarantinedNoRegions,
);
const QUARANTINED_RELEASED: StoredPoisonPrestate = stored(
    "node-absent-file-quarantined-regions-released",
    CustodyState::Quarantined,
    CustodyState::Released,
    CustodyState::Released,
    MapStoredPoisonPrestateV1::NodeAbsentFileQuarantinedRegionsReleased,
);
const RELEASED_EMPTY: StoredPoisonPrestate = stored(
    "node-absent-file-released-no-regions",
    CustodyState::Released,
    CustodyState::NotReached,
    CustodyState::NotReached,
    MapStoredPoisonPrestateV1::NodeAbsentFileReleasedNoRegions,
);
const RELEASED_REGIONS: StoredPoisonPrestate = stored(
    "node-absent-file-and-regions-released",
    CustodyState::Released,
    CustodyState::Released,
    CustodyState::Released,
    MapStoredPoisonPrestateV1::NodeAbsentFileAndRegionsReleased,
);
const MAPPING_ONLY_NO_VIEW: StoredPoisonPrestate = stored(
    "live-node-mapping-only-view-not-created",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::NotReached,
    MapStoredPoisonPrestateV1::LiveNodeMappingOnlyViewNotCreated,
);
const MAPPING_ONLY_VIEW_RELEASED: StoredPoisonPrestate = stored(
    "live-node-mapping-only-view-released",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::Released,
    MapStoredPoisonPrestateV1::LiveNodeMappingOnlyViewReleased,
);
const MAPPING_ONLY_WITH_RETAINED_VIEW: StoredPoisonPrestate = stored(
    "live-node-mapping-only-with-prior-retained-view",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::Retained,
    MapStoredPoisonPrestateV1::LiveNodeMappingOnlyWithRetainedView,
);
const VIEW_UNMAP_RETAINED: StoredPoisonPrestate = stored(
    "live-node-view-unmap-partial-retained",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::Retained,
    MapStoredPoisonPrestateV1::LiveNodeViewUnmapPartialRetained,
);
const LIVE_AFTER_REGION_RELEASE: StoredPoisonPrestate = stored(
    "live-node-regions-released",
    CustodyState::Retained,
    CustodyState::Released,
    CustodyState::Released,
    MapStoredPoisonPrestateV1::LiveNodeRegionsReleased,
);

pub(super) fn stored_poison_prestates(
    cell: poison::StoredPoisonCell,
) -> &'static [StoredPoisonPrestate] {
    use super::super::super::terminal_descriptor::PhaseV1 as Phase;
    match (cell.typed_phase, cell.mutation, cell.lock_outcome_uncertain) {
        (Phase::Gate, MutationState::None, false) => &[NO_NODE, LIVE_EMPTY, LIVE_COMPLETE],
        (Phase::FileClose, MutationState::None, false) => &[QUARANTINED_EMPTY],
        (Phase::ExactSiblingDelete, MutationState::None, false)
        | (Phase::ExactSiblingOpen, MutationState::Uncertain, false) => &[RELEASED_EMPTY],
        (Phase::DmsTruncate, MutationState::Uncertain, false) => &[LIVE_EMPTY],
        (Phase::FileClose, MutationState::Uncertain, false) => {
            &[QUARANTINED_EMPTY, QUARANTINED_RELEASED]
        }
        (Phase::ExactSiblingDelete, MutationState::Uncertain, false) => {
            &[RELEASED_EMPTY, RELEASED_REGIONS]
        }
        (Phase::FileGrow, MutationState::Uncertain, false) => &[LIVE_EMPTY, LIVE_COMPLETE],
        (Phase::MappingClose, MutationState::Uncertain, false) => &[
            MAPPING_ONLY_NO_VIEW,
            MAPPING_ONLY_VIEW_RELEASED,
            MAPPING_ONLY_WITH_RETAINED_VIEW,
        ],
        (Phase::ViewUnmap, MutationState::Uncertain, false) => &[VIEW_UNMAP_RETAINED],
        (Phase::LockRelease, MutationState::None, true)
        | (Phase::ConnectionDetach, MutationState::None, true) => &[LIVE_EMPTY, LIVE_COMPLETE],
        (Phase::DeleteAuthorization, MutationState::None, true) => {
            &[NO_NODE, LIVE_EMPTY, LIVE_COMPLETE]
        }
        (Phase::DmsExclusiveRelease, MutationState::Uncertain, true)
        | (Phase::DmsSharedRelease, MutationState::Uncertain, true) => {
            &[LIVE_EMPTY, LIVE_AFTER_REGION_RELEASE]
        }
        _ => panic!("unclassified production stored-poison cell: {cell:?}"),
    }
}
