const SESSION_NO_WORK: &str =
    include_str!("../../external-pool-adapter-session-core/src/no_work.rs");
const SESSION_TRANSPORT: &str = concat!(
    include_str!("../../external-pool-adapter-session-core/src/transport.rs"),
    include_str!("../../external-pool-adapter-session-core/src/transport_io.rs")
);
const BROKER_NO_WORK: &str = include_str!("external_pool_adapter_broker_tls/no_work.rs");
const BROKER_TRANSPORT: &str = include_str!("external_pool_adapter_broker_tls/transport.rs");
const BROKER_STORE: &str =
    include_str!("../store/compute_external_pool_adapter_upstream_transport_target/broker_tls.rs");
const DELIVERY_STORE: &str = concat!(
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/secret_delivery.rs"),
    include_str!(
        "../store/compute_external_pool_adapter_runtime_bundle/secret_delivery/binding.rs"
    )
);
const PROBE_PREPARATION_STORE: &str =
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/probe_preparation.rs");
const OWNED_PROBE_PREPARATION: &str = include_str!(
    "../store/compute_external_pool_adapter_runtime_bundle/probe_preparation/owned.rs"
);
const SUPERVISOR_LIFECYCLE: &str =
    include_str!("external_pool_adapter_linux_supervisor/lifecycle.rs");
const PROBE_REPROOF: &str =
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/no_work_probe/reproof.rs");
const PROBE_EXECUTION: &str = include_str!(
    "../store/compute_external_pool_adapter_runtime_bundle/no_work_probe/execution.rs"
);
const PROBE_STORE: &str = concat!(
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/no_work_probe.rs"),
    include_str!(
        "../store/compute_external_pool_adapter_runtime_bundle/no_work_probe/execution.rs"
    ),
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/no_work_probe/reproof.rs")
);
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
        "mod reproof;",
        "prepare_current_external_pool_adapter_broker_tls_channel",
        "prepare_current_external_pool_adapter_ephemeral_secret_delivery",
        "broker.target() != &delivery_target",
        "delivery.receive_no_work_request()?",
        ".exchange_no_work(",
        "delivery.complete_no_work_request(request, &response)?",
        "drop(response)",
        "drop(broker)",
        "with_reproved_external_pool_adapter_no_work_roots",
        "current_external_pool_adapter_runtime_bundle_authority_on",
        "attests_runtime_bundle_identity_commitment(",
        "current_external_pool_adapter_supervisor_session_policy_companion_authority_on",
        "transaction_with_behavior(TransactionBehavior::Immediate)",
        "if &observed != expected",
        "delivery.shutdown_and_reap()?",
        "CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority",
    ] {
        assert!(
            PROBE_STORE.contains(required),
            "missing Store no-work rule {required}"
        );
    }
    assert!(!PROBE_STORE.contains("final_bundle_commitment !="));
    let physical = PROBE_EXECUTION
        .split_once("pub(super) async fn execute_external_pool_adapter_no_work_probe(")
        .unwrap()
        .1
        .split_once("impl Store {")
        .unwrap()
        .0;
    let request = physical
        .find("delivery.receive_no_work_request()?")
        .unwrap();
    let exchange = physical.find(".exchange_no_work(").unwrap();
    let completion = physical
        .find("delivery.complete_no_work_request(request, &response)?")
        .unwrap();
    let response_drop = physical.find("drop(response)").unwrap();
    let broker_drop = physical.find("drop(broker)").unwrap();
    let cleanup = physical
        .find("let cleaned = delivery.shutdown_and_reap()?")
        .unwrap();
    assert!(
        request < exchange
            && exchange < completion
            && completion < response_drop
            && response_drop < broker_drop
            && broker_drop < cleanup
    );

    let probe = PROBE_EXECUTION
        .split_once(
            "pub(in crate::store) async fn with_current_external_pool_adapter_no_work_probe_observation<",
        )
        .unwrap()
        .1;
    let connect = probe
        .find("prepare_current_external_pool_adapter_broker_tls_channel")
        .unwrap();
    let delivery = probe
        .find("prepare_current_external_pool_adapter_ephemeral_secret_delivery")
        .unwrap();
    let execution = probe
        .find("execute_external_pool_adapter_no_work_probe(")
        .unwrap();
    let reproof = probe
        .find("with_reproved_external_pool_adapter_no_work_roots")
        .unwrap();
    let final_bundle_reopen = probe.find("let reproof_bundle_prepared").unwrap();
    let final_session_reopen = probe.find("let reproof_session_prepared").unwrap();
    assert!(
        connect < delivery
            && delivery < execution
            && execution < final_bundle_reopen
            && final_bundle_reopen < final_session_reopen
            && final_session_reopen < reproof
    );
    let probe_reopens = probe.matches("reopen_prepared()").count();
    assert_eq!(probe_reopens, 4);
    let channel_preparation = BROKER_STORE
        .split_once("pub(in crate::store) async fn prepare_current_external_pool_adapter_broker_tls_channel")
        .unwrap()
        .1;
    let broker_reopens = channel_preparation.matches("reopen_prepared()").count();
    assert_eq!(broker_reopens, 2);
    assert_eq!(broker_reopens + probe_reopens, 6);

    let broker_preflight_commit = channel_preparation.find("transaction.commit()?").unwrap();
    let broker_network = channel_preparation
        .find("connect_external_pool_adapter_broker_tls(broker_target).await?")
        .unwrap();
    let broker_postflight_reopen = channel_preparation
        .find("let postflight_prepared = reopen_prepared()")
        .unwrap();
    let broker_postflight_commit = channel_preparation.rfind("transaction.commit()?").unwrap();
    assert!(
        broker_preflight_commit < broker_network
            && broker_network < broker_postflight_reopen
            && broker_postflight_reopen < broker_postflight_commit
    );

    let delivery_commit = DELIVERY_STORE.find("transaction.commit()?").unwrap();
    let delivery_connection_drop = DELIVERY_STORE.find("drop(connection)").unwrap();
    let child_launch = DELIVERY_STORE
        .find("deliver_to_authenticated_child(")
        .unwrap();
    assert!(delivery_commit < delivery_connection_drop && delivery_connection_drop < child_launch);
    assert!(!DELIVERY_STORE.contains(".await"));

    let final_reproof = PROBE_REPROOF;
    let final_begin = final_reproof
        .find("transaction_with_behavior(TransactionBehavior::Immediate)")
        .unwrap();
    let final_commitment = final_reproof
        .find("post_cleanup_observation_commitment(")
        .unwrap();
    let final_callback = final_reproof
        .find("consume(&transaction, &observation)?")
        .unwrap();
    let final_commit = final_reproof.find("transaction.commit()?").unwrap();
    assert!(
        final_begin < final_commitment
            && final_commitment < final_callback
            && final_callback < final_commit
    );

    for required in [
        "mod binding;",
        "transaction.commit()?",
        "drop(connection)",
        "deliver_to_authenticated_child(",
        "bundle.into_prepared_bundle()",
        "drop(bundle)",
        "drop(capsule)",
        "ExternalPoolAdapterEphemeralSecretDeliveryBinding",
        "bundle_material_digest",
        "probe_timeout_ms",
        "source_capsule_digest",
        "launch_capsule_digest",
        "launch_capsule_size_bytes",
        "self.launch_capsule_digest.clone()",
        "Result<CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority>",
        "Ok(CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority",
        "verify_slices_are_equal(",
    ] {
        assert!(
            DELIVERY_STORE.contains(required),
            "missing delivery handoff rule {required}"
        );
    }
    assert!(!DELIVERY_STORE.contains(
        "runtime_bundle_identity_commitment == other.runtime_bundle_identity_commitment"
    ));
    assert!(PROBE_PREPARATION_STORE.contains("mod owned;"));
    for required in [
        "prepare_external_pool_adapter_entrypoint_capsule(&source)?",
        "audit_capsule(bundle, selected, &capsule, &policy)?",
        "recheck_callback_freshness(bundle, selected)?",
        "consume(&authority)?",
        "bundle.revalidate()",
    ] {
        assert!(
            OWNED_PROBE_PREPARATION.contains(required),
            "missing owned probe preparation rule {required}"
        );
    }
    assert!(!OWNED_PROBE_PREPARATION.contains(".await"));
    assert!(BROKER_STORE.contains("Transaction-free one-shot channel"));
    assert!(BROKER_STORE.contains("drop(current_target)"));
    assert!(BROKER_STORE.contains("transaction.commit()?"));
    let wait = SUPERVISOR_LIFECYCLE
        .split_once("pub(crate) fn wait(")
        .unwrap()
        .1
        .split_once("pub(crate) fn terminate(")
        .unwrap()
        .0;
    let reap = wait.find("self.reaped = true").unwrap();
    let cleanup = wait.find("self.cleanup_after_reap()?").unwrap();
    let success = wait.find("Ok(Some(observed))").unwrap();
    assert!(reap < cleanup && cleanup < success);
}

#[test]
fn v265_observation_is_private_expiring_and_preserves_all_no_effect_fences() {
    for required in [
        "pub(in crate::store) struct CurrentExternalPoolAdapterNoWorkProbeObservationAuthority",
        "Process-private proof",
        "probe_checked_at",
        "expires_at",
        "no_work_observed",
        "post_cleanup_observation_commitment",
        "current_external_pool_adapter_runtime_compatibility_verification_authority_on",
        "consume(&transaction, &observation)?",
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
