const API: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification_api.rs");
const SERVICE: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification_service.rs");
const VALIDATION: &str = include_str!(
    "../external_pool_adapter_runtime_compatibility_verification_service_validation.rs"
);
const REDACTION: &str = include_str!(
    "../external_pool_adapter_runtime_compatibility_verification_service_redaction.rs"
);
const RELEASE_API: &str = include_str!("../external_pool_adapter_release_api.rs");
const ROUTER: &str = include_str!("../../router.rs");

const EXACT_ROUTES: &[&str] = &[
    "/api/admin/compute/external-pool-adapter-runtime-compatibility-profile-v2",
    "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/challenge",
    "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications",
    "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/currentness",
    "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/:verification_receipt_id/revoke",
    "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/:challenge_id/signing-handoff",
];

#[test]
fn runtime_compatibility_api_source_freezes_exact_admin_only_surface() {
    assert_eq!(API.matches(".route(").count(), 6);
    for path in EXACT_ROUTES {
        assert!(API.contains(path), "missing exact V268 route {path}");
    }
    for binding in [
        ".route(PROFILE_V2_PATH, get(profile_v2))",
        ".route(CHALLENGE_PATH, post(challenge))",
        ".route(VERIFICATIONS_PATH, post(record))",
        ".route(CURRENTNESS_PATH, get(currentness))",
        ".route(REVOCATION_PATH, post(revoke))",
        ".route(SIGNING_HANDOFF_PATH, post(signing_handoff))",
    ] {
        assert!(
            API.contains(binding),
            "missing exact route binding {binding}"
        );
    }
    for required in [
        "auth_from_headers(state, headers)",
        "matches!(user.role.as_str(), \"admin\" | \"owner\")",
        "StatusCode::UNAUTHORIZED",
        "StatusCode::FORBIDDEN",
        "StatusCode::BAD_REQUEST",
        "StatusCode::NOT_FOUND",
        "StatusCode::CONFLICT",
        "StatusCode::UNPROCESSABLE_ENTITY",
        "StatusCode::INTERNAL_SERVER_ERROR",
        "StatusCode::SERVICE_UNAVAILABLE",
        "JsonRejection",
    ] {
        assert!(API.contains(required), "missing HTTP boundary {required}");
    }
    for forbidden in ["/api/me/", "/run\"", "/run-observation", "/observations"] {
        assert!(!API.contains(forbidden), "forbidden V268 route {forbidden}");
    }
    assert!(RELEASE_API.contains(
        ".merge(super::external_pool_adapter_runtime_compatibility_verification_api::routes())"
    ));
    assert!(ROUTER.contains(
        ".merge(crate::compute_federation::external_pool_adapter_release_api::routes())"
    ));
}

#[test]
fn runtime_compatibility_api_source_freezes_write_and_error_status_mapping() {
    let write = source_block(API, "fn write_response(", "fn read_response(");
    assert_ordered(
        write,
        &[
            ".get(\"replayed\")",
            "StatusCode::OK",
            "StatusCode::CREATED",
        ],
    );
    let errors = source_block(API, "fn error_response(", "fn platform_admin(");
    for exact in [
        "RuntimeCompatibilityVerificationServiceError::NotFound => StatusCode::NOT_FOUND",
        "RuntimeCompatibilityVerificationServiceError::Invalid(_) => StatusCode::BAD_REQUEST",
        "RuntimeCompatibilityVerificationServiceError::Conflict(_) => StatusCode::CONFLICT",
    ] {
        assert!(
            errors.contains(exact),
            "missing exact error mapping {exact}"
        );
    }
    assert_ordered(
        errors,
        &[
            "RuntimeCompatibilityVerificationServiceError::Internal(_)",
            "StatusCode::INTERNAL_SERVER_ERROR",
        ],
    );
}

#[test]
fn runtime_compatibility_api_source_freezes_exact_cas_bodies() {
    let predecessor = struct_block(
        VALIDATION,
        "ExpectedRuntimeCompatibilityVerificationPredecessor",
    );
    assert_fields(
        predecessor,
        &["verification_receipt_id", "verification_receipt_digest"],
        2,
    );
    let challenge = struct_block(VALIDATION, "CreateRuntimeCompatibilityChallengeBody");
    assert_fields(
        challenge,
        &[
            "expected_registry_release_digest",
            "sandbox_verifier_key_record_id",
            "expected_sandbox_verifier_key_record_digest",
            "expected_sandbox_verifier_key_id",
            "expected_profile_digest",
            "expected_runner_policy_digest",
            "expected_fixture_catalog_digest",
            "expected_predecessor",
            "idempotency_key",
            "confirm_challenge",
        ],
        10,
    );
    let record = struct_block(VALIDATION, "RecordRuntimeCompatibilityVerificationBody");
    assert_fields(
        record,
        &[
            "run_observation_id",
            "expected_run_observation_digest",
            "expected_signature_message_digest",
            "signature_base64",
            "idempotency_key",
            "confirm_verification",
        ],
        6,
    );
    let revoke = struct_block(VALIDATION, "RevokeRuntimeCompatibilityVerificationBody");
    assert_fields(
        revoke,
        &[
            "expected_verification_receipt_digest",
            "reason",
            "idempotency_key",
            "confirm_revocation",
        ],
        4,
    );
    for forbidden in [
        "pub admin_user_id",
        "pub actor",
        "pub idempotency_scope",
        "pub nonce",
        "pub observation:",
        "pub checked_at",
        "pub public_key_pem",
    ] {
        assert!(
            !VALIDATION.contains(forbidden),
            "caller-controlled {forbidden}"
        );
    }
    assert_eq!(
        VALIDATION.matches("#[serde(deny_unknown_fields)]").count(),
        4
    );
}

#[test]
fn runtime_compatibility_service_source_injects_actor_and_maps_store_seams() {
    for required in [
        "admin_user_id",
        "require_registry_release",
        "require_sandbox_verifier_key_record",
        "require_run_observation",
        "require_verification_receipt",
        "issue_external_pool_adapter_runtime_compatibility_verification_challenge",
        "record_external_pool_adapter_runtime_compatibility_verification",
        "external_pool_adapter_runtime_compatibility_verification_currentness",
        "revoke_external_pool_adapter_runtime_compatibility_verification",
        "RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_CONFIRMATION",
        "RUNTIME_COMPATIBILITY_VERIFICATION_CONFIRMATION",
        "RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_CONFIRMATION",
        "ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Conflict(error)",
        "ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Storage(error)",
        "RuntimeCompatibilityVerificationServiceError::Internal(error)",
    ] {
        assert!(
            SERVICE.contains(required),
            "missing Service seam {required}"
        );
    }
    for required in [
        "confirm_challenge",
        "confirm_verification",
        "confirm_revocation",
        "STANDARD.encode(&decoded) != value",
        "(12..=500)",
    ] {
        assert!(
            VALIDATION.contains(required),
            "missing validation {required}"
        );
    }
}

#[test]
fn runtime_compatibility_public_projection_recursively_redacts_private_roots() {
    for required in [
        "map.retain(|key, _| !redacted_key(key))",
        "map.values_mut().for_each(redact)",
        "challenge_nonce_base64",
        "signature_message_base64",
        "signature_base64",
        "observations",
        "registry_release",
        "fixture_resources",
        "source_capsule_sha256",
        "launch_image_sha256",
        "runner_internal",
        "created_by_admin_user_id",
        "idempotency_scope",
        "confirmation",
        "receipt_json",
        "key.ends_with(\"_json\")",
        "credential_locator_commitment",
        "delivery_root",
        "endpoint_root",
        "expected_tls_leaf_spki_sha256",
        "cgroup_path",
        "pidfd",
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
        assert!(
            block.contains(&format!("pub {field}:")),
            "missing body field {field}"
        );
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
