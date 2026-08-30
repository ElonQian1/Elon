use super::{
    super::{
        super::model::{
            DecisionStage, DmsLockCustody, ExclusionProof, FailureClass, ObservableCounts,
            TerminalDisposition,
        },
        builder::MapGraphBuilder,
        loop_expansion::{self, LoopSpec},
        projection, witnesses as w, MapMode,
    },
    helpers::{
        add_failure_branch, build_grow, failure_with_prestate, gate, mutation, RegionPrestate,
    },
};

#[allow(clippy::too_many_arguments)]
pub(super) fn build_file_size(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    entry_branch: &str,
    mode: MapMode,
    prestate: RegionPrestate,
    initialization_mutated: bool,
    dms_lock: DmsLockCustody,
    baseline_counts: ObservableCounts,
) {
    let file_size = gate(
        graph,
        from,
        &format!("{prefix}.file-size"),
        DecisionStage::NativeCall,
        entry_branch,
        w::managed("let current_size = state"),
    );
    let mut existing_size_failure = failure_with_prestate(
        "RequestValidation",
        FailureClass::ProtocolViolation,
        initialization_mutated,
        prestate,
    );
    existing_size_failure.quarantine = false;
    existing_size_failure.disposition = TerminalDisposition::Returned;
    existing_size_failure.dms_lock = dms_lock;
    existing_size_failure.counts = baseline_counts;
    let native_classes: &[(&str, &str, FailureClass)] = if initialization_mutated {
        &[(
            "after-mutation",
            "file_size_native_error_after_initialization_mutation",
            FailureClass::MutatedButKnown,
        )]
    } else {
        &[
            (
                "platform-unsupported",
                "file_size_platform_unsupported",
                FailureClass::PlatformUnsupported,
            ),
            (
                "io-before-mutation",
                "file_size_io_before_mutation",
                FailureClass::IoBeforeMutation,
            ),
        ]
    };
    for &(suffix, branch, class) in native_classes {
        let mut native_size_failure =
            failure_with_prestate("FileSize", class, initialization_mutated, prestate);
        native_size_failure.dms_lock = dms_lock;
        native_size_failure.counts = baseline_counts;
        add_failure_branch(
            graph,
            &file_size,
            &format!("{prefix}.file-size-native-error.{suffix}"),
            DecisionStage::NativeCall,
            branch,
            w::managed("mutation_class(initialization_mutated, &error)"),
            native_size_failure,
        );
    }
    add_failure_branch(
        graph,
        &file_size,
        &format!("{prefix}.existing-size-rejected"),
        DecisionStage::ManagedRequest,
        "existing_size_exceeds_authority_budget",
        w::managed_types(
            "fn validate_existing_size",
            "NODE_MANAGED_SQLITE_SHM_EXISTING_SIZE_BUDGET",
        ),
        existing_size_failure,
    );

    match mode {
        MapMode::Observe => {
            let short = gate(
                graph,
                &file_size,
                &format!("{prefix}.observe-not-present"),
                DecisionStage::ManagedRequest,
                "current_size_short",
                w::managed("return Ok(ManagedSqliteShmMapOutcome::NotPresent);"),
            );
            projection::managed_success(
                graph,
                &short,
                &format!("{prefix}.observe-not-present.projection"),
                false,
                prestate.has_mapping(),
                mutation(initialization_mutated),
                dms_lock,
                baseline_counts,
            );
        }
        MapMode::Extend => build_grow(
            graph,
            &file_size,
            &format!("{prefix}.extend-grow"),
            prestate,
            initialization_mutated,
            dms_lock,
            baseline_counts,
        ),
    }
    build_map_or_reuse(
        graph,
        &file_size,
        &format!("{prefix}.size-sufficient"),
        "current_size_sufficient",
        prestate,
        initialization_mutated,
        false,
        dms_lock,
        baseline_counts,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_map_or_reuse(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    branch: &str,
    prestate: RegionPrestate,
    initialization_mutated: bool,
    file_grew: bool,
    dms_lock: DmsLockCustody,
    baseline_counts: ObservableCounts,
) {
    if matches!(prestate, RegionPrestate::Reuse) {
        let selected = gate(
            graph,
            from,
            &format!("{prefix}.reuse-selection"),
            DecisionStage::Coordination,
            branch,
            w::managed("let selected = node.regions.get(region as usize)"),
        );
        let missing = format!("{prefix}.excluded.reused-region-custody-missing");
        graph.excluded(
            &missing,
            ExclusionProof::ControlFlow(
                "a live unpoisoned recorded region retains both view and logical pointer",
            ),
            w::managed("NODE_MANAGED_SQLITE_SHM_REGION_CUSTODY_MISSING"),
        );
        graph.edge(
            &selected,
            &missing,
            DecisionStage::Coordination,
            "selected_region_custody_missing",
        );
        projection::managed_success(
            graph,
            &selected,
            &format!("{prefix}.reuse.projection"),
            true,
            true,
            mutation(initialization_mutated || file_grew),
            dms_lock,
            baseline_counts,
        );
        return;
    }

    let entry = loop_expansion::build(
        graph,
        LoopSpec {
            prefix: format!("{prefix}.region-loop"),
            max_ordinal: if matches!(prestate, RegionPrestate::Empty) {
                256
            } else {
                255
            },
            prior_mutation: initialization_mutated || file_grew || prestate.has_mapping(),
            preexisting_mapping: prestate.has_mapping(),
            baseline_counts,
            dms_lock,
        },
    );
    graph.edge(from, &entry, DecisionStage::Coordination, branch);
}
