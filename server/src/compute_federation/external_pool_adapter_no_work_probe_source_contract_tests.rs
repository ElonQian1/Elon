const SESSION_NO_WORK: &str =
    include_str!("../../external-pool-adapter-session-core/src/no_work.rs");
const SESSION_TRANSPORT: &str =
    include_str!("../../external-pool-adapter-session-core/src/transport.rs");
const BROKER_NO_WORK: &str = include_str!("external_pool_adapter_broker_tls/no_work.rs");
const BROKER_TRANSPORT: &str = include_str!("external_pool_adapter_broker_tls/transport.rs");
const BROKER_STORE: &str =
    include_str!("../store/compute_external_pool_adapter_upstream_transport_target/broker_tls.rs");
const DELIVERY_STORE: &str =
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/secret_delivery.rs");
const PROBE_STORE: &str =
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/no_work_probe.rs");
const V258_POLICY: &str =
    include_str!("../store/compute_external_pool_adapter_upstream_transport_target/policy.rs");

#[test]
fn v265_elnw_is_one_shot_root_bound_and_stays_inside_authenticated_control_frames() {
    for required in [
        "const PROBE_MAGIC: &[u8; 4] = b\"ELNW\"",
        "ExternalPoolAdapterSessionFrameKind::Control",
        "const MAX_REQUEST_BYTES: usize = 16_384",
        "const MAX_RESPONSE_BYTES: usize = 65_536",
        "request_bytes.to_be_bytes()",
        "response_bytes.to_be_bytes()",
        "request_sha256",
        "response_sha256",
        "pub fn complete(",
        "validate_response: impl FnOnce(&[u8]) -> Result<()>",
        "session.terminate()",
    ] {
        assert!(
            SESSION_NO_WORK.contains(required),
            "missing ELNW rule {required}"
        );
    }
    assert!(SESSION_TRANSPORT.contains("pub fn receive_with_timeout("));
    assert!(SESSION_TRANSPORT
        .contains("const MAX_FRAME_IO_TIMEOUT: Duration = Duration::from_millis(15_000)"));
    for forbidden in [
        "TcpStream",
        "reqwest::",
        "rustls",
        "INSERT INTO",
        "UPDATE compute_",
    ] {
        assert!(
            !SESSION_NO_WORK.contains(forbidden),
            "ELNW child owns forbidden capability {forbidden}"
        );
    }
}

#[test]
fn v265_broker_exposes_only_exact_bounded_application_exchange() {
    for required in [
        "request.is_empty()",
        "request.len() > MAX_REQUEST_BYTES",
        "expected_response_bytes > MAX_RESPONSE_BYTES",
        "timeout > MAX_PROBE_TIMEOUT",
        "channel.begin_application_exchange()?",
        "stream.write_all(request).await?",
        "stream.read_exact(&mut response[..]).await?",
        "application_exchange_used",
    ] {
        let combined = [BROKER_NO_WORK, BROKER_TRANSPORT].concat();
        assert!(
            combined.contains(required),
            "missing broker no-work rule {required}"
        );
    }
    for forbidden in [
        "read_to_end",
        "read_until",
        "proxy",
        "redirect",
        "reqwest::",
    ] {
        assert!(
            !BROKER_NO_WORK.contains(forbidden),
            "broker no-work uses ambiguous I/O {forbidden}"
        );
    }
}

#[test]
fn v265_store_commits_before_network_and_reproves_exact_roots_after_exchange() {
    for required in [
        "prepare_current_external_pool_adapter_broker_tls_channel",
        "prepare_current_external_pool_adapter_ephemeral_secret_delivery",
        "broker.target() != &delivery_target",
        "delivery.receive_no_work_request()?",
        ".exchange_no_work(",
        "delivery.complete_no_work_request(request, &response)?",
        "drop(response)",
        "drop(broker)",
        "reprove_external_pool_adapter_no_work_roots",
        "current_external_pool_adapter_runtime_bundle_authority_on",
        "current_external_pool_adapter_supervisor_session_policy_companion_authority_on",
        "transaction_with_behavior(TransactionBehavior::Immediate)",
        "if &observed != expected",
        "delivery.shutdown_and_reap()?",
    ] {
        assert!(
            PROBE_STORE.contains(required),
            "missing Store no-work rule {required}"
        );
    }
    let connect = PROBE_STORE
        .find("prepare_current_external_pool_adapter_broker_tls_channel")
        .unwrap();
    let delivery = PROBE_STORE
        .find("prepare_current_external_pool_adapter_ephemeral_secret_delivery")
        .unwrap();
    let exchange = PROBE_STORE.find(".exchange_no_work(").unwrap();
    let reproof = PROBE_STORE
        .find("reprove_external_pool_adapter_no_work_roots")
        .unwrap();
    assert!(connect < delivery && delivery < exchange && exchange < reproof);

    for required in [
        "transaction.commit()?",
        "Ok(Some(delivered.ok_or_else",
        "ExternalPoolAdapterEphemeralSecretDeliveryBinding",
        "bundle_material_digest",
        "probe_timeout_ms",
    ] {
        assert!(
            DELIVERY_STORE.contains(required),
            "missing delivery handoff rule {required}"
        );
    }
    assert!(BROKER_STORE.contains("Transaction-free one-shot channel"));
    assert!(BROKER_STORE.contains("drop(current_target)"));
    assert!(BROKER_STORE.contains("transaction.commit()?"));
}

#[test]
fn v265_observation_is_private_expiring_and_preserves_all_no_effect_fences() {
    for required in [
        "pub(in crate::store) struct CurrentExternalPoolAdapterNoWorkProbeObservationAuthority",
        "Process-private proof",
        "expires_at",
        "no_work_observed",
        "consume(&observation)?",
    ] {
        assert!(
            PROBE_STORE.contains(required),
            "missing observation boundary {required}"
        );
    }
    for forbidden in [
        "pub fn ",
        "INSERT INTO",
        "UPDATE compute_",
        "DELETE FROM",
        "activate_external_pool",
        "compute_route",
        "compute_service_actor",
        "compute_usage",
        "compute_settlement",
        "sui_client",
        "axum",
        "mcp",
    ] {
        assert!(
            !PROBE_STORE.contains(forbidden),
            "V265 crosses no-effect fence {forbidden}"
        );
    }
    for inert in [
        "upstream_probe_observed: false",
        "runtime_launch_ready: false",
        "activation_ready: false",
    ] {
        assert!(
            V258_POLICY.contains(inert),
            "V258/V265 inert fence drifted: {inert}"
        );
    }
}
