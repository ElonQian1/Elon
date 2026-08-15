use sha2::{Digest, Sha256};

const API: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification_api.rs");
const SERVICE: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_signing_handoff_service.rs");
const VALIDATION: &str = include_str!(
    "../external_pool_adapter_runtime_compatibility_signing_handoff_service_validation.rs"
);
const DOMAIN_HANDOFF: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification/handoff_types.rs");
const INSTALLATION_AUDIT: &str =
    include_str!("../external_pool_adapter_installation/filesystem/audit.rs");
const STORE_HANDOFF: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/handoff.rs"
);
const STORE_READ: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/read.rs"
);
const STORE_MODULE: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_compatibility_verification.rs");
const PROFILE: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification/profile.rs");
const V254_FENCES: &str = include_str!(
    "../../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);

const EXACT_ROUTE: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/:challenge_id/signing-handoff";

#[test]
fn signing_handoff_source_freezes_exact_admin_post_and_http_statuses() {
    assert!(API.contains(&format!(
        "const SIGNING_HANDOFF_PATH: &str = \"{EXACT_ROUTE}\";"
    )));
    assert!(API.contains(".route(SIGNING_HANDOFF_PATH, post(signing_handoff))"));
    let handler = source_block(API, "async fn signing_handoff(", "fn write_response(");
    assert_ordered(
        handler,
        &[
            "platform_admin(&state, &headers)",
            "json_body(payload)",
            "signing_handoff_service::signing_handoff_for_admin(",
        ],
    );
    for required in [
        "matches!(user.role.as_str(), \"admin\" | \"owner\")",
        "StatusCode::UNAUTHORIZED",
        "StatusCode::FORBIDDEN",
        "StatusCode::BAD_REQUEST",
        "StatusCode::NOT_FOUND",
        "StatusCode::CONFLICT",
        "StatusCode::UNPROCESSABLE_ENTITY",
        "StatusCode::SERVICE_UNAVAILABLE",
        "StatusCode::INTERNAL_SERVER_ERROR",
    ] {
        assert!(
            API.contains(required),
            "missing V269 HTTP boundary {required}"
        );
    }
    let response = source_block(API, "fn signing_handoff_response(", "fn json_body<");
    assert_ordered(
        response,
        &["output.replayed", "StatusCode::OK", "StatusCode::CREATED"],
    );
    for forbidden in [
        "/api/me/",
        "/run\"",
        "/run-observation",
        "/observations",
        "/signer-callback",
        "/owner/",
    ] {
        assert!(!API.contains(forbidden), "forbidden V269 route {forbidden}");
    }
}

#[test]
fn signing_handoff_source_freezes_exact_six_field_body() {
    assert_eq!(
        VALIDATION.matches("#[serde(deny_unknown_fields)]").count(),
        1
    );
    let body = struct_block(VALIDATION, "RuntimeCompatibilitySigningHandoffBody");
    assert_fields(
        body,
        &[
            "expected_challenge_digest",
            "provider_binding_id",
            "expected_provider_binding_digest",
            "expected_installation_receipt_id",
            "expected_installation_receipt_digest",
            "confirm_signing_handoff",
        ],
        6,
    );
    assert!(body.contains("pub confirm_signing_handoff: bool"));
    for forbidden in [
        "admin_user_id",
        "actor",
        "scope",
        "idempotency",
        "sandbox_verifier_key",
        "signature",
        "observation",
        "fixture",
        "prepared",
        "path",
        "fd",
        "cgroup",
        "timeout",
        "timestamp",
        "nonce",
        "policy",
        "secret",
        "target",
    ] {
        assert!(
            !body
                .to_ascii_lowercase()
                .contains(&format!("pub {forbidden}")),
            "caller can express forbidden V269 field {forbidden}"
        );
    }
    for required in [
        "if !body.confirm_signing_handoff",
        "value.len() != 64",
        "byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)",
    ] {
        assert!(
            VALIDATION.contains(required),
            "missing body gate {required}"
        );
    }
}

#[test]
fn signing_handoff_source_is_one_blocking_exact_binding_operation() {
    let operation = source_block(
        SERVICE,
        "async fn run_signing_handoff(",
        "fn classify_filesystem_error(",
    );
    for required in [
        "external_pool_adapter_registry_release_exists(registry_release_id)",
        "external_pool_adapter_runtime_compatibility_verification_challenge_exists(",
        "external_pool_adapter_registry_provider_binding_audit_target(&body.provider_binding_id)",
        "target.installation_receipt_id != body.expected_installation_receipt_id",
        "target.installation_receipt_digest != body.expected_installation_receipt_digest",
    ] {
        assert!(
            operation.contains(required),
            "missing exact target gate {required}"
        );
    }
    assert_eq!(SERVICE.matches("tokio::task::spawn_blocking").count(), 1);
    let blocking = SERVICE.split_once("tokio::task::spawn_blocking").unwrap().1;
    assert_ordered(
        blocking,
        &[
            "audit_external_pool_adapter_installation(",
            "run_external_pool_adapter_runtime_compatibility_signing_handoff(",
            "runtime.cgroup_parent()",
        ],
    );
    for exact_argument in [
        "&expected_registry_release_id",
        "&challenge_id",
        "&body.expected_challenge_digest",
        "&body.provider_binding_id",
        "&body.expected_provider_binding_digest",
        "&body.expected_installation_receipt_id",
        "&body.expected_installation_receipt_digest",
        "prepared",
        "runtime.cgroup_parent()",
    ] {
        assert!(
            blocking.contains(exact_argument),
            "missing exact runner input {exact_argument}"
        );
    }
    assert!(SERVICE.contains(
        "ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Conflict(error)"
    ));
    assert!(SERVICE
        .contains("ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Storage(error)"));
    assert!(SERVICE.contains("AnyError::new(error)"));
    assert!(INSTALLATION_AUDIT.contains("pub(crate) fn audit_external_pool_adapter_installation"));
}

#[test]
fn signing_handoff_source_freezes_direct_json_allowlist_without_recursive_redaction() {
    let response = struct_block(SERVICE, "RuntimeCompatibilitySigningHandoffResponse");
    assert_fields(
        response,
        &["schema", "record_binding", "signer_payload", "replayed"],
        4,
    );
    assert!(SERVICE.contains("#[derive(Serialize)]"));
    assert!(!SERVICE.contains("redacted_json"));

    let binding = struct_block(
        DOMAIN_HANDOFF,
        "ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRecordBinding",
    );
    assert_fields(
        binding,
        &["run_observation_id", "run_observation_digest"],
        2,
    );
    let payload = struct_block(
        DOMAIN_HANDOFF,
        "ExternalPoolAdapterRuntimeCompatibilitySignerPayload",
    );
    assert_fields(
        payload,
        &[
            "schema",
            "signature_algorithm",
            "sandbox_verifier_key_record_id",
            "sandbox_verifier_key_record_digest",
            "sandbox_verifier_key_id",
            "signature_message_base64",
            "signature_message_digest",
            "expires_at",
        ],
        8,
    );
    for exact in [
        "compute_federation.external_pool_adapter_runtime_compatibility_signing_handoff.v1",
        "compute_federation.external_pool_adapter_runtime_compatibility_signer_payload.v1",
    ] {
        assert!(
            DOMAIN_HANDOFF.contains(exact),
            "missing response schema {exact}"
        );
    }
    // This is a direct JSON-field allowlist only. The Base64-encoded domain-framed signer
    // message intentionally commits the V268 observation roots needed by the signer.
    for forbidden in [
        "verifier_operator",
        "verifier_product",
        "public_key",
        "public_key_pem",
        "signature_base64",
        "fixture_resources",
        "source_capsule",
        "launch_image",
        "provider_binding",
        "installation_receipt",
        "admin_user",
        "cgroup",
        "pidfd",
    ] {
        let projection = format!("{response}{binding}{payload}");
        assert!(
            !projection.contains(forbidden),
            "direct response projection gained {forbidden}"
        );
    }
}

#[test]
fn signing_handoff_store_source_freezes_current_roots_and_durable_replay() {
    let signature = STORE_HANDOFF
        .split_once(
            "pub(crate) fn run_external_pool_adapter_runtime_compatibility_signing_handoff(",
        )
        .unwrap()
        .1
        .split_once(") ->")
        .unwrap()
        .0;
    for input in [
        "expected_registry_release_id",
        "challenge_id",
        "expected_challenge_digest",
        "provider_binding_id",
        "expected_provider_binding_digest",
        "expected_installation_receipt_id",
        "expected_installation_receipt_digest",
        "PreparedExternalPoolAdapterInstallation",
        "ExternalPoolAdapterSupervisorCgroupParent",
    ] {
        assert!(signature.contains(input), "missing Store ABI input {input}");
    }
    for required in [
        "transaction_with_behavior(TransactionBehavior::Immediate)",
        "require_fresh_current_authority(&tx, &challenge.receipt, &checked_at)",
        "current_external_pool_adapter_registry_provider_binding_authority_on(",
        "binding.provider_binding_digest != expected_provider_binding_digest",
        "binding_material.registry_release_id != expected_registry_release_id",
        "binding_material.registry_release_digest",
        "binding_material.installation_receipt_id",
        "binding_material.installation_receipt_digest",
        "binding_material.installation_content_digest",
        "authority.into_prepared()",
        "run_external_pool_adapter_runtime_compatibility_verification_challenge(",
        "runtime_compatibility_signature_challenge(",
        "replayed: private.replayed",
    ] {
        assert!(
            STORE_HANDOFF.contains(required),
            "missing Store gate {required}"
        );
    }
    assert_ordered(
        STORE_HANDOFF,
        &[
            "tx.commit()",
            "run_external_pool_adapter_runtime_compatibility_verification_challenge(",
            "run_observation_by_challenge_on(&tx, challenge_id)",
            "runtime_compatibility_signature_challenge(",
            "tx.commit()",
        ],
    );
    assert!(STORE_READ.contains(
        "pub(crate) fn external_pool_adapter_runtime_compatibility_verification_challenge_exists"
    ));
    let compact_store_read = STORE_READ.split_whitespace().collect::<String>();
    assert!(compact_store_read
        .contains("stored.receipt.challenge.registry_release.registry_release_id"));
    assert!(STORE_MODULE.contains("mod handoff;"));
}

#[test]
fn signing_handoff_source_preserves_no_effect_no_readiness_and_no_new_runtime_authority() {
    assert!(!SERVICE.contains("tracing::"));
    assert!(!STORE_HANDOFF.contains("tracing::"));
    for required in [
        "effects: runtime_compatibility_no_effects()",
        "readiness: runtime_compatibility_no_readiness()",
    ] {
        assert!(
            PROFILE.contains(required),
            "V268 no-effect profile drifted: {required}"
        );
    }
    for forbidden in [
        "record_external_pool_adapter_runtime_compatibility_verification(",
        "INSERT INTO compute_external_pool_providers",
        "UPDATE compute_external_pool_providers",
        "compute_route_adapters",
        "compute_service_actor_authorizations",
        "compute_capacity_pools",
        "compute_offers",
        "compute_attempt_start_outbox",
        "compute_attempt_settlements",
    ] {
        let source = format!("{SERVICE}{STORE_HANDOFF}");
        assert!(
            !source.contains(forbidden),
            "V269 gained authority {forbidden}"
        );
    }
}

#[test]
fn signing_handoff_source_preserves_v254_eighteen_fences_exactly() {
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );
    assert_eq!(V254_TRIGGER_NAMES.len(), 18);
    for name in V254_TRIGGER_NAMES {
        assert!(V254_FENCES.contains(name), "missing V254 fence {name}");
    }
}

const V254_TRIGGER_NAMES: &[&str] = &[
    "v254_external_pool_provider_activation_fence",
    "v254_external_pool_provider_insert_active_fence",
    "v254_external_pool_provider_identity_update_fence",
    "v254_external_pool_provider_kind_update_fence",
    "v254_external_pool_provider_version_active_fence",
    "v254_external_pool_candidate_projection_adapter_fence",
    "v254_external_pool_candidate_projection_adapter_version_fence",
    "v254_external_pool_candidate_service_actor_fence",
    "v254_external_pool_route_credential_fence",
    "v254_external_pool_route_authorization_fence",
    "v254_external_pool_route_capability_fence",
    "v254_external_pool_route_seal_fence",
    "v254_external_pool_capacity_pool_insert_active_fence",
    "v254_external_pool_capacity_pool_update_active_fence",
    "v254_external_pool_capacity_pool_version_active_fence",
    "v254_external_pool_offer_insert_market_fence",
    "v254_external_pool_offer_update_market_fence",
    "v254_external_pool_offer_version_market_fence",
];

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
            "missing field {field}"
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
