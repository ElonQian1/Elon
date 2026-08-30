mod helpers;
mod mapping;

use super::{
    super::{
        initialization,
        model::{DecisionStage, DmsLockCustody, ExclusionProof, FailureClass, ObservableCounts},
        poison,
    },
    builder::MapGraphBuilder,
    witnesses as w, MapMode,
};
use helpers::{
    add_failure_branch, build_post_initialization, exclude_region_arm, failure,
    failure_with_prestate, gate, project_stored_poison, RegionPrestate,
};
use mapping::build_file_size;

pub(super) fn build(graph: &mut MapGraphBuilder, admitted: &str, mode: MapMode) {
    let prefix = format!("map.{}.managed", mode.name());
    let active = graph.decision(
        &format!("{prefix}.connection-active"),
        w::pinned_map("if !self.active"),
    );
    graph.edge(
        admitted,
        &active,
        DecisionStage::ManagedRequest,
        "pinned_shm_map_entered",
    );
    let inactive = graph.excluded(
        &format!("{prefix}.excluded.connection-inactive"),
        ExclusionProof::ControlFlow(
            "production sets active=false only after consuming unmap success; its sole detached-error constructor follows a same-guard connection-removal invariant",
        ),
        w::pinned_map("NODE_MANAGED_SQLITE_SHM_CONNECTION_INACTIVE"),
    );
    graph.edge(
        &active,
        &inactive,
        DecisionStage::ManagedRequest,
        "connection_inactive",
    );

    let region_size = gate(
        graph,
        &active,
        &format!("{prefix}.region-size-budget"),
        DecisionStage::ManagedRequest,
        "connection_active",
        w::managed_types(
            "fn validate_region_size",
            "if size.get() > self.max_region_size",
        ),
    );
    add_failure_branch(
        graph,
        &region_size,
        &format!("{prefix}.region-size-rejected"),
        DecisionStage::ManagedRequest,
        "region_size_exceeds_authority_budget",
        w::managed_types(
            "fn validate_region_size",
            "NODE_MANAGED_SQLITE_SHM_REGION_SIZE_BUDGET",
        ),
        failure("RequestValidation", FailureClass::ProtocolViolation, false),
    );
    let region_count = gate(
        graph,
        &region_size,
        &format!("{prefix}.region-count-budget"),
        DecisionStage::ManagedRequest,
        "region_size_within_authority_budget",
        w::managed_types("fn validate_logical_end", "if region >= self.max_regions"),
    );
    add_failure_branch(
        graph,
        &region_count,
        &format!("{prefix}.region-count-rejected"),
        DecisionStage::ManagedRequest,
        "region_index_exceeds_authority_budget",
        w::managed_types(
            "fn validate_logical_end",
            "NODE_MANAGED_SQLITE_SHM_REGION_COUNT_BUDGET",
        ),
        failure("RequestValidation", FailureClass::ProtocolViolation, false),
    );
    let logical_end = gate(
        graph,
        &region_count,
        &format!("{prefix}.logical-end"),
        DecisionStage::ManagedRequest,
        "region_index_within_authority_budget",
        w::managed_types("fn validate_logical_end", ".checked_add(1)"),
    );
    let overflow = format!("{prefix}.excluded.logical-end-overflow");
    graph.excluded(
        &overflow,
        ExclusionProof::ControlFlow(
            "u32 region below 256 times nonzero u32 region size cannot overflow u64",
        ),
        w::managed_types(
            "fn validate_logical_end",
            "NODE_MANAGED_SQLITE_SHM_LOGICAL_END_OVERFLOW",
        ),
    );
    graph.edge(
        &logical_end,
        &overflow,
        DecisionStage::ManagedRequest,
        "checked_logical_end_overflow",
    );
    add_failure_branch(
        graph,
        &logical_end,
        &format!("{prefix}.logical-size-rejected"),
        DecisionStage::ManagedRequest,
        "logical_end_exceeds_authority_budget",
        w::managed_types(
            "fn validate_logical_end",
            "NODE_MANAGED_SQLITE_SHM_LOGICAL_SIZE_BUDGET",
        ),
        failure("RequestValidation", FailureClass::ProtocolViolation, false),
    );

    let granularity = gate(
        graph,
        &logical_end,
        &format!("{prefix}.allocation-granularity"),
        DecisionStage::ManagedRequest,
        "logical_end_within_authority_budget",
        w::managed("platform_shm::allocation_granularity().map_err"),
    );
    add_failure_branch(
        graph,
        &granularity,
        &format!("{prefix}.allocation-granularity-error"),
        DecisionStage::NativeCall,
        "windows_granularity_query_failed",
        w::windows_shm(
            "fn allocation_granularity",
            "NODE_MANAGED_SQLITE_SHM_ALLOCATION_GRANULARITY_ZERO",
        ),
        failure("RequestValidation", FailureClass::IoBeforeMutation, false),
    );
    let zero = format!("{prefix}.excluded.allocation-granularity-zero-after-ok");
    graph.excluded(
        &zero,
        ExclusionProof::ControlFlow("Windows allocation_granularity returns Err rather than Ok(0)"),
        w::managed("if granularity == 0"),
    );
    graph.edge(
        &granularity,
        &zero,
        DecisionStage::ManagedRequest,
        "granularity_ok_zero",
    );

    let state = gate(
        graph,
        &granularity,
        &format!("{prefix}.coordinator-state"),
        DecisionStage::ManagedRequest,
        "granularity_ok_nonzero",
        w::managed("let mut state = self"),
    );
    let mutex_poisoned = graph.excluded(
        &format!("{prefix}.coordinator-mutex-poisoned"),
        poison::coordinator_mutex_poison_proof(),
        w::managed("self.poisoned_failure())?"),
    );
    graph.edge(
        &state,
        &mutex_poisoned,
        DecisionStage::Coordination,
        "coordinator_mutex_poisoned",
    );
    let attached = gate(
        graph,
        &state,
        &format!("{prefix}.connection-membership"),
        DecisionStage::Coordination,
        "coordinator_mutex_acquired",
        w::managed("state.connections.contains_key(&connection_id)"),
    );
    let missing = graph.excluded(
        &format!("{prefix}.excluded.connection-missing"),
        ExclusionProof::ControlFlow(
            "the connection object and coordinator membership are minted together; production removal consumes that object before another Map can borrow it",
        ),
        w::managed("NODE_MANAGED_SQLITE_SHM_CONNECTION_NOT_ATTACHED"),
    );
    graph.edge(
        &attached,
        &missing,
        DecisionStage::Coordination,
        "connection_id_not_attached",
    );

    let existing_poison = gate(
        graph,
        &attached,
        &format!("{prefix}.stored-poison"),
        DecisionStage::Coordination,
        "connection_id_attached",
        w::managed("if let Some(poison) = state.poisoned"),
    );
    project_stored_poison(graph, &existing_poison, &prefix);

    let init_prefix = format!("{prefix}.initialization");
    let (entry, successes, failures) =
        graph.merge_initialization(initialization::build(&init_prefix));
    graph.edge(
        &existing_poison,
        &entry,
        DecisionStage::Initialization,
        "stored_poison_absent_enter_initialization",
    );
    for success in successes {
        build_post_initialization(graph, mode, &success);
    }
    helpers::project_initialization_failures(graph, failures);
}

#[allow(clippy::too_many_arguments)]
fn build_region_size(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    mode: MapMode,
    prestate: RegionPrestate,
    new_node: bool,
    initialization_mutated: bool,
    dms_lock: DmsLockCustody,
    baseline_counts: ObservableCounts,
) {
    let profile = format!("{prefix}.{}", prestate.name());
    let region_size = gate(
        graph,
        from,
        &format!("{profile}.region-size"),
        DecisionStage::Coordination,
        if new_node {
            "new_node_regions_empty"
        } else {
            prestate.name()
        },
        w::managed("match node.region_size"),
    );
    if new_node {
        exclude_region_arm(
            graph,
            &region_size,
            &profile,
            "some-changed",
            "new_node stores region_size None",
        );
        exclude_region_arm(
            graph,
            &region_size,
            &profile,
            "some-same",
            "new_node stores region_size None",
        );
        build_file_size(
            graph,
            &region_size,
            &format!("{profile}.region-size-unset"),
            "region_size_unset_assigned",
            mode,
            prestate,
            initialization_mutated,
            dms_lock,
            baseline_counts,
        );
        return;
    }

    add_failure_branch(
        graph,
        &region_size,
        &format!("{profile}.region-size-changed"),
        DecisionStage::ManagedRequest,
        "region_size_changed",
        w::managed("NODE_MANAGED_SQLITE_SHM_REGION_SIZE_CHANGED"),
        {
            let mut spec = failure_with_prestate(
                "RequestValidation",
                FailureClass::ProtocolViolation,
                false,
                prestate,
            );
            spec.dms_lock = dms_lock;
            spec.counts = baseline_counts;
            spec
        },
    );
    build_file_size(
        graph,
        &region_size,
        &format!("{profile}.region-size-same"),
        "region_size_same",
        mode,
        prestate,
        false,
        dms_lock,
        baseline_counts,
    );
    if prestate.has_mapping() {
        exclude_region_arm(
            graph,
            &region_size,
            &profile,
            "none-with-existing-regions",
            "recorded regions imply region_size was assigned by an earlier map",
        );
    } else {
        build_file_size(
            graph,
            &region_size,
            &format!("{profile}.region-size-unset"),
            "region_size_unset_assigned",
            mode,
            prestate,
            false,
            dms_lock,
            baseline_counts,
        );
    }
}
