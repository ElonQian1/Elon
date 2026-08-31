mod exclusions;

use super::{
    super::model::{
        CustodyState, DecisionStage, DmsLockCustody, ExclusionProof, FailureClass, MutationState,
        ObservableCounts, TerminalDisposition,
    },
    super::terminal_descriptor::{
        FaultSeamV1, MapManagedStimulusV1, MapOperationV1, MapPrestateV1, MapProfileV1,
        MapRegionPrestateV1, PhaseV1, SourceSiteV1, StimulusV1, TimingV1,
    },
    builder::MapGraphBuilder,
    dynamic,
    projection::{self, FailureSpec},
    witnesses as w,
};
use exclusions::{
    add_iteration_exclusions, add_mapping_precondition_exclusions, add_view_precondition_exclusions,
};

const MAX_REGIONS: u16 = 256;
const MAX_REGION_SIZE: u64 = 64 * 1024;
const MAX_LOGICAL_BYTES: u64 = 8 * 1024 * 1024;
const MAX_MAPPED_BYTES: u64 = 24 * 1024 * 1024;
const WINDOWS_GRANULARITY: u64 = 64 * 1024;
const CANONICAL_REGION_SIZE: u64 = 32 * 1024;

pub(super) struct LoopSpec {
    pub(super) prefix: String,
    pub(super) max_ordinal: u16,
    pub(super) prior_mutation: bool,
    pub(super) preexisting_mapping: bool,
    pub(super) baseline_counts: ObservableCounts,
    pub(super) dms_lock: DmsLockCustody,
    pub(super) profile: MapProfileV1,
}

pub(super) fn assert_authority_loop_bounds() {
    assert_eq!(usize::BITS, 64, "production-Windows denominator is x64");
    assert_eq!(MAX_REGIONS, 256);
    assert_eq!(MAX_REGION_SIZE, WINDOWS_GRANULARITY);
    assert!(
        MAX_LOGICAL_BYTES + u64::from(MAX_REGIONS) * (WINDOWS_GRANULARITY - 1) <= MAX_MAPPED_BYTES
    );
    let mut mapped = 0_u64;
    for ordinal in 1..=u64::from(MAX_REGIONS) {
        let index = ordinal - 1;
        let offset = index * CANONICAL_REGION_SIZE;
        let shift = offset % WINDOWS_GRANULARITY;
        mapped += shift + CANONICAL_REGION_SIZE;
        assert!(ordinal * CANONICAL_REGION_SIZE <= MAX_LOGICAL_BYTES);
        assert!(mapped <= MAX_MAPPED_BYTES);
    }
}

pub(super) fn build(graph: &mut MapGraphBuilder, spec: LoopSpec) -> String {
    assert!((1..=MAX_REGIONS).contains(&spec.max_ordinal));
    let entry = iteration_node(graph, &spec.prefix, 1);
    let mut iteration = entry.clone();
    for ordinal in 1..=spec.max_ordinal {
        add_iteration_exclusions(graph, &iteration, &spec.prefix, ordinal);
        let create = graph.decision(
            &format!("{}.ordinal-{ordinal:03}.mapping-create", spec.prefix),
            w::managed("platform_shm::create_mapping(&node.file.file, logical_end)"),
        );
        graph.edge(
            &iteration,
            &create,
            DecisionStage::NativeCall,
            format!("ordinal_{ordinal:03}_validated_create_mapping"),
        );
        add_mapping_precondition_exclusions(graph, &create, &spec.prefix, ordinal);

        let mutated_before = spec.prior_mutation || ordinal > 1;
        if mutated_before {
            add_mapping_failure(
                graph,
                &create,
                &spec,
                ordinal,
                "after-known-mutation",
                FailureClass::MutatedButKnown,
                true,
            );
        } else {
            add_mapping_failure(
                graph,
                &create,
                &spec,
                ordinal,
                "io-before-mutation",
                FailureClass::IoBeforeMutation,
                false,
            );
            add_mapping_failure(
                graph,
                &create,
                &spec,
                ordinal,
                "platform-unsupported",
                FailureClass::PlatformUnsupported,
                false,
            );
        }

        let view = graph.decision(
            &format!("{}.ordinal-{ordinal:03}.view-map", spec.prefix),
            w::managed("platform_shm::map_view(&mapping, aligned_offset, mapped_length)"),
        );
        graph.edge(
            &create,
            &view,
            DecisionStage::NativeCall,
            "mapping_create_succeeded",
        );
        add_view_precondition_exclusions(graph, &view, &spec.prefix, ordinal);
        if mutated_before {
            add_view_failure(
                graph,
                &view,
                &spec,
                ordinal,
                "after-known-mutation",
                FailureClass::MutatedButKnown,
                true,
            );
        } else {
            add_view_failure(
                graph,
                &view,
                &spec,
                ordinal,
                "io-before-mutation",
                FailureClass::IoBeforeMutation,
                false,
            );
            add_view_failure(
                graph,
                &view,
                &spec,
                ordinal,
                "platform-unsupported",
                FailureClass::PlatformUnsupported,
                false,
            );
        }

        let base = graph.decision(
            &format!("{}.ordinal-{ordinal:03}.view-base", spec.prefix),
            w::managed("let Some(base) = view.base() else"),
        );
        graph.edge(
            &view,
            &base,
            DecisionStage::NativeCall,
            "view_map_succeeded",
        );
        let null = format!("{}.ordinal-{ordinal:03}.excluded.null-view", spec.prefix);
        graph.excluded(
            &null,
            ExclusionProof::TypeInvariant(
                "Windows map_view constructs OwnedSqliteShmView only from a non-null address",
            ),
            w::windows_shm("fn map_view", "if address.Value.is_null()"),
        );
        graph.edge(&base, &null, DecisionStage::NativeCall, "view_base_null");

        let record = graph.decision(
            &format!("{}.ordinal-{ordinal:03}.record-region", spec.prefix),
            w::managed("logical_pointer: Some(logical_pointer)"),
        );
        graph.edge(
            &base,
            &record,
            DecisionStage::Coordination,
            "view_base_non_null",
        );
        let missing = format!(
            "{}.ordinal-{ordinal:03}.excluded.node-missing-after-view-map",
            spec.prefix
        );
        graph.excluded(
            &missing,
            ExclusionProof::ControlFlow(
                "the mutex-held node is not removed during a successful mapping iteration",
            ),
            w::managed("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AFTER_VIEW_MAP"),
        );
        graph.edge(
            &record,
            &missing,
            DecisionStage::Coordination,
            "node_missing_after_view_map",
        );

        let control = graph.decision(
            &format!("{}.ordinal-{ordinal:03}.loop-control", spec.prefix),
            w::managed("while state"),
        );
        graph.edge(
            &record,
            &control,
            DecisionStage::Coordination,
            "region_recorded",
        );
        add_target_success(graph, &control, &spec, ordinal);
        if ordinal < spec.max_ordinal {
            let next = iteration_node(graph, &spec.prefix, ordinal + 1);
            graph.edge(
                &control,
                &next,
                DecisionStage::Coordination,
                format!("requested_target_after_ordinal_{ordinal:03}"),
            );
            iteration = next;
        } else {
            let beyond = format!("{}.excluded.target-after-ordinal-{ordinal:03}", spec.prefix);
            graph.excluded(
                &beyond,
                ExclusionProof::ControlFlow(
                    "authority region/logical/mapped budgets bound this concrete loop profile",
                ),
                w::managed_types(
                    "fn validate_logical_end",
                    "NODE_MANAGED_SQLITE_SHM_REGION_COUNT_BUDGET",
                ),
            );
            graph.edge(
                &control,
                &beyond,
                DecisionStage::ManagedRequest,
                format!("requested_target_after_ordinal_{ordinal:03}"),
            );
        }
    }
    entry
}

fn iteration_node(graph: &mut MapGraphBuilder, prefix: &str, ordinal: u16) -> String {
    graph.decision(
        &format!("{prefix}.ordinal-{ordinal:03}.iteration"),
        w::managed("while state"),
    )
}

fn add_mapping_failure(
    graph: &mut MapGraphBuilder,
    create: &str,
    spec: &LoopSpec,
    ordinal: u16,
    suffix: &str,
    class: FailureClass,
    mutated: bool,
) {
    let prefix = format!(
        "{}.ordinal-{ordinal:03}.mapping-create-{suffix}",
        spec.prefix
    );
    let cause = graph.decision(
        &format!("{prefix}.cause"),
        w::managed("mutation_class(prior_mapping_mutation, &error)"),
    );
    graph.edge(create, &cause, DecisionStage::NativeCall, suffix);
    projection::operation_failure(
        graph,
        &cause,
        &format!("{prefix}.projection"),
        failure_spec(spec, ordinal, PhaseV1::MappingCreate, class, mutated, false),
    );
}

fn add_view_failure(
    graph: &mut MapGraphBuilder,
    view: &str,
    spec: &LoopSpec,
    ordinal: u16,
    suffix: &str,
    class: FailureClass,
    mutated: bool,
) {
    let prefix = format!("{}.ordinal-{ordinal:03}.view-map-{suffix}", spec.prefix);
    let close = graph.decision(
        &format!("{prefix}.mapping-close"),
        w::windows_shm("fn close_explicit(", "CloseHandle(self.handle)"),
    );
    graph.edge(view, &close, DecisionStage::NativeCall, suffix);
    projection::operation_failure(
        graph,
        &close,
        &format!("{prefix}.close-succeeded"),
        failure_spec(spec, ordinal, PhaseV1::ViewMap, class, mutated, false),
    );

    let retained = graph.decision(
        &format!("{prefix}.close-failed-retain"),
        w::managed("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AT_MAP_FAILURE"),
    );
    graph.edge(
        &close,
        &retained,
        DecisionStage::Cleanup,
        "mapping_close_failed",
    );
    let missing = format!("{prefix}.excluded.node-missing-at-map-failure");
    graph.excluded(
        &missing,
        ExclusionProof::ControlFlow("mutex-held node survives the failed view call"),
        w::managed("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AT_MAP_FAILURE"),
    );
    graph.edge(
        &retained,
        &missing,
        DecisionStage::Cleanup,
        "node_missing_while_retaining_mapping",
    );
    let mut close_failure = failure_spec(
        spec,
        ordinal,
        PhaseV1::MappingClose,
        FailureClass::OutcomeUncertainPoisoned,
        true,
        true,
    );
    close_failure.mutation = MutationState::Uncertain;
    close_failure.disposition = TerminalDisposition::CleanupRewritten;
    projection::operation_failure(
        graph,
        &retained,
        &format!("{prefix}.close-failed"),
        close_failure,
    );
}

fn failure_spec(
    spec: &LoopSpec,
    ordinal: u16,
    phase: PhaseV1,
    failure: FailureClass,
    mutated: bool,
    retains_new_mapping: bool,
) -> FailureSpec {
    let prior_mapping = spec.preexisting_mapping || ordinal > 1;
    let prior_custody = if prior_mapping {
        CustodyState::Retained
    } else {
        CustodyState::Unchanged
    };
    let mut counts = spec.baseline_counts;
    counts.mapping_create += ordinal;
    counts.view_map += if matches!(phase, PhaseV1::MappingCreate) {
        ordinal - 1
    } else {
        ordinal
    };
    FailureSpec {
        phase: phase.static_name(),
        failure,
        mutation: if mutated {
            MutationState::Known
        } else {
            MutationState::None
        },
        disposition: if mutated {
            TerminalDisposition::Quarantined
        } else {
            TerminalDisposition::Returned
        },
        file: CustodyState::Retained,
        mapping: if retains_new_mapping {
            CustodyState::Retained
        } else {
            prior_custody
        },
        view: prior_custody,
        payload: CustodyState::NotReached,
        counts,
        quarantine: mutated,
        lock_outcome_uncertain: false,
        dms_lock: spec.dms_lock,
        dynamic: loop_failure_seed(spec, ordinal, phase),
    }
}

fn add_target_success(graph: &mut MapGraphBuilder, control: &str, spec: &LoopSpec, ordinal: u16) {
    let prefix = format!("{}.ordinal-{ordinal:03}.target", spec.prefix);
    let selected = graph.decision(
        &format!("{prefix}.selection"),
        w::managed("let selected = node.regions.get(region as usize)"),
    );
    graph.edge(
        control,
        &selected,
        DecisionStage::Coordination,
        format!("requested_target_reached_at_ordinal_{ordinal:03}"),
    );
    let missing = format!("{prefix}.excluded.region-custody-missing");
    graph.excluded(
        &missing,
        ExclusionProof::ControlFlow("the just-recorded target owns a non-null logical pointer"),
        w::managed("NODE_MANAGED_SQLITE_SHM_REGION_CUSTODY_MISSING"),
    );
    graph.edge(
        &selected,
        &missing,
        DecisionStage::Coordination,
        "selected_region_custody_missing",
    );
    let mut counts = spec.baseline_counts;
    counts.mapping_create += ordinal;
    counts.view_map += ordinal;
    projection::managed_success(
        graph,
        &selected,
        &format!("{prefix}.projection"),
        true,
        spec.preexisting_mapping,
        MutationState::Known,
        spec.dms_lock,
        counts,
        dynamic::ordinal_seed(
            spec.profile,
            ordinal,
            SourceSiteV1::AbiProjection,
            StimulusV1::MapManaged(MapManagedStimulusV1::Success),
            loop_prestate(spec.profile),
            MapOperationV1::SuccessProjection,
            PhaseV1::Success,
            TimingV1::Natural,
            FaultSeamV1::Natural,
        ),
    );
}

fn loop_failure_seed(spec: &LoopSpec, ordinal: u16, phase: PhaseV1) -> dynamic::DescriptorSeedV1 {
    let (site, stimulus, operation, timing, seam) = match phase {
        PhaseV1::MappingCreate => (
            SourceSiteV1::MapMappingCreate,
            MapManagedStimulusV1::MappingCreate,
            MapOperationV1::MappingCreate,
            TimingV1::AtCall,
            FaultSeamV1::NativeOperation,
        ),
        PhaseV1::ViewMap => (
            SourceSiteV1::MapViewMap,
            MapManagedStimulusV1::ViewMap,
            MapOperationV1::ViewMap,
            TimingV1::AtCall,
            FaultSeamV1::NativeOperation,
        ),
        PhaseV1::MappingClose => (
            SourceSiteV1::MapMappingClose,
            MapManagedStimulusV1::MappingClose,
            MapOperationV1::MappingClose,
            TimingV1::Cleanup,
            FaultSeamV1::Cleanup,
        ),
        _ => unreachable!("Map loop failure uses one of three typed phases"),
    };
    dynamic::ordinal_seed(
        spec.profile,
        ordinal,
        site,
        StimulusV1::MapManaged(stimulus),
        loop_prestate(spec.profile),
        operation,
        phase,
        timing,
        seam,
    )
}

fn loop_prestate(profile: MapProfileV1) -> MapPrestateV1 {
    match profile.prestate {
        MapRegionPrestateV1::Empty => MapPrestateV1::RegionsEmpty,
        MapRegionPrestateV1::NonemptyTargetMissing => MapPrestateV1::TargetMissing,
        MapRegionPrestateV1::Reuse => MapPrestateV1::TargetMapped,
        MapRegionPrestateV1::ObserveNotPresent => MapPrestateV1::RegionsEmpty,
    }
}
