use super::{
    super::{
        super::{
            model::{DecisionStage, ExclusionProof},
            source::{witness, ProductionOwner},
        },
        builder::Builder,
    },
    projection::{add_completion_terminal, Completion},
    registry_operations, Shape,
};

pub(super) fn expand(builder: &mut Builder, from: &str, prefix: &str, shape: Shape) {
    let gate = builder.decision(
        format!("{prefix}.callback-completion"),
        registry_operations("fn with_shm<T>", "match (result, callback.complete())"),
    );
    builder.edge(
        from,
        &gate,
        DecisionStage::CallbackCompletion,
        "complete_callback",
    );

    add_completion_terminal(
        builder,
        &gate,
        prefix,
        "completed",
        shape,
        Completion::Completed,
        witness(
            ProductionOwner::RegistryState,
            "pub(super) fn finish_callback",
            "self.callbacks_in_flight -= 1;",
            1,
        ),
    );
    let owner_poisoned = builder.excluded(
        format!("{prefix}.excluded.owner-poisoned"),
        super::super::super::poison::owner_mutex_poison_proof(),
        owner_poisoned_witness(),
    );
    builder.edge(
        &gate,
        &owner_poisoned,
        DecisionStage::CallbackCompletion,
        "owner-poisoned",
    );
    add_completion_terminal(
        builder,
        &gate,
        prefix,
        "route-unknown-prior-quarantine",
        shape,
        Completion::RouteUnknown,
        route_unknown_witness(),
    );
    add_invariant_exclusions(builder, &gate, prefix);
}

pub(super) fn add_invariant_exclusions(builder: &mut Builder, gate: &str, prefix: &str) {
    for (branch, proof, source) in [
        (
            "callback-lease-missing",
            ExclusionProof::TypeInvariant(
                "complete consumes a freshly constructed one-shot routed callback whose private lease is Some and cannot previously have been taken",
            ),
            witness(
                ProductionOwner::RegistryProcessOwner,
                "pub(super) fn complete(mut self)",
                "expect(\"live routed callback lease must contain state custody\")",
                1,
            ),
        ),
        (
            "route-identity-mismatch",
            ExclusionProof::TypeInvariant(
                "route tokens are never reused and callback completion carries the immutable route that minted this exact linear lease",
            ),
            witness(
                ProductionOwner::RegistryOwner,
                "fn exact_entry_mut",
                "return Err(ManagedSqliteRegistryRouteRejection::IdentityMismatch);",
                1,
            ),
        ),
        (
            "state-shape-invalid",
            ExclusionProof::TypeInvariant(
                "private registry state and every production transition preserve session shape while the exact callback lease is live",
            ),
            witness(
                ProductionOwner::RegistryState,
                "fn ensure_shape",
                "Err(ManagedSqliteRegistryTransitionRejection::StateInvariantViolated)",
                1,
            ),
        ),
        (
            "callback-lease-session-or-count-invalid",
            ExclusionProof::TypeInvariant(
                "the just-minted linear lease has this session id and contributes one outstanding callback until its sole consuming completion",
            ),
            witness(
                ProductionOwner::RegistryState,
                "pub(super) fn finish_callback",
                "return Err(ManagedSqliteRegistryTransitionRejection::SessionIdentityMismatch);",
                1,
            ),
        ),
    ] {
        let excluded = builder.excluded(format!("{prefix}.excluded.{branch}"), proof, source);
        builder.edge(
            gate,
            &excluded,
            DecisionStage::CallbackCompletion,
            branch,
        );
    }
}

pub(super) fn owner_poisoned_witness() -> super::super::super::source::SourceWitness {
    witness(
        ProductionOwner::RegistryProcessOwner,
        "fn lock_routes",
        "ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned",
        1,
    )
}

pub(super) fn route_unknown_witness() -> super::super::super::source::SourceWitness {
    witness(
        ProductionOwner::RegistryOwner,
        "fn exact_entry_mut",
        "return Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired);",
        1,
    )
}
