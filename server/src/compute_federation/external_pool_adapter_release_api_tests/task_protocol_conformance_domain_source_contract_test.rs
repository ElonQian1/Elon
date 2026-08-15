const FACADE: &str = include_str!("../external_pool_adapter_task_protocol_conformance.rs");
const TYPES: &str = include_str!("../external_pool_adapter_task_protocol_conformance/types.rs");
const EVIDENCE: &str =
    include_str!("../external_pool_adapter_task_protocol_conformance/runtime_evidence.rs");
const CATALOG: &str = include_str!("../external_pool_adapter_task_protocol_conformance/catalog.rs");
const BUILDERS: &str =
    include_str!("../external_pool_adapter_task_protocol_conformance/builders.rs");
const CANONICAL: &str =
    include_str!("../external_pool_adapter_task_protocol_conformance/canonical.rs");
const VALIDATION_ROOTS: &str =
    include_str!("../external_pool_adapter_task_protocol_conformance/validation/roots.rs");
const VALIDATION_RECEIPT: &str =
    include_str!("../external_pool_adapter_task_protocol_conformance/validation/receipt.rs");
const VALIDATION_OBSERVATIONS: &str =
    include_str!("../external_pool_adapter_task_protocol_conformance/validation/observations.rs");
const RUNNER_EXECUTION: &str = include_str!(
    "../../store/compute_external_pool_adapter_task_protocol_conformance/run/execution.rs"
);
const SERVICE: &str = include_str!("../external_pool_adapter_task_protocol_conformance_service.rs");
const API: &str = include_str!("../external_pool_adapter_task_protocol_conformance_api.rs");

const ROOTS: &[&str] = &[
    "supervisor_session_policy_digest",
    "task_protocol_profile_digest",
    "run_nonce_digest",
    "fixture_catalog_digest",
    "registry_release_digest",
    "installation_content_digest",
    "capability_set_digest",
    "sandbox_reattestation_receipt_digest",
    "runtime_compatibility_verification_receipt_digest",
    "source_capsule_sha256",
    "launch_image_sha256",
    "public_fixture_delivery_root",
    "synthetic_fixture_lane_digest",
    "synthetic_fixture_executor_digest",
];

#[test]
fn task_protocol_conformance_domain_freezes_catalog_and_neutral_authority() {
    for module in [
        "mod builders;",
        "mod canonical;",
        "mod catalog;",
        "mod runtime_evidence;",
        "mod types;",
        "mod validation;",
    ] {
        assert!(FACADE.contains(module), "Domain facade lost {module}");
    }
    assert_ordered(
        source_array(CATALOG, "TASK_PROTOCOL_CONFORMANCE_SESSION_ROOT_NAMES"),
        ROOTS,
    );
    for domain in [
        "elon.external_pool_adapter.task_protocol_conformance.session.roots.v1\\0",
        "elon.external_pool_adapter.task_protocol_conformance.session.kdf_salt.v1\\0",
        "elon.external_pool_adapter.task_protocol.request.v1\\0",
        "elon.external_pool_adapter.task_protocol.exchange.v1\\0",
    ] {
        assert_eq!(
            CATALOG.matches(domain).count(),
            1,
            "domain drifted: {domain}"
        );
    }
    for capability in [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ] {
        assert!(CATALOG.contains(capability), "catalog lost {capability}");
    }
    assert!(CATALOG.contains("server_run_observation_required"));
    assert!(!CATALOG.contains("capability_evidence_policy: \"passed_server_run\""));
    assert!(TYPES.contains("server_run_completed_no_production_authority"));
    assert!(TYPES.contains("non_production_no_v213_authority"));
    assert_eq!(
        CATALOG
            .matches("TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into()")
            .count(),
        9
    );
    assert_eq!(CATALOG.matches(": false,").count(), 9);
}

#[test]
fn task_protocol_conformance_domain_freezes_fresh_delivery_and_runtime_lineage() {
    assert!(TYPES.contains("pub runtime_compatibility: ExternalPoolAdapterTaskProtocolConformanceRuntimeCompatibilityRoots"));
    assert!(TYPES.contains("pub public_fixture_delivery_root: String"));
    assert!(BUILDERS.contains("runtime_compatibility: roots.runtime_compatibility"));
    assert!(BUILDERS.contains("evidence.public_fixture_delivery_root"));
    assert!(BUILDERS.contains(
        "evidence.source_capsule_sha256 != roots.runtime_compatibility.source_capsule_sha256"
    ));
    assert!(BUILDERS.contains("evidence.source_capsule_size_bytes"));
    assert!(BUILDERS.contains("roots.runtime_compatibility.source_capsule_size_bytes"));
    assert!(BUILDERS.contains(
        "evidence.launch_image_sha256 != roots.runtime_compatibility.launch_image_sha256"
    ));
    assert!(BUILDERS.contains(
        "evidence.launch_image_size_bytes != roots.runtime_compatibility.launch_image_size_bytes"
    ));
    assert!(VALIDATION_ROOTS.contains("&runtime.public_fixture_delivery_root"));
    assert!(VALIDATION_RECEIPT.contains("digest(&value.public_fixture_delivery_root)?"));
    assert!(VALIDATION_RECEIPT
        .contains("evidence.session_transcript_digest != evidence.session_roots_digest"));
    assert!(VALIDATION_RECEIPT
        .contains("value.session_transcript_digest != value.session_roots_digest"));
    assert!(
        !BUILDERS.contains("public_fixture_delivery_root != runtime.public_fixture_delivery_root")
    );
    assert!(
        !BUILDERS.contains("public_fixture_delivery_root == runtime.public_fixture_delivery_root")
    );
}

#[test]
fn task_protocol_conformance_domain_freezes_canonical_and_runtime_only_evidence() {
    for private in [
        "provider_binding_id",
        "provider_binding_digest",
        "installation_receipt_id",
        "installation_receipt_digest",
        "recorded_by_admin_user_id",
        "idempotency_scope",
        "process_hmac_seal",
    ] {
        assert!(
            !TYPES.contains(private),
            "canonical Domain gained {private}"
        );
        assert!(
            !EVIDENCE.contains(private),
            "runtime evidence gained {private}"
        );
    }
    assert_eq!(
        RUNNER_EXECUTION
            .matches("TaskProtocolConformanceRunEvidence {")
            .count(),
        1
    );
    assert!(!SERVICE.contains("TaskProtocolConformanceRunEvidence {"));
    assert!(!API.contains("TaskProtocolConformanceRunEvidence {"));
    assert!(CANONICAL.contains("task_protocol_conformance_receipt_integrity_digest("));
    assert_ordered(
        source_block(
            CANONICAL,
            "pub(crate) fn task_protocol_conformance_receipt_integrity_digest(",
            "pub(crate) fn task_protocol_conformance_capability_fixture_digest(",
        ),
        &[
            "run_receipt_digest",
            "runtime_custody_epoch_digest",
            "process_hmac_seal",
        ],
    );
}

#[test]
fn task_protocol_conformance_domain_freezes_recovery_and_event_replay_evidence() {
    for field in [
        "commit_uncertainty_state_before",
        "commit_uncertainty_state_after",
        "commit_uncertainty_marker_digest",
        "event_replay_classification",
        "event_replay_batch_count",
        "event_replay_root",
    ] {
        assert!(EVIDENCE.contains(field), "runtime evidence lost {field}");
    }
    for marker in [
        "unknown_after_remote_acceptance",
        "resolved_by_reconcile",
        "exact_duplicate_batch_replay",
        "task_protocol_conformance_commit_uncertainty_marker_digest(three)?",
        "three.commit_uncertainty_marker_digest != four.commit_uncertainty_marker_digest",
        "item.operation_kind == \"idempotent_commit\"",
        "actual.event_replay_root.as_ref() != actual.event_inventory_digest.as_ref()",
    ] {
        let source = format!("{CATALOG}{CANONICAL}{VALIDATION_OBSERVATIONS}");
        assert!(source.contains(marker), "recovery contract lost {marker}");
    }
    assert!(CATALOG.contains(
        "elon.external_pool_adapter.task_protocol_conformance.fixture.commit_uncertainty.v1\\0"
    ));
    assert!(CANONICAL
        .contains("pub(crate) fn task_protocol_conformance_commit_uncertainty_marker_digest("));
    assert!(VALIDATION_OBSERVATIONS
        .contains("two.upstream_response_sha256 != three.upstream_response_sha256"));
}

fn source_array<'a>(source: &'a str, name: &str) -> &'a str {
    source
        .split_once(name)
        .unwrap()
        .1
        .split_once("];")
        .unwrap()
        .0
}

fn source_block<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap()
        .1
        .split_once(end)
        .unwrap()
        .0
}

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}
