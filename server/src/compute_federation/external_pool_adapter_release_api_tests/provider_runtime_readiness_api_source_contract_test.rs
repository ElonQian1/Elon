const API: &str = include_str!("../external_pool_adapter_provider_runtime_readiness_api.rs");
const DOMAIN_INPUT: &str =
    include_str!("../external_pool_adapter_provider_runtime_readiness/input.rs");
const VALIDATION: &str =
    include_str!("../external_pool_adapter_provider_runtime_readiness_service_validation.rs");
const REDACTION: &str =
    include_str!("../external_pool_adapter_provider_runtime_readiness_service_redaction.rs");

const EXACT_ROOT: &str = "/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/upstream-transport-targets/:target_id/supervisor-session-policy-companions/:companion_id/provider-runtime-readiness-receipts";

#[test]
fn provider_runtime_readiness_api_source_freezes_exact_five_routes() {
    assert!(API.contains(&format!("const READINESS_ROOT: &str = \"{EXACT_ROOT}\";")));
    assert_eq!(API.matches(".route(").count(), 5);
    for required in [
        "{ADMIN_BINDINGS}{READINESS_ROOT}",
        "{ADMIN_BINDINGS}{READINESS_ROOT}/:readiness_receipt_id/currentness",
        "{ADMIN_BINDINGS}{READINESS_ROOT}/:readiness_receipt_id/revocation",
        "{OWNER_BINDINGS}{READINESS_ROOT}/:readiness_receipt_id/currentness",
        "{OWNER_BINDINGS}{READINESS_ROOT}/:readiness_receipt_id/revocation",
        "post(admin_create)",
        "get(admin_currentness)",
        "post(admin_revoke)",
        "get(owner_currentness)",
        "post(owner_revoke)",
    ] {
        assert!(
            API.contains(required),
            "missing exact V270 route {required}"
        );
    }
    for forbidden in [
        "owner_create",
        "owner_trigger",
        "signer-callback",
        "activation-ready",
        "runtime-readiness-verifications",
        "provider-runtime-readiness-verifications",
    ] {
        assert!(!API.contains(forbidden), "forbidden V270 route {forbidden}");
    }
}

#[test]
fn provider_runtime_readiness_api_source_freezes_auth_shape_and_statuses() {
    for required in [
        "JsonRejection",
        "StatusCode::UNPROCESSABLE_ENTITY",
        "StatusCode::UNAUTHORIZED",
        "StatusCode::FORBIDDEN",
        "StatusCode::BAD_REQUEST",
        "StatusCode::NOT_FOUND",
        "StatusCode::CONFLICT",
        "StatusCode::SERVICE_UNAVAILABLE",
        "StatusCode::INTERNAL_SERVER_ERROR",
        "StatusCode::CREATED",
        "StatusCode::OK",
        "matches!(user.role.as_str(), \"admin\" | \"owner\")",
        "ProviderRuntimeReadinessActor::ProviderOwner(user.id)",
        "ProviderRuntimeReadinessActor::PlatformAdmin(user.id)",
    ] {
        assert!(API.contains(required), "missing HTTP boundary {required}");
    }
    assert_eq!(API.matches("StatusCode::SERVICE_UNAVAILABLE").count(), 1);
    assert_eq!(API.matches("trigger_response(").count(), 2);
    let create = source_block(API, "async fn admin_create(", "async fn admin_currentness(");
    assert_ordered(
        create,
        &[
            "admin_actor(&state, &headers)",
            "json_body(payload)",
            "trigger_response(",
            "service::create(",
        ],
    );
    assert!(
        !source_block(API, "async fn owner_currentness(", "async fn owner_revoke(")
            .contains("trigger_response")
    );
    assert!(
        !source_block(API, "async fn owner_revoke(", "fn dispatch_currentness(")
            .contains("trigger_response")
    );
}

#[test]
fn provider_runtime_readiness_api_source_freezes_exact_request_shapes() {
    assert_eq!(
        DOMAIN_INPUT
            .matches("#[serde(deny_unknown_fields)]")
            .count(),
        3
    );
    assert_fields(
        struct_block(DOMAIN_INPUT, "CreateProviderRuntimeReadinessReceiptBody"),
        &[
            "expected_provider_binding_digest",
            "expected_installation_receipt_id",
            "expected_installation_receipt_digest",
            "expected_candidate_digest",
            "expected_profile_digest",
            "expected_target_digest",
            "expected_companion_digest",
            "runtime_compatibility_verification_receipt_id",
            "expected_runtime_compatibility_verification_receipt_digest",
            "expected_predecessor",
            "idempotency_key",
            "confirm_provider_runtime_readiness",
        ],
        12,
    );
    assert_fields(
        struct_block(DOMAIN_INPUT, "ExpectedProviderRuntimeReadinessPredecessor"),
        &["readiness_receipt_id", "readiness_receipt_digest"],
        2,
    );
    assert_fields(
        struct_block(DOMAIN_INPUT, "RevokeProviderRuntimeReadinessReceiptBody"),
        &[
            "expected_readiness_receipt_digest",
            "reason",
            "idempotency_key",
            "confirm_revocation",
        ],
        4,
    );
    let create = struct_block(DOMAIN_INPUT, "CreateProviderRuntimeReadinessReceiptBody");
    for forbidden in [
        "content_digest",
        "actor",
        "scope",
        "checked_at",
        "expires_at",
        "endpoint",
        "secret",
        "path",
        "fd",
        "cgroup",
        "nonce",
        "observation",
        "readiness",
        "result",
    ] {
        assert!(
            !create
                .to_ascii_lowercase()
                .contains(&format!("pub {forbidden}")),
            "caller can express forbidden field {forbidden}"
        );
    }
}

#[test]
fn provider_runtime_readiness_api_source_freezes_validation_and_recursive_redaction() {
    for required in [
        "validate_create_provider_runtime_readiness_receipt_body(body)",
        "validate_revoke_provider_runtime_readiness_receipt_body(body)",
        "Provider binding ID",
        "activation candidate ID",
        "runtime launch-profile ID",
        "upstream transport-target ID",
        "supervisor/session policy-companion ID",
        "Provider runtime-readiness receipt ID",
        "v270:provider-runtime-readiness:create:{actor_user_id}",
        "v270:provider-runtime-readiness:{operation}:{actor_kind}:{actor_user_id}",
    ] {
        assert!(
            VALIDATION.contains(required),
            "missing validation {required}"
        );
    }
    for required in [
        "map.values_mut().for_each(redact)",
        "runtime_custody_epoch_digest",
        "runtime_bundle_identity_commitment",
        "post_cleanup_observation_commitment",
        "request_digest",
        "response_digest",
        "request_bytes",
        "response_bytes",
        "selected_address",
        "expected_tls_leaf_spki_sha256",
        "credential_sha256",
        "bundle_generation",
        "entrypoint_path",
        "session_key",
        "pidfd",
        "cgroup_path",
        "recorded_by_actor_user_id",
        "revoked_by_actor_user_id",
        "idempotency_key",
        "receipt_json",
    ] {
        assert!(REDACTION.contains(required), "missing redaction {required}");
    }
}

fn struct_block<'a>(source: &'a str, name: &str) -> &'a str {
    source
        .split_once(&format!("struct {name} {{"))
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0
}

fn assert_fields(block: &str, fields: &[&str], expected_count: usize) {
    assert_eq!(block.matches("pub ").count(), expected_count);
    for field in fields {
        assert!(block.contains(&format!("pub {field}:")), "missing {field}");
    }
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
