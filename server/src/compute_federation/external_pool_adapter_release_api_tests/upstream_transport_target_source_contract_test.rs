#[test]
fn upstream_transport_http_source_keeps_policy_server_fixed_and_response_private() {
    let service = include_str!("../external_pool_adapter_upstream_transport_target_service.rs");
    let validation =
        include_str!("../external_pool_adapter_upstream_transport_target_service_validation.rs");
    let api = include_str!("../external_pool_adapter_upstream_transport_target_api.rs");
    let redaction =
        include_str!("../external_pool_adapter_upstream_transport_target_service_redaction.rs");

    for required in [
        "expected_profile_digest",
        "expected_candidate_digest",
        "expected_provider_binding_digest",
        "expected_target_policy_digest",
        "draft: UpstreamTransportTargetDraftBody",
        "expected_predecessor: Option<ExpectedUpstreamTransportTargetPredecessor>",
        "ProviderOwner(String)",
        "PlatformAdmin(String)",
        "Self::ProviderOwner",
        "Self::PlatformAdmin",
        "audit_external_pool_adapter_installation",
        "external_pool_adapter_upstream_transport_target_policy_summary",
        "external_pool_adapter_upstream_transport_target_currentness",
        "broker_connect_ready",
        "upstream_probe_observed",
    ] {
        assert!(
            service.contains(required),
            "missing Service boundary {required}"
        );
    }
    for required in [
        "validate_upstream_transport_dns_hostname",
        "expected_tls_leaf_spki_sha256",
        "UPSTREAM_TRANSPORT_TARGET_NO_EFFECT",
        "UPSTREAM_TRANSPORT_TARGET_REVOCATION_EFFECT",
        "value.provider_effect",
        "value.credential_effect",
        "value.route_effect",
        "value.execution_effect",
        "value.usage_effect",
        "value.market_effect",
        "value.settlement_effect",
    ] {
        assert!(
            validation.contains(required),
            "missing validation boundary {required}"
        );
    }
    for required in [
        "upstream-transport-policy",
        "upstream-transport-targets",
        "owner_policy",
        "admin_policy",
        "owner_create",
        "admin_create",
        "owner_currentness",
        "admin_currentness",
        "owner_revoke",
        "admin_revoke",
        "JsonRejection",
        "authenticated_user(state, headers).map(UpstreamTransportTargetActor::ProviderOwner)",
        "Ok(UpstreamTransportTargetActor::PlatformAdmin(user.id))",
        "matches!(user.role.as_str(), \"admin\" | \"owner\")",
    ] {
        assert!(api.contains(required), "missing HTTP boundary {required}");
    }
    for required in [
        "dns_hostname",
        "port",
        "tls_server_name",
        "expected_tls_leaf_spki_sha256",
        "recorded_by_actor_user_id",
        "revoked_by_actor_user_id",
        "idempotency_key",
        "confirmation",
        "receipt_json",
    ] {
        assert!(
            redaction.contains(required),
            "missing redaction key {required}"
        );
    }
}

#[test]
fn upstream_transport_http_source_has_no_network_secret_probe_or_downstream_write_path() {
    let source = concat!(
        include_str!("../external_pool_adapter_upstream_transport_target_service.rs"),
        include_str!("../external_pool_adapter_upstream_transport_target_service_validation.rs"),
        include_str!("../external_pool_adapter_upstream_transport_target_api.rs"),
        include_str!("../external_pool_adapter_upstream_transport_target_service_redaction.rs"),
    );
    for forbidden in [
        "std::process::Command",
        "tokio::process",
        "TcpStream",
        "TcpListener",
        "reqwest::",
        "trust_dns",
        "hickory_resolver",
        "rustls::",
        "credential_locator(",
        "resolve_external_pool_adapter_runtime_bundle",
        "with_sensitive_bytes",
        "LockedSensitiveBytes",
        "sensitive_frame",
        "authenticated_sensitive_frame",
        "deliver_sensitive",
        "resolve_external_pool",
        "probe_external_pool",
        "compute_capacity_pools",
        "compute_offers",
        "compute_price_snapshots",
        "compute_jobs",
        "compute_reservations",
        "compute_attempt_start_outbox",
        "compute_attempt_settlements",
        "activate_external_pool",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden V258 HTTP path {forbidden}"
        );
    }
    assert_eq!(
        source
            .matches("audit_external_pool_adapter_installation")
            .count(),
        2,
        "one import plus one audit call are required"
    );
    assert_eq!(source.matches("spawn_blocking").count(), 1);
}
