const LOCAL_AUTHORITY_ROOT: &str =
    include_str!("../../node_agent_compute_plugin_host/local_authority.rs");
const OPENED_AUTHORITY: &str =
    include_str!("../../node_agent_compute_plugin_host/local_authority/opened_authority.rs");
const CURRENTNESS: &str = include_str!(
    "../../node_agent_compute_plugin_host/local_authority/ready_source_currentness.rs"
);
const WORK_ADMISSION_ROOT: &str =
    include_str!("../../node_agent_compute_plugin_host/local_authority/work_admission_store.rs");
const WORK_ADMISSION_PLANNING: &str = include_str!(
    "../../node_agent_compute_plugin_host/local_authority/work_admission_store/planning.rs"
);
const WORK_ADMISSION_CURRENT: &str = include_str!(
    "../../node_agent_compute_plugin_host/local_authority/work_admission_store/current.rs"
);
const WORK_ADMISSION_CAPABILITY: &str =
    include_str!("../../node_agent_compute_plugin_host/work_admission_contract/capability.rs");
const READY: &str = include_str!("../../node_agent_compute_plugin_host/ready_capability.rs");
const BASE_LINEAGE: &str = include_str!("../user_node_ready_source_lineage/types.rs");
const AUTHORITY: &str = include_str!(
    "../../../../docs/distributed-compute/user-node-ready-local-currentness-authority.md"
);
const ACCEPTANCE: &str = include_str!(
    "../../../../docs/distributed-compute/user-node-ready-local-currentness-acceptance.md"
);

#[test]
fn currentness_is_handle_bound_query_only_and_transaction_scoped() {
    assert!(
        LOCAL_AUTHORITY_ROOT
            .lines()
            .any(|line| line.trim() == "mod ready_source_currentness;"),
        "currentness route must stay private"
    );
    assert!(
        !LOCAL_AUTHORITY_ROOT.contains("CurrentComputeUserNodeReadySourceLineageSeal"),
        "private currentness seal must not be re-exported from local authority"
    );
    for marker in [
        "struct CurrentComputeUserNodeReadySourceLineageSeal",
        "impl OpenedComputePluginLocalAuthority",
        "with_current_user_node_ready_source_lineage",
        "with_deferred_read",
        "for<'snapshot> FnOnce",
        "PhantomData<Rc<&'snapshot ()>>",
        "process_fence.ensure_process_owner_current()",
        "fresh_time.ensure_live(Instant::now())",
        "verify_schema_v8_read_only(transaction)",
        "authority_witness.ensure_unchanged_on(transaction)",
        "catch_unwind(AssertUnwindSafe",
        "resume_unwind(payload)",
    ] {
        assert!(
            CURRENTNESS.contains(marker),
            "missing currentness fence {marker}"
        );
    }
    for marker in [
        "pragma_update(None, \"query_only\", true)",
        "transaction_with_behavior(TransactionBehavior::Deferred)",
        "COMPUTE_PLUGIN_OPENED_AUTHORITY_READ_COMMIT",
        "COMPUTE_PLUGIN_OPENED_AUTHORITY_QUERY_ONLY_RESTORE",
    ] {
        assert!(
            OPENED_AUTHORITY.contains(marker),
            "missing Deferred/query-only owner marker {marker}"
        );
    }
    for forbidden in [
        "with_immediate",
        "INSERT INTO",
        "UPDATE ",
        "DELETE FROM",
        "#[derive(Clone",
        "Serialize",
        "Deserialize",
        "pub fn new(",
    ] {
        assert!(
            !CURRENTNESS.contains(forbidden),
            "currentness opened a forbidden seam: {forbidden}"
        );
    }

    let seal_definition_start = CURRENTNESS
        .find("struct CurrentComputeUserNodeReadySourceLineageSeal")
        .expect("missing seal definition");
    let seal_definition = &CURRENTNESS[seal_definition_start..];
    let seal_definition_end = seal_definition
        .find("\n}\n")
        .expect("unterminated seal definition");
    let seal_definition = &seal_definition[..seal_definition_end];
    assert!(
        seal_definition
            .lines()
            .filter(|line| line.trim_end().ends_with(','))
            .all(|line| !line.trim_start().starts_with("pub")),
        "seal fields must stay private"
    );
    assert!(
        CURRENTNESS
            .lines()
            .all(|line| !line.trim_start().starts_with("pub")),
        "currentness file gained an unreviewed visible item or constructor"
    );
    assert!(
        CURRENTNESS
            .lines()
            .all(|line| !line.trim_start().starts_with("mod ")),
        "private currentness prover must not gain an unreviewed descendant module"
    );
    assert_eq!(
        CURRENTNESS
            .matches("with_current_user_node_ready_source_lineage")
            .count(),
        1,
        "private currentness prover gained a call site or duplicate definition"
    );
    let prover_start = CURRENTNESS
        .find("fn with_current_user_node_ready_source_lineage(")
        .expect("missing private currentness prover");
    let prover = &CURRENTNESS[prover_start..];
    let prover_body = prover
        .find(" {\n")
        .expect("unterminated currentness prover signature");
    let prover_signature = prover[..prover_body].trim_end();
    assert!(
        prover_signature.ends_with(") -> Result<()>") && !prover_signature.contains("<R>"),
        "currentness prover must stay private and return only unit"
    );
}

#[test]
fn currentness_reaudits_exact_work_admission_and_ready_successor() {
    for marker in [
        "read_current_work_admission_head_pair_on",
        "read_pair_required",
        "validate_head_pair",
        "validate_predecessor_chain",
        "validate_current_installed_receipts",
        "COMPUTE_PLUGIN_READY_CURRENT_WORK_ADMISSION_HEAD_CHANGED",
    ] {
        assert!(
            WORK_ADMISSION_PLANNING.contains(marker)
                || WORK_ADMISSION_ROOT.contains(marker)
                || WORK_ADMISSION_CURRENT.contains(marker),
            "missing work-admission audit {marker}"
        );
    }
    for marker in [
        "revalidate_ready_publication_at_current_authority",
        "fresh_time.observed_at() <= prior_time.observed_at()",
        "fresh_time.trusted_now() <= prior_time.trusted_now()",
        "current_inventory.inventory_revision != publication.inventory_revision()",
        "current_record != publication.record()",
        "validate_ready_record(current_record, fresh_time.trusted_now())",
    ] {
        assert!(READY.contains(marker), "missing Ready reproof {marker}");
    }
    for marker in [
        "fn trusted_time(",
        "fn revalidated_at(&self) -> Instant",
        "self.revalidated.trusted_time()",
        "self.revalidated.revalidated_at()",
    ] {
        assert!(
            WORK_ADMISSION_CAPABILITY.contains(marker),
            "missing retained admission barrier {marker}"
        );
    }
    for marker in [
        "authority.state_revision <= transition.authority_state_revision_after()",
        "authority.inventory.inventory_revision <= transition.inventory_revision_after()",
        "authority.process_owner_epoch != transition.process_owner_epoch()",
        "current_authorization_matches",
        "authority.node_profile_digest != plan.node_profile_digest()",
        "authority.manifest_catalog_revision != plan.manifest_catalog_revision()",
        "authority.keyring_bundle_revision != plan.keyring_bundle_revision()",
        "authority.target_id != source.launch_profile().target_id()",
        "witness.updated_at_ms != authority.trusted_time_high_water_ms",
        "fresh_time.trusted_now().timestamp_millis() <= authority.trusted_time_high_water_ms",
        "authority.publisher_keyring == authority.control_keyring",
        "fresh_time.observed_at() <= admitted.revalidated_at()",
        "project_user_node_ready_source_lineage",
    ] {
        assert!(
            CURRENTNESS.contains(marker),
            "missing successor fence {marker}"
        );
    }
}

#[test]
fn serialized_projection_keeps_all_gaps_and_currentness_has_zero_effects() {
    for marker in [
        "node_local_authority_currentness",
        "runtime_transition_authority",
        "host_runtime_authority",
        "v15_authenticated_session",
    ] {
        assert!(BASE_LINEAGE.contains(marker), "base gap changed {marker}");
        assert!(AUTHORITY.contains(marker), "authority omitted gap {marker}");
    }
    for marker in [
        "migration/table/writer=none/none/none",
        "Provider",
        "Offer",
        "Job",
        "Lease",
        "Receipt",
        "implementation_uncompiled",
        "implementation_unrun",
        "passed=0/failed=0",
    ] {
        assert!(
            AUTHORITY.contains(marker) || ACCEPTANCE.contains(marker),
            "missing zero-effect/status marker {marker}"
        );
    }
    for forbidden in [
        "produce_compute_attempt_execution_plan",
        "VerifiedComputeExecutionCapability",
        "ComputeReadyCapability {",
        "planning_snapshot_bootstrap_only + Ready",
    ] {
        assert!(
            !CURRENTNESS.contains(forbidden),
            "currentness gained downstream authority: {forbidden}"
        );
    }
}
