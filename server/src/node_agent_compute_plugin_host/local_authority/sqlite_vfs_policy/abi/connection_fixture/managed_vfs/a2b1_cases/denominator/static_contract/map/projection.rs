mod exclusions;

use super::{
    super::{
        model::{
            CustodyState, DecisionStage, DmsLockCustody, ExclusionProof, FailureClass,
            MutationState, ObservableCounts, TerminalDisposition,
        },
        poison,
    },
    builder::MapGraphBuilder,
    expected, witnesses as w,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct FailureSpec {
    pub(super) phase: &'static str,
    pub(super) failure: FailureClass,
    pub(super) mutation: MutationState,
    pub(super) disposition: TerminalDisposition,
    pub(super) file: CustodyState,
    pub(super) mapping: CustodyState,
    pub(super) view: CustodyState,
    pub(super) payload: CustodyState,
    pub(super) counts: ObservableCounts,
    pub(super) quarantine: bool,
    pub(super) lock_outcome_uncertain: bool,
    pub(super) dms_lock: DmsLockCustody,
}

pub(super) fn operation_failure(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    spec: FailureSpec,
) {
    if spec.quarantine {
        unsafe_failure(graph, from, prefix, spec);
    } else {
        safe_failure(graph, from, prefix, spec);
    }
}

pub(super) fn callback_admission_failure(graph: &mut MapGraphBuilder, from: &str, prefix: &str) {
    exclude_owner_poison(
        graph,
        from,
        &format!("{prefix}.owner-poisoned"),
        DecisionStage::CallbackAdmission,
        "owner-poisoned",
    );
    for (variant, route, disposition, witness) in [
        (
            "route-unknown-prior-quarantine",
            CustodyState::Quarantined,
            TerminalDisposition::Returned,
            w::registry_owner(
                "fn exact_entry_mut",
                "ManagedSqliteRegistryRouteRejection::UnknownOrRetired",
            ),
        ),
        (
            "callback-counter-overflow",
            CustodyState::Quarantined,
            TerminalDisposition::Quarantined,
            w::registry_state(
                "fn begin_callback",
                "ManagedSqliteRegistryTerminalReason::CallbackCounterOverflow",
            ),
        ),
    ] {
        let adapter = graph.decision(&format!("{prefix}.{variant}.adapter"), witness);
        let terminal_id = format!("{prefix}.{variant}.terminal");
        let mut counts = ObservableCounts::default();
        counts.callback_begin = 1;
        let mut value = expected::unavailable(
            "CallbackAdmission",
            FailureClass::RegistryRejected,
            MutationState::None,
            disposition,
            counts,
        );
        value.route = route;
        value.callback = CustodyState::NotReached;
        value.file = CustodyState::Unchanged;
        graph.terminal(&terminal_id, value, w::abi_failure());
        graph.edge(from, &adapter, DecisionStage::CallbackAdmission, variant);
        graph.edge(
            &adapter,
            &terminal_id,
            DecisionStage::AbiProjection,
            "map_unavailable",
        );
    }

    for (variant, proof, witness) in [
        (
            "identity-mismatch",
            ExclusionProof::TypeInvariant(
                "route tokens are never reused and a pinned file retains the immutable handle minted for its exact live route",
            ),
            w::registry_owner(
                "fn exact_entry_mut",
                "ManagedSqliteRegistryRouteRejection::IdentityMismatch",
            ),
        ),
        (
            "state-shape-invalid",
            ExclusionProof::TypeInvariant(
                "registry session fields are private and every production transition preserves shape before a pinned-file callback can enter",
            ),
            w::registry_state("fn begin_callback", "self.ensure_shape()?;"),
        ),
        (
            "terminal-phase",
            ExclusionProof::ControlFlow(
                "the apply_route error path removes and permanently retains every route whose state entered TerminalQuarantine",
            ),
            w::registry_state(
                "fn begin_callback",
                "ManagedSqliteRegistryTransitionRejection::Terminal",
            ),
        ),
        (
            "wrong-phase",
            ExclusionProof::TypeInvariant(
                "a callable pinned WAL-main file exists only in Opening, Active or Closing, all of which admit Shm callbacks",
            ),
            w::registry_state(
                "fn begin_callback",
                "ManagedSqliteRegistryTransitionRejection::WrongPhase",
            ),
        ),
    ] {
        let excluded = graph.excluded(&format!("{prefix}.excluded.{variant}"), proof, witness);
        graph.edge(
            from,
            &excluded,
            DecisionStage::CallbackAdmission,
            variant,
        );
    }
}

pub(super) fn managed_success(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    mapped: bool,
    preexisting_mapping: bool,
    mutation: MutationState,
    dms_lock: DmsLockCustody,
    mut counts: ObservableCounts,
) {
    counts.callback_begin = 1;
    counts.callback_complete = 1;
    let completion = graph.decision(
        &format!("{prefix}.callback-completion"),
        w::registry("fn with_shm<T>", "match (result, callback.complete())"),
    );
    graph.edge(
        from,
        &completion,
        DecisionStage::CallbackCompletion,
        "managed_success",
    );

    let adapter = graph.decision(
        &format!("{prefix}.adapter-success"),
        w::adapter(if mapped {
            "ManagedSqliteShmMapOutcome::Mapped(pointer)"
        } else {
            "ManagedSqliteShmMapOutcome::NotPresent"
        }),
    );
    graph.edge(
        &completion,
        &adapter,
        DecisionStage::CallbackCompletion,
        "completion_succeeded",
    );
    if mapped {
        exclusions::adapter(graph, &adapter, prefix);
    }
    let terminal = format!("{prefix}.terminal.success");
    let mut value = expected::success(mapped, mutation, counts);
    value.dms_lock = dms_lock;
    if !mapped && preexisting_mapping {
        value.mapping = CustodyState::Retained;
        value.view = CustodyState::Retained;
    }
    graph.terminal(&terminal, value, w::abi_ok());
    graph.edge(
        &adapter,
        &terminal,
        DecisionStage::AbiProjection,
        if mapped {
            "write_non_null_pointer"
        } else {
            "preserve_null_output"
        },
    );

    exclusions::normal_completion(graph, &completion, prefix);
    for (variant, route) in [(
        "completion-rejected-route-already-quarantined",
        CustodyState::Quarantined,
    )] {
        let rejected = graph.decision(
            &format!("{prefix}.{variant}.adapter"),
            w::process_owner("fn finish_callback(", "self.apply_route_retaining_failure("),
        );
        graph.edge(
            &completion,
            &rejected,
            DecisionStage::CallbackCompletion,
            variant,
        );
        let terminal = format!("{prefix}.terminal.{variant}");
        let mut value = expected::unavailable(
            "CallbackCompletion",
            FailureClass::RegistryRejected,
            mutation,
            TerminalDisposition::Quarantined,
            counts,
        );
        value.route = route;
        value.callback = CustodyState::Retained;
        value.mapping = if mapped || preexisting_mapping {
            CustodyState::Retained
        } else {
            CustodyState::Unchanged
        };
        value.view = value.mapping;
        value.payload = CustodyState::Released;
        graph.terminal(&terminal, value, w::abi_failure());
        graph.edge(
            &rejected,
            &terminal,
            DecisionStage::AbiProjection,
            "success_payload_dropped_then_unavailable",
        );
    }
}

fn safe_failure(graph: &mut MapGraphBuilder, from: &str, prefix: &str, mut spec: FailureSpec) {
    spec.counts.callback_begin = 1;
    spec.counts.callback_complete = 1;
    let completion = graph.decision(
        &format!("{prefix}.callback-completion"),
        w::registry("fn with_shm<T>", "match (result, callback.complete())"),
    );
    graph.edge(
        from,
        &completion,
        DecisionStage::CallbackCompletion,
        "operation_error",
    );
    exclusions::normal_completion(graph, &completion, prefix);
    for (variant, route, callback) in [
        (
            "completion-succeeded",
            CustodyState::Unchanged,
            CustodyState::Released,
        ),
        (
            "completion-rejected-route-already-quarantined",
            CustodyState::Quarantined,
            CustodyState::Retained,
        ),
    ] {
        add_failure_terminal(
            graph,
            &completion,
            &format!("{prefix}.{variant}"),
            spec,
            route,
            callback,
            variant,
        );
    }
}

fn unsafe_failure(graph: &mut MapGraphBuilder, from: &str, prefix: &str, mut spec: FailureSpec) {
    spec.counts.callback_begin = 1;
    spec.counts.callback_complete = 1;
    // Production leaks the marker before attempting the route transition, so physical failure
    // evidence is retained for every success and rejection of registry quarantine.
    spec.payload = CustodyState::Retained;
    let quarantine = graph.decision(
        &format!("{prefix}.quarantine"),
        w::registry(
            "fn quarantine_unsafe_shm_failure",
            "let _ = self.owner.retain_terminal_custody(",
        ),
    );
    graph.edge(
        from,
        &quarantine,
        DecisionStage::Quarantine,
        "unsafe_failure_requires_retention",
    );
    let retained = graph.decision(
        &format!("{prefix}.retention-succeeded.completion"),
        w::registry("fn with_shm<T>", "match (result, callback.complete())"),
    );
    graph.edge(
        &quarantine,
        &retained,
        DecisionStage::Quarantine,
        "retention_succeeded",
    );
    exclusions::unsafe_completion_success(
        graph,
        &retained,
        &format!("{prefix}.retention-succeeded"),
        "successful retention removes the exact route, so completing its still-live callback lease must reject UnknownOrRetired",
    );
    add_failure_terminal(
        graph,
        &retained,
        &format!("{prefix}.retention-succeeded.completion-rejected"),
        spec,
        CustodyState::Quarantined,
        CustodyState::Retained,
        "completion_rejected_route_already_quarantined",
    );

    let identity_mismatch = graph.excluded(
        &format!("{prefix}.retention-rejected.excluded.identity-mismatch"),
        ExclusionProof::TypeInvariant(
            "route tokens are never reused and unsafe failure retention uses the immutable route carried by the same pinned file",
        ),
        w::registry_owner(
            "fn exact_entry(",
            "ManagedSqliteRegistryRouteRejection::IdentityMismatch",
        ),
    );
    graph.edge(
        &quarantine,
        &identity_mismatch,
        DecisionStage::Quarantine,
        "retention_rejected_identity_mismatch",
    );

    exclude_owner_poison(
        graph,
        &quarantine,
        &format!("{prefix}.retention-rejected.owner-poisoned"),
        DecisionStage::Quarantine,
        "retention_rejected_owner_poisoned",
    );
    let rejected = graph.decision(
        &format!("{prefix}.retention-rejected-route-unknown.completion"),
        w::registry("fn with_shm<T>", "match (result, callback.complete())"),
    );
    graph.edge(
        &quarantine,
        &rejected,
        DecisionStage::Quarantine,
        "retention_rejected_route_already_absent",
    );
    exclusions::unsafe_completion_success(
        graph,
        &rejected,
        &format!("{prefix}.retention-rejected-route-unknown"),
        "the exact route was already absent and route tokens are never reused, so immediate callback completion must reject UnknownOrRetired",
    );
    add_failure_terminal(
        graph,
        &rejected,
        &format!(
            "{prefix}.retention-rejected-route-unknown.completion-rejected-route-already-quarantined"
        ),
        spec,
        CustodyState::Quarantined,
        CustodyState::Retained,
        "completion-rejected-route-already-quarantined",
    );
}

pub(super) fn exclude_owner_poison(
    graph: &mut MapGraphBuilder,
    from: &str,
    id: &str,
    stage: DecisionStage,
    branch: &str,
) {
    let excluded = graph.excluded(
        id,
        poison::owner_mutex_poison_proof(),
        w::process_owner(
            "fn lock_routes",
            "ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned",
        ),
    );
    graph.edge(from, &excluded, stage, branch);
}

fn add_failure_terminal(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    spec: FailureSpec,
    route: CustodyState,
    callback: CustodyState,
    branch: &str,
) {
    let adapter = graph.decision(
        &format!("{prefix}.adapter"),
        w::adapter("self.file.shm_map(region, region_size, mode).map_err(drop)?"),
    );
    graph.edge(from, &adapter, DecisionStage::CallbackCompletion, branch);
    let terminal = format!("{prefix}.terminal");
    let mut value = expected::unavailable(
        spec.phase,
        spec.failure,
        spec.mutation,
        spec.disposition,
        spec.counts,
    );
    value.route = route;
    value.callback = callback;
    value.file = spec.file;
    value.mapping = spec.mapping;
    value.view = spec.view;
    value.payload = spec.payload;
    value.lock_outcome_uncertain = spec.lock_outcome_uncertain;
    value.dms_lock = spec.dms_lock;
    if callback == CustodyState::Retained && value.disposition == TerminalDisposition::Returned {
        value.disposition = TerminalDisposition::Quarantined;
    }
    graph.terminal(&terminal, value, w::abi_failure());
    graph.edge(
        &adapter,
        &terminal,
        DecisionStage::AbiProjection,
        "operation_error_wins",
    );
}
