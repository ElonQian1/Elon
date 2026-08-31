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
            terminal_descriptor::{
                FaultSeamV1, InitializationFaultSiteV1, InitializationProfileV1, MapFilePathV1,
                MapManagedStimulusV1, MapModeV1, MapOperationV1, MapPrestateV1, MapProfileV1,
                MapRegionPrestateV1, MapRegionSizeArmV1, SourceSiteV1, StimulusV1,
            },
        },
        builder::MapGraphBuilder,
        dynamic::{self, DescriptorSeedV1},
        projection::{self, FailureSpec},
        witnesses as w, MapMode,
    },
    build_region_size,
    mapping::build_map_or_reuse,
    poison_profile::stored_poison_prestates,
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

    pub(super) const fn descriptor(self) -> MapPrestateV1 {
        match self {
            Self::Empty => MapPrestateV1::RegionsEmpty,
            Self::Reuse => MapPrestateV1::TargetMapped,
            Self::Missing => MapPrestateV1::TargetMissing,
        }
    }

    const fn profile(self) -> MapRegionPrestateV1 {
        match self {
            Self::Empty => MapRegionPrestateV1::Empty,
            Self::Reuse => MapRegionPrestateV1::Reuse,
            Self::Missing => MapRegionPrestateV1::NonemptyTargetMissing,
        }
    }
}

pub(super) const fn profile(
    mode: MapMode,
    initialization: InitializationProfileV1,
    prestate: RegionPrestate,
    region_size_arm: MapRegionSizeArmV1,
    file_path: MapFilePathV1,
    prior_mutation: bool,
) -> MapProfileV1 {
    MapProfileV1 {
        mode: match mode {
            MapMode::Observe => MapModeV1::Observe,
            MapMode::Extend => MapModeV1::Extend,
        },
        initialization,
        prestate: prestate.profile(),
        region_size_arm,
        file_path,
        prior_mutation,
        preexisting_mapping: prestate.has_mapping(),
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
    if matches!(success.profile, InitializationProfileV1::NodeLive) {
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
                success.profile,
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
            success.profile,
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
    mode: MapMode,
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
                dynamic: DescriptorSeedV1::new(
                    initialization_source(failure.stimulus.fault_site),
                    StimulusV1::Initialization(failure.stimulus),
                    MapPrestateV1::NodeAbsent,
                    super::super::super::terminal_descriptor::MapOperationV1::Initialization,
                    failure.typed_phase,
                    failure.timing,
                    failure.occurrence,
                    FaultSeamV1::Initialization,
                    dynamic::mode_axes(mode),
                ),
            },
        );
    }
}

pub(super) fn project_stored_poison(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    mode: MapMode,
) {
    poison::validate_manifest();
    for cell in poison::STORED_POISON_CELLS {
        let label = cell.label();
        for prestate in stored_poison_prestates(*cell) {
            let mut stored = failure(
                cell.phase,
                FailureClass::OutcomeUncertainPoisoned,
                cell.mutation != MutationState::None,
                dynamic::managed_seed(
                    mode,
                    SourceSiteV1::CoordinatorState,
                    StimulusV1::MapManaged(MapManagedStimulusV1::StoredPoison),
                    MapPrestateV1::StoredPoison(prestate.typed),
                    MapOperationV1::ManagedRequest,
                    cell.typed_phase,
                    super::super::super::terminal_descriptor::TimingV1::BeforeCall,
                    FaultSeamV1::ManagedRequest,
                ),
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

pub(super) fn build_grow(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    profile: MapProfileV1,
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
        dynamic::profile_seed(
            MapProfileV1 {
                file_path: MapFilePathV1::GrowAttempted,
                ..profile
            },
            SourceSiteV1::MapFileGrow,
            StimulusV1::MapManaged(MapManagedStimulusV1::FileGrow),
            prestate.descriptor(),
            MapOperationV1::FileGrow,
            super::super::super::terminal_descriptor::PhaseV1::FileGrow,
            super::super::super::terminal_descriptor::TimingV1::AtCall,
            FaultSeamV1::NativeOperation,
        ),
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
        MapProfileV1 {
            file_path: MapFilePathV1::GrowSucceeded,
            prior_mutation: true,
            ..profile
        },
        prestate,
        initialization_mutated,
        true,
        dms_lock,
        spec.counts,
    );
}

fn initialization_source(site: InitializationFaultSiteV1) -> SourceSiteV1 {
    match site {
        InitializationFaultSiteV1::ParentValidationBeforeOpen
        | InitializationFaultSiteV1::ParentHandle
        | InitializationFaultSiteV1::PlatformOpen
        | InitializationFaultSiteV1::OpenCompletionValidation
        | InitializationFaultSiteV1::OpenFileValidation
        | InitializationFaultSiteV1::ParentValidationAfterOpen => SourceSiteV1::InitializationOpen,
        InitializationFaultSiteV1::DmsExclusiveAcquire
        | InitializationFaultSiteV1::DmsTruncate
        | InitializationFaultSiteV1::DmsExclusiveRelease
        | InitializationFaultSiteV1::DmsSharedAcquire => SourceSiteV1::InitializationDms,
    }
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

pub(super) fn failure(
    phase: &'static str,
    class: FailureClass,
    mutated: bool,
    dynamic: DescriptorSeedV1,
) -> FailureSpec {
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
        dynamic,
    }
}

pub(super) fn failure_with_prestate(
    phase: &'static str,
    class: FailureClass,
    mutated: bool,
    prestate: RegionPrestate,
    dynamic: DescriptorSeedV1,
) -> FailureSpec {
    let mut spec = failure(phase, class, mutated, dynamic);
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
