use super::{
    super::{
        super::{
            model::{
                CustodyState, DecisionStage, ExclusionProof, SqliteResult, TerminalDisposition,
            },
            source::{witness, ProductionOwner, SourceWitness},
        },
        builder::Builder,
        dynamic::TerminalPathV1,
    },
    completion::{add_invariant_exclusions, owner_poisoned_witness, route_unknown_witness},
    projection::{adapter_projection, add_abi_terminal, expected},
    registry_operations, Shape,
};

#[derive(Debug, Clone, Copy)]
enum Retention {
    Succeeded,
    RouteUnknown,
}

impl Retention {
    fn label(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::RouteUnknown => "route-unknown-prior-quarantine",
        }
    }

    fn route(self) -> CustodyState {
        match self {
            Self::Succeeded | Self::RouteUnknown => CustodyState::Quarantined,
        }
    }

    fn witness(self) -> SourceWitness {
        match self {
            Self::Succeeded => witness(
                ProductionOwner::RegistryOwner,
                "pub(super) fn quarantine",
                "Self::retain_terminal(entry);",
                1,
            ),
            Self::RouteUnknown => witness(
                ProductionOwner::RegistryOwner,
                "fn exact_entry(",
                "return Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired);",
                1,
            ),
        }
    }
}

pub(super) fn expand(builder: &mut Builder, from: &str, prefix: &str, shape: Shape) {
    let gate = builder.decision(
        format!("{prefix}.unsafe-retention"),
        registry_operations(
            "fn quarantine_unsafe_shm_failure",
            "let _ = self.owner.retain_terminal_custody",
        ),
    );
    builder.edge(
        from,
        &gate,
        DecisionStage::Quarantine,
        "unsafe_failure_requires_retention",
    );

    for retention in [Retention::Succeeded, Retention::RouteUnknown] {
        expand_retention_result(builder, &gate, prefix, shape, retention);
    }
    let owner_poisoned = builder.excluded(
        format!("{prefix}.retention.excluded.owner-poisoned"),
        super::super::super::poison::owner_mutex_poison_proof(),
        owner_poisoned_witness(),
    );
    builder.edge(
        &gate,
        &owner_poisoned,
        DecisionStage::Quarantine,
        "owner-poisoned",
    );
    add_retention_exclusions(builder, &gate, prefix);
}

fn expand_retention_result(
    builder: &mut Builder,
    gate: &str,
    prefix: &str,
    shape: Shape,
    retention: Retention,
) {
    let retention_prefix = format!("{prefix}.retention.{}", retention.label());
    let retained = builder.continuation(
        format!("{retention_prefix}.outcome"),
        "unsafe failure marker retention result",
        retention.witness(),
    );
    builder.edge(
        gate,
        &retained,
        DecisionStage::Quarantine,
        retention.label(),
    );
    let completion = builder.decision(
        format!("{retention_prefix}.callback-completion"),
        registry_operations("fn with_shm<T>", "match (result, callback.complete())"),
    );
    builder.edge(
        &retained,
        &completion,
        DecisionStage::CallbackCompletion,
        "retention_result_precedes_callback_completion",
    );

    exclude_completion_success(builder, &completion, &retention_prefix, retention);
    add_invariant_exclusions(
        builder,
        &completion,
        &format!("{retention_prefix}.completion"),
    );
    let owner_poisoned = builder.excluded(
        format!("{retention_prefix}.completion.excluded.owner-poisoned"),
        super::super::super::poison::owner_mutex_poison_proof(),
        owner_poisoned_witness(),
    );
    builder.edge(
        &completion,
        &owner_poisoned,
        DecisionStage::CallbackCompletion,
        "owner-poisoned",
    );
    add_unsafe_completion_terminal(
        builder,
        &completion,
        &retention_prefix,
        shape,
        retention,
        "route-unknown",
        route_unknown_witness(),
    );
}

fn add_unsafe_completion_terminal(
    builder: &mut Builder,
    completion: &str,
    prefix: &str,
    shape: Shape,
    retention: Retention,
    completion_branch: &str,
    source: SourceWitness,
) {
    let outcome = builder.continuation(
        format!("{prefix}.completion-outcome.{completion_branch}"),
        "operation-error callback completion rejection",
        source,
    );
    builder.edge(
        completion,
        &outcome,
        DecisionStage::CallbackCompletion,
        completion_branch,
    );
    let mut value = expected(shape);
    value.sqlite = SqliteResult::LockUnavailable;
    value.disposition = if shape.disposition == TerminalDisposition::CleanupRewritten {
        TerminalDisposition::CleanupRewritten
    } else {
        TerminalDisposition::Quarantined
    };
    value.route = retention.route();
    value.callback = CustodyState::Retained;
    value.payload = CustodyState::Retained;
    value.counts.callback_complete = 1;
    let projection = adapter_projection(
        builder,
        &format!("{prefix}.projection.{completion_branch}"),
        SqliteResult::LockUnavailable,
    );
    builder.edge(
        &outcome,
        &projection,
        DecisionStage::CallbackCompletion,
        "operation_error_wins",
    );
    add_abi_terminal(
        builder,
        &projection,
        &format!("{prefix}.terminal.{completion_branch}"),
        value,
        shape.descriptor,
        match retention {
            Retention::Succeeded => TerminalPathV1::UnsafeRetentionSucceededThenRouteUnknown,
            Retention::RouteUnknown => TerminalPathV1::UnsafeRetentionRouteUnknownThenRouteUnknown,
        },
    );
}

fn exclude_completion_success(
    builder: &mut Builder,
    completion: &str,
    prefix: &str,
    retention: Retention,
) {
    let proof = match retention {
        Retention::Succeeded => {
            "successful unsafe retention removes the exact route, so the still-live callback lease cannot complete successfully"
        }
        Retention::RouteUnknown => {
            "the exact route was already absent and cannot reappear because route tokens are never reused, so callback completion cannot succeed"
        }
    };
    let excluded = builder.excluded(
        format!("{prefix}.completion.excluded.succeeded"),
        ExclusionProof::ControlFlow(proof),
        witness(
            ProductionOwner::RegistryState,
            "pub(super) fn finish_callback",
            "Ok(())",
            1,
        ),
    );
    builder.edge(
        completion,
        &excluded,
        DecisionStage::CallbackCompletion,
        "completion_succeeded",
    );
}

fn add_retention_exclusions(builder: &mut Builder, gate: &str, prefix: &str) {
    for (branch, proof, source) in [
        (
            "identity-mismatch",
            ExclusionProof::TypeInvariant(
                "unsafe retention uses the immutable route carried by the same pinned file and route tokens are never reused",
            ),
            witness(
                ProductionOwner::RegistryOwner,
                "fn exact_entry(",
                "return Err(ManagedSqliteRegistryRouteRejection::IdentityMismatch);",
                1,
            ),
        ),
        (
            "validated-route-disappeared",
            ExclusionProof::ControlFlow(
                "exact_entry validation and removal run under one exclusive owner mutex guard, so no route mutation can intervene",
            ),
            witness(
                ProductionOwner::RegistryOwner,
                "pub(super) fn quarantine",
                "expect(\"validated route must remain present under exclusive owner access\")",
                1,
            ),
        ),
    ] {
        let excluded = builder.excluded(
            format!("{prefix}.retention.excluded.{branch}"),
            proof,
            source,
        );
        builder.edge(gate, &excluded, DecisionStage::Quarantine, branch);
    }
}
