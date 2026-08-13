#[test]
fn runtime_launch_http_source_keeps_policy_server_fixed_and_public_projection_redacted() {
    let service = include_str!("../external_pool_adapter_runtime_launch_profile_service.rs");
    let api = include_str!("../external_pool_adapter_runtime_launch_profile_api.rs");
    let redaction =
        include_str!("../external_pool_adapter_runtime_launch_profile_service_redaction.rs");

    for required in [
        "expected_launch_policy_digest",
        "expected_predecessor: Option<ExpectedRuntimeLaunchProfilePredecessor>",
        "RuntimeLaunchProfileActor::ProviderOwner",
        "Self::PlatformAdmin",
        "audit_external_pool_adapter_installation",
        "external_pool_adapter_runtime_launch_policy_summary",
        "external_pool_adapter_runtime_launch_profile_currentness",
        "runtime_launch_ready",
        "adapter_effect",
        "RUNTIME_LAUNCH_PROFILE_NO_EFFECT",
        "usage_effect",
    ] {
        assert!(
            service.contains(required),
            "missing Service boundary {required}"
        );
    }
    for required in [
        "runtime-launch-policy",
        "runtime-launch-profiles",
        "owner_policy",
        "admin_policy",
        "owner_create",
        "admin_create",
        "owner_currentness",
        "admin_currentness",
        "owner_revoke",
        "admin_revoke",
        "JsonRejection",
    ] {
        assert!(api.contains(required), "missing HTTP boundary {required}");
    }
    for required in [
        "credential_locator_commitment",
        "resolver_backend_policy_digest",
        "recorded_by_actor_user_id",
        "revoked_by_actor_user_id",
        "entrypoint_path",
        "entrypoint_relative_path",
        "receipt_json",
    ] {
        assert!(
            redaction.contains(required),
            "missing redaction key {required}"
        );
    }
}

#[test]
fn runtime_launch_http_source_has_no_launch_secret_or_downstream_write_path() {
    let source = concat!(
        include_str!("../external_pool_adapter_runtime_launch_profile_service.rs"),
        include_str!("../external_pool_adapter_runtime_launch_profile_api.rs"),
        include_str!("../external_pool_adapter_runtime_launch_profile_service_redaction.rs"),
    );
    for forbidden in [
        "std::process::Command",
        "tokio::process",
        "TcpStream",
        "TcpListener",
        "reqwest::",
        "credential_ref_scheme(",
        "credential_locator_commitment(",
        "compute_capacity_pools",
        "compute_offers",
        "compute_price_snapshots",
        "compute_jobs",
        "compute_reservations",
        "compute_attempt_start_outbox",
        "compute_attempt_settlements",
        "compute_route_adapters",
        "compute_service_actor_authorizations",
        "activate_external_pool",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden V255 HTTP path {forbidden}"
        );
    }
    assert_eq!(
        source
            .matches("audit_external_pool_adapter_installation")
            .count(),
        2
    );
    assert_eq!(source.matches("spawn_blocking").count(), 1);
}
