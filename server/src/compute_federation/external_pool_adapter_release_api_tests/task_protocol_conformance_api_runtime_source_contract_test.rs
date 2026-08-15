const API: &str = include_str!("../external_pool_adapter_task_protocol_conformance_api.rs");
const SERVICE: &str = include_str!("../external_pool_adapter_task_protocol_conformance_service.rs");
const VALIDATION: &str =
    include_str!("../external_pool_adapter_task_protocol_conformance_service_validation.rs");
const REDACTION: &str =
    include_str!("../external_pool_adapter_task_protocol_conformance_service_redaction.rs");
const RUNTIME: &str =
    include_str!("../../store/compute_external_pool_adapter_task_protocol_conformance/runtime.rs");
const STORE_FACADE: &str =
    include_str!("../../store/compute_external_pool_adapter_task_protocol_conformance.rs");
const STORE_ROOT: &str = include_str!("../../store.rs");
const PROJECT_AUTH: &str = include_str!("../../project_auth.rs");
const STARTUP: &str = include_str!("../../node_endpoint_session_startup.rs");
const COMPUTE_MOD: &str = include_str!("../mod.rs");
const RELEASE_API: &str = include_str!("../external_pool_adapter_release_api.rs");

const RUNS_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/task-protocol-conformance-runs";
const CURRENTNESS_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/task-protocol-conformance-runs/currentness";
const REVOCATION_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/task-protocol-conformance-runs/:run_receipt_id/revoke";

#[test]
fn task_protocol_conformance_api_freezes_three_admin_routes_and_statuses() {
    assert_eq!(API.matches(".route(").count(), 3);
    for (constant, path, method) in [
        ("RUNS_PATH", RUNS_PATH, "post(create)"),
        ("CURRENTNESS_PATH", CURRENTNESS_PATH, "get(currentness)"),
        ("REVOCATION_PATH", REVOCATION_PATH, "post(revoke)"),
    ] {
        assert!(API.contains(path), "route path drifted: {path}");
        let routes = source_block(API, "pub(crate) fn routes()", "async fn create(");
        assert!(routes.contains(&format!(".route({constant}, {method})")));
    }
    assert!(!API.contains("/api/me"));
    assert!(API.contains("user.id == \"local-owner\""));
    assert!(API.contains("only durable platform administrators"));
    assert_ordered(
        source_block(API, "async fn create(", "async fn currentness("),
        &["platform_admin", "json_body", "service::create", ".await"],
    );
    assert_ordered(
        source_block(API, "async fn currentness(", "async fn revoke("),
        &["platform_admin", "service::currentness"],
    );
    assert_ordered(
        source_block(API, "async fn revoke(", "fn platform_admin("),
        &["platform_admin", "json_body", "service::revoke"],
    );
    assert_eq!(API.matches("platform_admin(&state, &headers)").count(), 3);
    let virtual_owner = source_block(
        PROJECT_AUTH,
        "fn make_local_owner() -> PublicUser",
        "pub fn project_access(",
    );
    assert!(virtual_owner.contains("id: \"local-owner\".to_string()"));
    assert!(virtual_owner.contains("role: \"owner\".to_string()"));
    for status in [
        "StatusCode::UNAUTHORIZED",
        "StatusCode::FORBIDDEN",
        "StatusCode::UNPROCESSABLE_ENTITY",
        "StatusCode::BAD_REQUEST",
        "StatusCode::NOT_FOUND",
        "StatusCode::CONFLICT",
        "StatusCode::SERVICE_UNAVAILABLE",
        "StatusCode::INTERNAL_SERVER_ERROR",
        "StatusCode::CREATED",
        "StatusCode::OK",
    ] {
        assert!(API.contains(status), "HTTP classification lost {status}");
    }
}

#[test]
fn task_protocol_conformance_api_freezes_strict_caller_shape() {
    assert_eq!(
        VALIDATION.matches("#[serde(deny_unknown_fields)]").count(),
        3
    );
    let create = struct_block(VALIDATION, "CreateTaskProtocolConformanceRunBody");
    assert_eq!(
        create
            .lines()
            .filter(|line| line.trim().starts_with("pub "))
            .count(),
        14
    );
    for field in [
        "expected_registry_release_digest",
        "provider_binding_id",
        "expected_provider_binding_digest",
        "expected_installation_receipt_id",
        "expected_installation_receipt_digest",
        "sandbox_reattestation_receipt_id",
        "expected_sandbox_reattestation_receipt_digest",
        "runtime_compatibility_verification_receipt_id",
        "expected_runtime_compatibility_verification_receipt_digest",
        "expected_task_protocol_profile_digest",
        "expected_fixture_catalog_digest",
        "expected_predecessor",
        "idempotency_key",
        "confirm_task_protocol_conformance_run",
    ] {
        assert!(create.contains(field), "create body lost {field}");
    }
    for forbidden in [
        "expected_registry_release_material_digest",
        "vulnerability_reattestation_receipt_id",
        "actor",
        "idempotency_scope",
        "checked_at",
        "expires_at",
        "run_nonce",
        "observation",
        "transcript",
        "hmac",
        "effect",
        "readiness",
        "result",
        "cgroup",
        "path",
    ] {
        assert!(
            !create.contains(forbidden),
            "caller body gained {forbidden}"
        );
    }
    let revoke = struct_block(VALIDATION, "RevokeTaskProtocolConformanceRunBody");
    assert_eq!(
        revoke
            .lines()
            .filter(|line| line.trim().starts_with("pub "))
            .count(),
        4
    );
    for field in [
        "expected_run_receipt_digest",
        "reason",
        "idempotency_key",
        "confirm_revocation",
    ] {
        assert!(revoke.contains(field), "revoke body lost {field}");
    }
}

#[test]
fn task_protocol_conformance_service_freezes_gate_before_objects_and_blocking_run() {
    let create = source_block(
        SERVICE,
        "pub(crate) async fn create(",
        "pub(crate) fn currentness(",
    );
    assert_ordered(
        create,
        &[
            "validate_create(",
            "external_pool_adapter_task_protocol_conformance_runtime()",
            "require_release_and_receipts(",
            "external_pool_adapter_registry_provider_binding_audit_target(",
            "tokio::task::spawn_blocking(move ||",
            "audit_external_pool_adapter_installation(",
            "create_external_pool_adapter_task_protocol_conformance_run(",
        ],
    );
    let current = source_block(
        SERVICE,
        "pub(crate) fn currentness(",
        "pub(crate) fn revoke(",
    );
    assert_ordered(
        current,
        &[
            "validate_currentness(",
            "external_pool_adapter_task_protocol_conformance_runtime()",
            "external_pool_adapter_registry_release_exists(",
            "external_pool_adapter_task_protocol_conformance_currentness(",
        ],
    );
    let revoke = source_block(
        SERVICE,
        "pub(crate) fn revoke(",
        "fn require_release_and_receipts(",
    );
    assert_ordered(
        revoke,
        &[
            "validate_revoke(",
            "external_pool_adapter_task_protocol_conformance_runtime()",
            "external_pool_adapter_registry_release_exists(",
            "external_pool_adapter_task_protocol_conformance_run_exists(",
            "revoke_external_pool_adapter_task_protocol_conformance_run(",
        ],
    );
    assert!(SERVICE.contains("idempotency_scope(\"create\""));
    assert!(SERVICE.contains("idempotency_scope(\"revoke\""));
    assert!(!SERVICE.contains("TaskProtocolConformanceRunEvidence {"));
}

#[test]
fn task_protocol_conformance_runtime_freezes_independent_default_off_custody() {
    for environment in [
        "ELON_EXTERNAL_POOL_ADAPTER_TASK_PROTOCOL_CONFORMANCE_ENABLED",
        "ELON_EXTERNAL_POOL_ADAPTER_TASK_PROTOCOL_CONFORMANCE_CGROUP_PARENT_PATH",
    ] {
        assert_eq!(RUNTIME.matches(environment).count(), 1);
    }
    assert_eq!(RUNTIME.matches("std::env::var_os(").count(), 2);
    assert!(RUNTIME.contains("static TASK_PROTOCOL_CONFORMANCE_RUNTIME: OnceLock<"));
    assert!(RUNTIME.contains("Option<Arc<ExternalPoolAdapterTaskProtocolConformanceRuntime>>"));
    assert!(RUNTIME.contains("generate_task_protocol_conformance()?"));
    let configured = source_block(
        RUNTIME,
        "fn configured_runtime()",
        "fn required_absolute_path(",
    );
    assert_ordered(
        configured,
        &[
            "std::env::var_os(ENABLED_ENV)",
            "None => false",
            "Some(\"true\") => true",
            "Some(\"false\") => false",
            "std::env::var_os(CGROUP_PARENT_PATH_ENV)",
            "if !enabled",
            "if cgroup_path.is_some()",
            "return Ok(None)",
            "required_absolute_path(cgroup_path)?",
            "from_operator_delegated_path(&cgroup_path)?",
            "generate_task_protocol_conformance()?",
        ],
    );
    let required_path = source_block(RUNTIME, "fn required_absolute_path(", "}");
    assert_ordered(
        required_path,
        &[
            ".filter(|value| !value.is_empty())",
            "enabled task protocol conformance lacks its cgroup path",
            "if !path.is_absolute()",
            "task protocol conformance cgroup path is not absolute",
        ],
    );
    assert!(RUNTIME.contains("task protocol conformance requires Linux x86-64"));
    for forbidden in [
        "RUNTIME_COMPATIBILITY_SIGNING_HANDOFF_ENABLED",
        "PROVIDER_RUNTIME_READINESS_ENABLED",
        "BUNDLE_ROOT_PATH",
    ] {
        assert!(
            !RUNTIME.contains(forbidden),
            "runtime coupled to {forbidden}"
        );
    }
    assert!(
        STARTUP.contains("initialize_external_pool_adapter_task_protocol_conformance_runtime()?")
    );
    assert!(STORE_FACADE.contains("external_pool_adapter_task_protocol_conformance_runtime"));
    assert!(STORE_ROOT.contains(
        "pub(crate) use compute_external_pool_adapter_task_protocol_conformance::api::*;"
    ));
    assert!(COMPUTE_MOD.contains("external_pool_adapter_task_protocol_conformance_api"));
    assert_eq!(
        RELEASE_API
            .matches("external_pool_adapter_task_protocol_conformance_api::routes()")
            .count(),
        1
    );
}

#[test]
fn task_protocol_conformance_api_freezes_recursive_private_redaction() {
    assert!(REDACTION.contains("map.values_mut().for_each(redact)"));
    assert!(REDACTION.contains("Value::Array(values) => values.iter_mut().for_each(redact)"));
    for private in [
        "runtime_custody_epoch_digest",
        "process_hmac_seal",
        "receipt_integrity_digest",
        "provider_binding_id",
        "installation_receipt_id",
        "recorded_by_admin_user_id",
        "revoked_by_admin_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "raw_transcript",
        "config_bytes",
        "credential_bytes",
        "cgroup_path",
        "pidfd",
    ] {
        assert!(REDACTION.contains(private), "redaction lost {private}");
    }
}

fn struct_block<'a>(source: &'a str, name: &str) -> &'a str {
    source
        .split_once(&format!("struct {name} {{"))
        .unwrap()
        .1
        .split_once("}\n")
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
