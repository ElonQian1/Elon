const SERVICE: &str =
    include_str!("../external_pool_adapter_provider_runtime_readiness_service.rs");
const VALIDATION: &str =
    include_str!("../external_pool_adapter_provider_runtime_readiness_service_validation.rs");
const REDACTION: &str =
    include_str!("../external_pool_adapter_provider_runtime_readiness_service_redaction.rs");
const DOMAIN_POLICY: &str =
    include_str!("../external_pool_adapter_provider_runtime_readiness/policy.rs");
const DOMAIN_SUMMARY: &str =
    include_str!("../external_pool_adapter_provider_runtime_readiness/summary.rs");
const STORE_WRITE: &str =
    include_str!("../../store/compute_external_pool_adapter_provider_runtime_readiness/write.rs");
const STORE_CURRENT: &str =
    include_str!("../../store/compute_external_pool_adapter_provider_runtime_readiness/current.rs");
const STORE_REVOCATION: &str = include_str!(
    "../../store/compute_external_pool_adapter_provider_runtime_readiness/revocation.rs"
);

#[test]
fn provider_runtime_readiness_service_source_freezes_single_blocking_runtime_operation() {
    assert_eq!(SERVICE.matches("tokio::task::spawn_blocking").count(), 1);
    assert_eq!(SERVICE.matches("handle.block_on").count(), 1);
    let create = source_block(
        SERVICE,
        "pub(crate) async fn create(",
        "pub(crate) fn currentness(",
    );
    assert_ordered(
        create,
        &[
            "validate_create(path, &body)",
            "ProviderRuntimeReadinessActor::PlatformAdmin",
            "external_pool_adapter_provider_runtime_readiness_runtime()",
            "authorize_binding(&state, &actor, path)",
            "exact_companion_target(&state, &actor, path)",
            "tokio::runtime::Handle::current()",
            "tokio::task::spawn_blocking",
            "let mut reopen_prepared = ||",
            "audit_external_pool_adapter_installation(",
            "handle.block_on(",
            "create_external_pool_adapter_provider_runtime_readiness(",
        ],
    );
    for required in [
        "ExternalPoolAdapterInstallationReopener<'_>",
        "with_current_external_pool_adapter_no_work_probe_observation(",
        "preflight_create(transaction, &input, target, checked_at)",
        "finalize_create(transaction, &input, observation, runtime)",
    ] {
        assert!(
            STORE_WRITE.contains(required),
            "missing Store ABI {required}"
        );
    }
    for forbidden in [
        "TransactionBehavior",
        "rusqlite::Connection",
        "TcpStream",
        "reqwest::",
        "std::process::Command",
    ] {
        assert!(!create.contains(forbidden), "Service retained {forbidden}");
    }
}

#[test]
fn provider_runtime_readiness_service_source_freezes_owner_auth_and_runtime_boundaries() {
    let currentness = source_block(
        SERVICE,
        "pub(crate) fn currentness(",
        "pub(crate) fn revoke(",
    );
    assert_ordered(
        currentness,
        &[
            "validate_currentness(path, readiness_receipt_id)",
            "authorize_binding(state, &actor, path)",
            "exact_companion_target(state, &actor, path)",
            "external_pool_adapter_provider_runtime_readiness_runtime().ok()",
            "external_pool_adapter_provider_runtime_readiness_currentness(",
            "runtime.as_deref()",
        ],
    );
    let revoke = source_block(SERVICE, "pub(crate) fn revoke(", "fn authorize_binding(");
    assert_ordered(
        revoke,
        &[
            "validate_revoke(path, readiness_receipt_id, &body)",
            "authorize_binding(state, &actor, path)",
            "exact_companion_target(state, &actor, path)",
            "external_pool_adapter_provider_runtime_readiness_currentness(",
            "None",
            "ProviderRuntimeReadinessServiceError::NotFound",
            "revoke_external_pool_adapter_provider_runtime_readiness(",
        ],
    );
    assert!(!revoke.contains("external_pool_adapter_provider_runtime_readiness_runtime()"));
    let authorize = source_block(
        SERVICE,
        "fn authorize_binding(",
        "fn exact_companion_target(",
    );
    assert_ordered(
        authorize,
        &[
            "external_pool_provider_activation_candidate_audit_target(path[1])",
            "ProviderRuntimeReadinessActor::ProviderOwner",
            "ProviderRuntimeReadinessServiceError::NotFound",
            "target.provider_binding_id",
        ],
    );
    let companion = source_block(
        SERVICE,
        "fn exact_companion_target(",
        "fn classify_store_error(",
    );
    assert_ordered(
        companion,
        &[
            "require_exact(actual, expected, authority)",
            "ProviderRuntimeReadinessActor::ProviderOwner",
            "ProviderRuntimeReadinessServiceError::NotFound",
            "ProviderRuntimeReadinessActor::PlatformAdmin",
            "ProviderRuntimeReadinessServiceError::Conflict",
        ],
    );
    for required in [
        "PROVIDER_RUNTIME_READINESS_ACTOR_PLATFORM_ADMIN",
        "PROVIDER_RUNTIME_READINESS_ACTOR_PROVIDER_OWNER",
        "PROVIDER_RUNTIME_READINESS_CONFIRMATION",
        "PROVIDER_RUNTIME_READINESS_REVOCATION_CONFIRMATION",
        "idempotency_scope(\"create\"",
        "idempotency_scope(\"revoke\"",
    ] {
        assert!(
            SERVICE.contains(required),
            "missing actor binding {required}"
        );
    }
}

#[test]
fn provider_runtime_readiness_source_freezes_safe_projection_and_no_effects() {
    for required in [
        "process_spawn_ready: true",
        "ipc_session_ready: true",
        "secret_delivery_ready: true",
        "broker_connect_ready: true",
        "upstream_probe_observed: true",
        "runtime_launch_ready: true",
        "activation_ready: false",
    ] {
        assert!(
            DOMAIN_POLICY.contains(required),
            "missing readiness {required}"
        );
    }
    assert_eq!(
        DOMAIN_POLICY
            .matches("PROVIDER_RUNTIME_READINESS_NO_EFFECT.into()")
            .count(),
        10,
        "nine effects plus activation_authority must remain none"
    );
    for private in [
        "runtime_custody_epoch_digest",
        "runtime_bundle_identity_commitment",
        "post_cleanup_observation_commitment",
        "probe_execution_id",
        "request_bytes",
        "response_bytes",
        "recorded_by_actor_user_id",
        "idempotency_key",
    ] {
        assert!(
            !DOMAIN_SUMMARY.contains(private),
            "summary exposed {private}"
        );
        assert!(REDACTION.contains(private), "redaction lost {private}");
    }
    for forbidden in [
        "activate_external_pool",
        "compute_attempt_start_outbox",
        "compute_capacity_pools",
        "compute_offers",
        "compute_attempt_settlements",
        "signing_handoff",
        "unattended_signer",
    ] {
        let source = format!("{SERVICE}{STORE_WRITE}{STORE_CURRENT}{STORE_REVOCATION}");
        assert!(
            !source.contains(forbidden),
            "V270 gained authority {forbidden}"
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
