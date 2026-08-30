use super::{exclude_owner_poison, w, DecisionStage, ExclusionProof, MapGraphBuilder};

pub(super) fn normal_completion(graph: &mut MapGraphBuilder, completion: &str, prefix: &str) {
    exclude_owner_poison(
        graph,
        completion,
        &format!("{prefix}.excluded.owner-poisoned"),
        DecisionStage::CallbackCompletion,
        "completion-rejected-owner-poisoned",
    );
    for (variant, proof, witness) in [
        (
            "callback-lease-missing",
            ExclusionProof::TypeInvariant(
                "complete consumes a freshly constructed one-shot routed callback whose lease field is Some and cannot previously have been taken",
            ),
            w::process_owner(
                "fn complete(",
                ".expect(\"live routed callback lease must contain state custody\")",
            ),
        ),
        (
            "route-identity-mismatch",
            ExclusionProof::TypeInvariant(
                "route tokens are never reused and callback completion carries the immutable route used to mint this exact lease",
            ),
            w::registry_owner(
                "fn exact_entry_mut",
                "ManagedSqliteRegistryRouteRejection::IdentityMismatch",
            ),
        ),
        (
            "state-shape-invalid",
            ExclusionProof::TypeInvariant(
                "private registry state and every production transition preserve the session shape while the exact callback lease is live",
            ),
            w::registry_state("fn finish_callback", "self.ensure_shape()?;"),
        ),
        (
            "callback-lease-session-invalid",
            ExclusionProof::TypeInvariant(
                "the just-minted linear lease has this session id and contributes one still-outstanding callback until its sole completion",
            ),
            w::registry_state(
                "fn finish_callback",
                "ManagedSqliteRegistryTransitionRejection::SessionIdentityMismatch",
            ),
        ),
    ] {
        let excluded = graph.excluded(
            &format!("{prefix}.excluded.{variant}"),
            proof,
            witness,
        );
        graph.edge(
            completion,
            &excluded,
            DecisionStage::CallbackCompletion,
            variant,
        );
    }
}

pub(super) fn unsafe_completion_success(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    proof: &'static str,
) {
    exclude_owner_poison(
        graph,
        from,
        &format!("{prefix}.excluded.owner-poisoned"),
        DecisionStage::CallbackCompletion,
        "completion-rejected-owner-poisoned",
    );
    let missing = graph.excluded(
        &format!("{prefix}.excluded.callback-lease-missing"),
        ExclusionProof::TypeInvariant(
            "the unsafe operation still owns the same freshly minted one-shot callback lease when completion is attempted",
        ),
        w::process_owner(
            "fn complete(",
            ".expect(\"live routed callback lease must contain state custody\")",
        ),
    );
    graph.edge(
        from,
        &missing,
        DecisionStage::CallbackCompletion,
        "callback_lease_missing",
    );

    let succeeded = graph.excluded(
        &format!("{prefix}.excluded.completion-succeeded"),
        ExclusionProof::ControlFlow(proof),
        w::registry("fn with_shm<T>", "(Err(rejection), _) => Err(rejection)"),
    );
    graph.edge(
        from,
        &succeeded,
        DecisionStage::CallbackCompletion,
        "completion_succeeded",
    );
}

pub(super) fn adapter(graph: &mut MapGraphBuilder, from: &str, prefix: &str) {
    for (suffix, branch, proof, needle) in [
        (
            "region-mismatch",
            "returned_region_mismatch",
            "ManagedSqliteShmRegionPointer::new stores the requested region unchanged",
            "pointer.region() != region",
        ),
        (
            "length-mismatch",
            "returned_length_mismatch",
            "the mapped logical length is constructed from the validated region size",
            "pointer.length() != region_size.get() as usize",
        ),
        (
            "null-pointer",
            "returned_null_pointer",
            "ManagedSqliteShmRegionPointer carries NonNull and cannot project null",
            "NonNull::new(unsafe { pointer.as_mut_ptr() }.cast()).ok_or(())?",
        ),
    ] {
        let id = format!("{prefix}.excluded.{suffix}");
        graph.excluded(
            &id,
            ExclusionProof::TypeInvariant(proof),
            w::adapter(needle),
        );
        graph.edge(from, &id, DecisionStage::Adapter, branch);
    }
}
