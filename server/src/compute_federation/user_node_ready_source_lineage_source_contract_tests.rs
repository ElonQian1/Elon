#[path = "user_node_ready_source_lineage_source_contract_tests/currentness.rs"]
mod currentness;

const DOMAIN_ROOT: &str = include_str!("user_node_ready_source_lineage.rs");
const DOMAIN_TYPES: &str = include_str!("user_node_ready_source_lineage/types.rs");
const DOMAIN_CANONICAL: &str = include_str!("user_node_ready_source_lineage/canonical.rs");
const DOMAIN_VALIDATION: &str = include_str!("user_node_ready_source_lineage/validation.rs");
const SOURCE_INPUTS: &str = include_str!("user_node_ready_source_lineage/source_inputs.rs");
const SOURCE_EQUATIONS: &str = include_str!("user_node_ready_source_lineage/source_equations.rs");
const COMPUTE_MOD: &str = include_str!("mod.rs");
const NODE_HOST_MOD: &str = include_str!("../node_agent_compute_plugin_host/mod.rs");
const NODE_PROJECTION: &str =
    include_str!("../node_agent_compute_plugin_host/ready_source_lineage_projection.rs");
const AUTHORITY: &str =
    include_str!("../../../docs/distributed-compute/user-node-ready-source-lineage-authority.md");
const ACCEPTANCE: &str =
    include_str!("../../../docs/distributed-compute/user-node-ready-source-lineage-acceptance.md");

fn source_block<'a>(source: &'a str, start_marker: &str, end_marker: &str) -> &'a str {
    let start = source
        .find(start_marker)
        .unwrap_or_else(|| panic!("missing source-block start {start_marker}"));
    let tail = &source[start..];
    let end = tail
        .find(end_marker)
        .unwrap_or_else(|| panic!("missing source-block end {end_marker}"));
    &tail[..end]
}

fn source_field_names<'a>(source: &'a str, start_marker: &str, end_marker: &str) -> Vec<&'a str> {
    source_block(source, start_marker, end_marker)
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("pub(crate) ")
                .and_then(|field| field.split_once(':').map(|(name, _)| name))
        })
        .collect()
}

#[test]
fn ready_source_lineage_is_an_untrusted_six_key_projection() {
    for marker in [
        "compute_federation.user_node_ready_source_lineage.v1",
        "user_node_ready_source_lineage_v1",
        "ELON-COMPUTE-USER-NODE-READY-SOURCE-LINEAGE-V1",
        "rfc8785_jcs",
        "sha256",
        "UntrustedComputeUserNodeReadySourceLineageEnvelopeV1",
        "ProjectedComputeUserNodeReadySourceLineageV1",
    ] {
        assert!(DOMAIN_TYPES.contains(marker), "missing ABI marker {marker}");
    }
    let field_names = source_field_names(
        DOMAIN_TYPES,
        "pub(crate) struct UntrustedComputeUserNodeReadySourceLineageEnvelopeV1",
        "#[derive(Debug, Eq, PartialEq)]",
    );
    assert_eq!(
        field_names,
        vec![
            "schema",
            "lineage_kind",
            "lineage_digest",
            "canonicalization",
            "digest_algorithm",
            "lineage",
        ],
        "the untrusted envelope must retain exactly six ordered keys"
    );
    for key in [
        "schema",
        "lineage_kind",
        "lineage_digest",
        "canonicalization",
        "digest_algorithm",
        "lineage",
    ] {
        assert!(DOMAIN_TYPES.contains(key), "missing envelope key {key}");
    }
    assert!(DOMAIN_CANONICAL.contains("digest.update([0])"));
    assert!(DOMAIN_CANONICAL.contains("lineage_digest"));
    assert!(DOMAIN_CANONICAL.contains("canonical_json(&envelope)? == value"));
    assert!(DOMAIN_ROOT.contains("deliberately untrusted"));
}

#[test]
fn shared_source_is_explicitly_wired_into_both_logical_targets() {
    assert!(COMPUTE_MOD.contains("pub(crate) mod user_node_ready_source_lineage;"));
    assert!(COMPUTE_MOD.contains("mod user_node_ready_source_lineage_source_contract_tests;"));
    assert!(NODE_HOST_MOD
        .contains("#[path = \"../compute_federation/user_node_ready_source_lineage.rs\"]"));
    assert!(NODE_HOST_MOD.contains("mod user_node_ready_source_lineage_contract;"));
    assert!(NODE_HOST_MOD.contains("mod ready_source_lineage_projection;"));
    for leaf in [
        "canonical.rs",
        "source_equations.rs",
        "source_inputs.rs",
        "types.rs",
        "validation.rs",
    ] {
        let path = format!("#[path = \"user_node_ready_source_lineage/{leaf}\"]");
        assert!(
            DOMAIN_ROOT.contains(&path),
            "shared root lacks explicit leaf path {path}"
        );
    }
}

#[test]
fn node_projection_consumes_existing_linear_sources_without_minting_ready() {
    for marker in [
        "DurableWorkAdmittedPluginSlot",
        "ValidatedComputeReadyPublication",
        "admitted.receipts()",
        "receipts.source().source()",
        "receipts.receipt().receipt()",
        "source.launch_profile()",
        "ready.record()",
        "ready.trusted_time()",
        "build_compute_user_node_ready_source_lineage",
    ] {
        assert!(
            NODE_PROJECTION.contains(marker),
            "missing node owner source {marker}"
        );
    }
    for forbidden in [
        "ComputeReadyCapability {",
        "HashedComputeReadyCapability {",
        "VerifiedComputeExecutionCapability {",
        "ValidatedComputeAttemptExecutionPlanInputs {",
    ] {
        assert!(
            !NODE_PROJECTION.contains(forbidden),
            "projection must not mint downstream authority {forbidden}"
        );
    }
}

#[test]
fn source_equations_bind_work_admission_health_and_untrusted_host_observation() {
    for marker in [
        "work.installation_identity_digest == ready.installation_identity_digest",
        "work.plugin_id == ready.plugin_id",
        "work.plan_id == ready.last_plan_id",
        "work.plan_policy_revision == ready.desired_policy_revision",
        "work.clock_epoch_digest == ready.trusted_time.clock_epoch_digest",
        "work.slot_ref == ready.slot_ref",
        "work.release == ready.release",
        "work.install_generation == ready.install_generation",
        "work.activation_generation == ready.activation_generation",
        "work.grant_digest == ready.permission_grant_digest",
        "work.runner_digest == ready.runner_digest",
        "ready.runtime_generation > work.runtime_generation_before_ready",
        "ready.inventory_revision > work.inventory_revision",
        "health_observed_at.timestamp_millis() > work.admitted_at_ms",
        "host.runner_digest == work.runner_digest",
        "host.task_kinds == work.task_kinds",
    ] {
        assert!(
            DOMAIN_VALIDATION.contains(marker),
            "missing source equation {marker}"
        );
    }
    assert!(SOURCE_INPUTS.contains("UntrustedComputeUserNodeHostRuntimeObservationDraftV1"));
    assert!(
        DOMAIN_CANONICAL.contains("project_untrusted_compute_user_node_host_runtime_observation")
    );
    assert!(DOMAIN_VALIDATION.contains("canonical_untrusted_host_runtime_observation_digest"));
    assert!(SOURCE_EQUATIONS.contains("validate_compute_user_node_ready_source_lineage"));
    assert!(AUTHORITY.contains("只是结构排序，不是 transition proof"));
}

#[test]
fn cpu_only_and_signed_grant_boundaries_fail_closed() {
    for marker in [
        "observed.cpu_millicores <= granted.max_cpu_millicores",
        "observed.memory_bytes <= granted.max_memory_bytes",
        "observed.vram_bytes <= granted.max_vram_bytes",
        "observed.disk_bytes <= granted.max_disk_bytes",
        "observed.process_count <= granted.max_processes",
        "host.technical_concurrency_limit <= granted.max_processes",
        "observed.accelerator_count == 0 && observed.vram_bytes == 0",
        "accelerator target must explicitly observe a positive accelerator count",
    ] {
        assert!(
            DOMAIN_VALIDATION.contains(marker),
            "missing resource fence {marker}"
        );
    }
    assert!(AUTHORITY.contains("CPU-only"));
    assert!(AUTHORITY.contains("不得虚构 accelerator"));
}

#[test]
fn missing_authorities_and_all_downstream_effects_remain_explicit() {
    let gap_fields = source_field_names(
        DOMAIN_TYPES,
        "pub(crate) struct ComputeUserNodeReadySourceAuthorityGapsV1",
        "#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]",
    );
    assert_eq!(
        gap_fields,
        vec![
            "node_local_authority_currentness",
            "runtime_transition_authority",
            "host_runtime_authority",
            "v15_authenticated_session",
        ],
        "the projection must retain exactly four ordered authority gaps"
    );
    let effect_fields = source_field_names(
        DOMAIN_TYPES,
        "pub(crate) struct ComputeUserNodeReadySourceLineageEffectsV1",
        "#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]",
    );
    assert_eq!(
        effect_fields,
        vec![
            "projection_effect",
            "readiness_effect",
            "provider_effect",
            "route_effect",
            "offer_effect",
            "capacity_effect",
            "execution_effect",
            "lease_effect",
            "settlement_effect",
            "money_effect",
        ],
        "the projection marker plus nine downstream effects must remain exact"
    );
    for marker in [
        "missing_node_currentness_runtime_transition_host_runtime_and_v15_session_authority",
        "untrusted_source_projection_only",
        "node_local_authority_currentness",
        "runtime_transition_authority",
        "host_runtime_authority",
        "v15_authenticated_session",
        "readiness_effect",
        "provider_effect",
        "route_effect",
        "offer_effect",
        "capacity_effect",
        "execution_effect",
        "lease_effect",
        "settlement_effect",
        "money_effect",
    ] {
        assert!(
            DOMAIN_TYPES.contains(marker),
            "missing negative boundary {marker}"
        );
    }
    for gap in [
        "node_local_authority_currentness",
        "runtime_transition_authority",
        "host_runtime_authority",
        "v15_authenticated_session",
    ] {
        assert!(
            SOURCE_EQUATIONS.contains(gap) && DOMAIN_VALIDATION.contains(gap),
            "authority gap is not initialized and validated {gap}"
        );
    }
    assert_eq!(
        SOURCE_EQUATIONS
            .matches("COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING")
            .count(),
        4,
        "all four authority gaps must initialize to missing"
    );
    assert_eq!(
        DOMAIN_VALIDATION
            .matches("COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING")
            .count(),
        4,
        "all four authority gaps must fail closed during validation"
    );
    for effect in [
        "readiness_effect",
        "provider_effect",
        "route_effect",
        "offer_effect",
        "capacity_effect",
        "execution_effect",
        "lease_effect",
        "settlement_effect",
        "money_effect",
    ] {
        assert!(
            SOURCE_EQUATIONS.contains(effect) && DOMAIN_VALIDATION.contains(effect),
            "downstream effect is not initialized and validated {effect}"
        );
    }
    assert_eq!(
        SOURCE_EQUATIONS
            .matches("COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT.to_string()")
            .count(),
        9,
        "all nine downstream effects must initialize to none"
    );
    assert!(SOURCE_EQUATIONS
        .contains("projection_effect: COMPUTE_USER_NODE_READY_SOURCE_PROJECTION_EFFECT"));
    assert!(DOMAIN_VALIDATION
        .contains("effects.projection_effect == COMPUTE_USER_NODE_READY_SOURCE_PROJECTION_EFFECT"));
    assert!(DOMAIN_VALIDATION
        .contains(".all(|effect| effect == COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT)"));
    for source in [
        DOMAIN_ROOT,
        DOMAIN_TYPES,
        DOMAIN_CANONICAL,
        DOMAIN_VALIDATION,
        SOURCE_INPUTS,
        SOURCE_EQUATIONS,
        COMPUTE_MOD,
        NODE_HOST_MOD,
        NODE_PROJECTION,
    ] {
        for forbidden in [
            "rusqlite",
            "axum",
            "INSERT INTO",
            "UPDATE ",
            "DELETE FROM",
            "ComputeOffer",
            "ComputeAttemptLease",
        ] {
            assert!(
                !source.contains(forbidden),
                "source draft crossed forbidden boundary {forbidden}"
            );
        }
    }
    for source in [
        DOMAIN_ROOT,
        DOMAIN_TYPES,
        DOMAIN_CANONICAL,
        DOMAIN_VALIDATION,
        SOURCE_INPUTS,
        SOURCE_EQUATIONS,
        NODE_PROJECTION,
    ] {
        for forbidden in [
            "Store",
            "Migration",
            "migration",
            "Router",
            "http::",
            "tower::",
            "Mcp",
            "MCP",
            "Wire",
            "wire",
            "std::fs",
            "tokio::fs",
            "File::create",
            "OpenOptions",
            "write_all(",
            "ComputeReadyCapability {",
            "HashedComputeReadyCapability {",
            "VerifiedComputeExecutionCapability {",
            "ValidatedComputeAttemptExecutionPlanInputs {",
        ] {
            assert!(
                !source.contains(forbidden),
                "source draft crossed a Store/API/MCP/Wire/write/authority boundary {forbidden}"
            );
        }
    }
    for marker in [
        "implementation_uncompiled",
        "implementation_unrun",
        "passed=0",
        "failed=0",
        "migration/table/writer=none/none/none",
        "unregistered source draft",
    ] {
        assert!(
            ACCEPTANCE.contains(marker),
            "missing evidence boundary {marker}"
        );
    }
}
