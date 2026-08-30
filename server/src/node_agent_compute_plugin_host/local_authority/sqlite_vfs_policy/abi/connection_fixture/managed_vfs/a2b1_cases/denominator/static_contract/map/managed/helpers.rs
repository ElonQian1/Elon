use super::{
    super::{
        super::{
            initialization::{InitializationFailure, InitializationSuccess},
            model::{
                CustodyState, DecisionStage, DmsLockCustody, ExclusionProof, FailureClass,
                MutationState, ObservableCounts, TerminalDisposition,
            },
            poison,
            source::SourceWitness,
        },
        builder::MapGraphBuilder,
        projection::{self, FailureSpec},
        witnesses as w, MapMode,
    },
    build_region_size,
    mapping::build_map_or_reuse,
};

#[derive(Debug, Clone, Copy)]
pub(super) enum RegionPrestate {
    Empty,
    Reuse,
    Missing,
}

impl RegionPrestate {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Empty => "regions-empty",
            Self::Reuse => "target-reusable",
            Self::Missing => "regions-nonempty-target-missing",
        }
    }

    pub(super) const fn has_mapping(self) -> bool {
        !matches!(self, Self::Empty)
    }
}

pub(super) fn build_post_initialization(
    graph: &mut MapGraphBuilder,
    mode: MapMode,
    success: &InitializationSuccess,
) {
    let prefix = format!("{}.post-init", success.node);
    let initialization_mutated = success.mutation != MutationState::None;
    let baseline_counts = ObservableCounts {
        native_lock: success.native_lock,
        native_unlock: success.native_unlock,
        ..ObservableCounts::default()
    };
    if success.label == "node-live" {
        let prestate = graph.decision(&format!("{prefix}.region-prestate"), w::managed(".regions"));
        graph.edge(
            &success.node,
            &prestate,
            DecisionStage::Coordination,
            "expand_live_node_post_initialization",
        );
        for state in [
            RegionPrestate::Empty,
            RegionPrestate::Reuse,
            RegionPrestate::Missing,
        ] {
            build_region_size(
                graph,
                &prestate,
                &prefix,
                mode,
                state,
                false,
                false,
                success.dms_lock,
                baseline_counts,
            );
        }
    } else {
        build_region_size(
            graph,
            &success.node,
            &prefix,
            mode,
            RegionPrestate::Empty,
            true,
            initialization_mutated,
            success.dms_lock,
            baseline_counts,
        );
    }
}

pub(super) fn project_initialization_failures(
    graph: &mut MapGraphBuilder,
    failures: Vec<InitializationFailure>,
) {
    for failure in failures {
        let quarantine = failure.class == FailureClass::OutcomeUncertainPoisoned
            || failure.mutation != MutationState::None
            || failure.lock_uncertain;
        projection::operation_failure(
            graph,
            &failure.node,
            &failure.projection_prefix,
            FailureSpec {
                phase: failure.phase,
                failure: failure.class,
                mutation: failure.mutation,
                disposition: failure.disposition,
                file: failure.file,
                mapping: CustodyState::NotReached,
                view: CustodyState::NotReached,
                payload: CustodyState::NotReached,
                counts: ObservableCounts {
                    native_lock: failure.native_lock,
                    native_unlock: failure.native_unlock,
                    ..ObservableCounts::default()
                },
                quarantine,
                lock_outcome_uncertain: failure.lock_uncertain,
                dms_lock: failure.dms_lock,
            },
        );
    }
}

pub(super) fn project_stored_poison(graph: &mut MapGraphBuilder, from: &str, prefix: &str) {
    poison::validate_manifest();
    for cell in poison::STORED_POISON_CELLS {
        let label = cell.label();
        for prestate in stored_poison_prestates(*cell) {
            let mut stored = failure(
                cell.phase,
                FailureClass::OutcomeUncertainPoisoned,
                cell.mutation != MutationState::None,
            );
            stored.mutation = cell.mutation;
            stored.disposition = TerminalDisposition::Quarantined;
            stored.file = prestate.file;
            stored.mapping = prestate.mapping;
            stored.view = prestate.view;
            stored.quarantine = true;
            stored.lock_outcome_uncertain = cell.lock_outcome_uncertain;
            stored.dms_lock = DmsLockCustody::UnobservedRetained;
            add_failure_branch(
                graph,
                from,
                &format!(
                    "{prefix}.stored-poison-returned.{label}.prestate.{}",
                    prestate.label
                ),
                DecisionStage::Coordination,
                &format!("stored_poison_present.{label}.prestate.{}", prestate.label),
                w::managed("return Err(poison.failure());"),
                stored,
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct StoredPoisonPrestate {
    label: &'static str,
    file: CustodyState,
    mapping: CustodyState,
    view: CustodyState,
}

const fn stored_prestate(
    label: &'static str,
    file: CustodyState,
    mapping: CustodyState,
    view: CustodyState,
) -> StoredPoisonPrestate {
    StoredPoisonPrestate {
        label,
        file,
        mapping,
        view,
    }
}

const NO_NODE: StoredPoisonPrestate = stored_prestate(
    "no-node",
    CustodyState::NotReached,
    CustodyState::NotReached,
    CustodyState::NotReached,
);
const LIVE_EMPTY: StoredPoisonPrestate = stored_prestate(
    "live-node-regions-empty",
    CustodyState::Retained,
    CustodyState::NotReached,
    CustodyState::NotReached,
);
const LIVE_COMPLETE: StoredPoisonPrestate = stored_prestate(
    "live-node-complete-regions",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::Retained,
);
const QUARANTINED_EMPTY: StoredPoisonPrestate = stored_prestate(
    "node-absent-file-quarantined-no-regions",
    CustodyState::Quarantined,
    CustodyState::NotReached,
    CustodyState::NotReached,
);
const QUARANTINED_RELEASED: StoredPoisonPrestate = stored_prestate(
    "node-absent-file-quarantined-regions-released",
    CustodyState::Quarantined,
    CustodyState::Released,
    CustodyState::Released,
);
const RELEASED_EMPTY: StoredPoisonPrestate = stored_prestate(
    "node-absent-file-released-no-regions",
    CustodyState::Released,
    CustodyState::NotReached,
    CustodyState::NotReached,
);
const RELEASED_REGIONS: StoredPoisonPrestate = stored_prestate(
    "node-absent-file-and-regions-released",
    CustodyState::Released,
    CustodyState::Released,
    CustodyState::Released,
);
const MAPPING_ONLY_NO_VIEW: StoredPoisonPrestate = stored_prestate(
    "live-node-mapping-only-view-not-created",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::NotReached,
);
const MAPPING_ONLY_VIEW_RELEASED: StoredPoisonPrestate = stored_prestate(
    "live-node-mapping-only-view-released",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::Released,
);
const MAPPING_ONLY_WITH_RETAINED_VIEW: StoredPoisonPrestate = stored_prestate(
    "live-node-mapping-only-with-prior-retained-view",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::Retained,
);
const VIEW_UNMAP_RETAINED: StoredPoisonPrestate = stored_prestate(
    "live-node-view-unmap-partial-retained",
    CustodyState::Retained,
    CustodyState::Retained,
    CustodyState::Retained,
);
const LIVE_AFTER_REGION_RELEASE: StoredPoisonPrestate = stored_prestate(
    "live-node-regions-released",
    CustodyState::Retained,
    CustodyState::Released,
    CustodyState::Released,
);

fn stored_poison_prestates(cell: poison::StoredPoisonCell) -> &'static [StoredPoisonPrestate] {
    match (cell.phase, cell.mutation, cell.lock_outcome_uncertain) {
        ("Gate", MutationState::None, false) => &[NO_NODE, LIVE_EMPTY, LIVE_COMPLETE],
        ("FileClose", MutationState::None, false) => &[QUARANTINED_EMPTY],
        ("ExactSiblingDelete", MutationState::None, false) => &[RELEASED_EMPTY],
        ("ExactSiblingOpen", MutationState::Uncertain, false) => &[RELEASED_EMPTY],
        ("DmsTruncate", MutationState::Uncertain, false) => &[LIVE_EMPTY],
        ("FileClose", MutationState::Uncertain, false) => {
            &[QUARANTINED_EMPTY, QUARANTINED_RELEASED]
        }
        ("ExactSiblingDelete", MutationState::Uncertain, false) => {
            &[RELEASED_EMPTY, RELEASED_REGIONS]
        }
        ("FileGrow", MutationState::Uncertain, false) => &[LIVE_EMPTY, LIVE_COMPLETE],
        ("MappingClose", MutationState::Uncertain, false) => &[
            MAPPING_ONLY_NO_VIEW,
            MAPPING_ONLY_VIEW_RELEASED,
            MAPPING_ONLY_WITH_RETAINED_VIEW,
        ],
        ("ViewUnmap", MutationState::Uncertain, false) => &[VIEW_UNMAP_RETAINED],
        ("LockRelease", MutationState::None, true)
        | ("ConnectionDetach", MutationState::None, true) => &[LIVE_EMPTY, LIVE_COMPLETE],
        ("DeleteAuthorization", MutationState::None, true) => &[NO_NODE, LIVE_EMPTY, LIVE_COMPLETE],
        ("DmsExclusiveRelease", MutationState::Uncertain, true)
        | ("DmsSharedRelease", MutationState::Uncertain, true) => {
            &[LIVE_EMPTY, LIVE_AFTER_REGION_RELEASE]
        }
        _ => panic!("unclassified production stored-poison cell: {cell:?}"),
    }
}

pub(super) fn build_grow(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    prestate: RegionPrestate,
    initialization_mutated: bool,
    dms_lock: DmsLockCustody,
    baseline_counts: ObservableCounts,
) {
    let grow = gate(
        graph,
        from,
        &format!("{prefix}.native"),
        DecisionStage::NativeCall,
        "current_size_short",
        w::managed(".truncate(logical_end)"),
    );
    let mut spec = failure_with_prestate(
        "FileGrow",
        FailureClass::OutcomeUncertainPoisoned,
        true,
        prestate,
    );
    spec.mutation = MutationState::Uncertain;
    spec.counts = baseline_counts;
    spec.counts.file_grow += 1;
    spec.dms_lock = dms_lock;
    add_failure_branch(
        graph,
        &grow,
        &format!("{prefix}.native-error"),
        DecisionStage::NativeCall,
        "file_grow_native_error",
        w::managed("if let Err(error) = grow"),
        spec,
    );
    build_map_or_reuse(
        graph,
        &grow,
        &format!("{prefix}.succeeded"),
        "file_grow_succeeded",
        prestate,
        initialization_mutated,
        true,
        dms_lock,
        spec.counts,
    );
}

pub(super) fn gate(
    graph: &mut MapGraphBuilder,
    from: &str,
    id: &str,
    stage: DecisionStage,
    branch: &str,
    witness: SourceWitness,
) -> String {
    let node = graph.decision(id, witness);
    graph.edge(from, &node, stage, branch);
    node
}

#[allow(clippy::too_many_arguments)]
pub(super) fn add_failure_branch(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    stage: DecisionStage,
    branch: &str,
    witness: SourceWitness,
    spec: FailureSpec,
) {
    let cause = graph.decision(&format!("{prefix}.cause"), witness);
    graph.edge(from, &cause, stage, branch);
    projection::operation_failure(graph, &cause, &format!("{prefix}.projection"), spec);
}

pub(super) fn exclude_region_arm(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    suffix: &str,
    proof: &'static str,
) {
    let id = format!("{prefix}.excluded.region-size-{suffix}");
    graph.excluded(
        &id,
        ExclusionProof::ControlFlow(proof),
        w::managed("match node.region_size"),
    );
    graph.edge(from, &id, DecisionStage::Coordination, suffix);
}

pub(super) fn failure(phase: &'static str, class: FailureClass, mutated: bool) -> FailureSpec {
    FailureSpec {
        phase,
        failure: class,
        mutation: mutation(mutated),
        disposition: if mutated {
            TerminalDisposition::Quarantined
        } else {
            TerminalDisposition::Returned
        },
        file: CustodyState::Retained,
        mapping: CustodyState::NotReached,
        view: CustodyState::NotReached,
        payload: CustodyState::NotReached,
        counts: ObservableCounts::default(),
        quarantine: mutated,
        lock_outcome_uncertain: false,
        dms_lock: DmsLockCustody::NotReached,
    }
}

pub(super) fn failure_with_prestate(
    phase: &'static str,
    class: FailureClass,
    mutated: bool,
    prestate: RegionPrestate,
) -> FailureSpec {
    let mut spec = failure(phase, class, mutated);
    spec.mapping = if prestate.has_mapping() {
        CustodyState::Retained
    } else {
        CustodyState::Unchanged
    };
    spec.view = spec.mapping;
    spec
}

pub(super) const fn mutation(mutated: bool) -> MutationState {
    if mutated {
        MutationState::Known
    } else {
        MutationState::None
    }
}
