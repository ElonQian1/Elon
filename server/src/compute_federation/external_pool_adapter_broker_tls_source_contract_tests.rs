const BROKER_ROOT: &str = include_str!("external_pool_adapter_broker_tls.rs");
const BROKER_ADDRESS_POLICY: &str =
    include_str!("external_pool_adapter_broker_tls/address_policy.rs");
const BROKER_TARGET: &str = include_str!("external_pool_adapter_broker_tls/target.rs");
const BROKER_TRANSPORT: &str = include_str!("external_pool_adapter_broker_tls/transport.rs");
const BROKER_STORE: &str =
    include_str!("../store/compute_external_pool_adapter_upstream_transport_target/broker_tls.rs");
const V258_TARGET_ROOT: &str = include_str!("external_pool_adapter_upstream_transport_target.rs");
const V260_SESSION_ROOT: &str = include_str!("external_pool_adapter_supervisor_session.rs");

#[test]
fn v264_broker_owns_dns_direct_tcp_tls13_webpki_and_exact_spki_pin() {
    for required in [
        "lookup_host((target.hostname(), target.port()))",
        "validate_and_order_dns_answers",
        "TcpStream::connect(address)",
        "with_protocol_versions(&[&rustls::version::TLS13])",
        "webpki_roots::TLS_SERVER_ROOTS",
        "ServerName::try_from(target.server_name().to_owned())",
        "subject_public_key_info_der",
        "Sha256::digest(subject_public_key_info_der)",
        "constant_time::verify_slices_are_equal",
    ] {
        assert!(
            BROKER_TRANSPORT.contains(required),
            "missing broker transport rule {required}"
        );
    }
    for required in [
        "fresh_a_aaaa_all_answers_public_unicast_v1",
        "brokered_tls_tcp_v1",
        "sidecar_no_network_server_broker_only_v1",
    ] {
        assert!(
            BROKER_TARGET.contains(required),
            "missing target rule {required}"
        );
    }
    assert!(BROKER_ADDRESS_POLICY.contains("is_public_unicast"));
    assert!(BROKER_ROOT.contains("exposes no application read/write API"));
}

#[test]
fn v264_store_reproves_v258_and_full_installation_after_network_await() {
    for required in [
        "ExternalPoolAdapterInstallationReopener<'a>",
        "> + Send",
        "reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>",
        "let preflight_prepared = reopen_prepared()",
        "let postflight_prepared = reopen_prepared()",
        "current_external_pool_adapter_upstream_transport_target_authority_on",
        "current_installation_binding(&authority).clone()",
        "connect_external_pool_adapter_broker_tls(broker_target).await",
        "transaction_with_behavior(TransactionBehavior::Immediate)",
        "current_installation_binding(&current_target) != &preflight_binding",
        "channel.target() != &postflight_broker_target",
        "channel.is_current()",
        "consume(&authority)?",
    ] {
        assert!(
            BROKER_STORE.contains(required),
            "missing Store rule {required}"
        );
    }
    let preflight = BROKER_STORE.find("let (preflight_target").unwrap();
    let network = BROKER_STORE
        .find("connect_external_pool_adapter_broker_tls(broker_target).await")
        .unwrap();
    let postflight = network
        + BROKER_STORE[network..]
            .find("transaction_with_behavior(TransactionBehavior::Immediate)")
            .unwrap();
    assert!(preflight < network && network < postflight);
}

#[test]
fn v264_channel_is_short_lived_store_private_and_has_no_application_or_economic_effect() {
    for required in [
        "pub(crate) struct ExternalPoolAdapterBrokerTlsChannel",
        "const CHANNEL_MAX_AGE_SECONDS: u64 = 30",
        "exposes no I/O methods",
        "pub(in crate::store) struct CurrentExternalPoolAdapterBrokerTlsAuthority",
        "intentionally non-Clone/non-Debug/non-Serde",
    ] {
        let combined = [BROKER_TRANSPORT, BROKER_STORE].concat();
        assert!(
            combined.contains(required),
            "missing private rule {required}"
        );
    }
    let combined = [BROKER_TRANSPORT, BROKER_STORE].concat();
    for forbidden in [
        "AsyncWriteExt",
        ".write_all(",
        "reqwest::",
        "INSERT INTO",
        "UPDATE compute_",
        "DELETE FROM",
        "settlement",
        "marketplace",
        "activate_external_pool",
    ] {
        assert!(
            !combined.contains(forbidden),
            "V264 crosses no-effect fence {forbidden}"
        );
    }
    assert!(!V258_TARGET_ROOT.contains("external_pool_adapter_broker_tls"));
    assert!(!V260_SESSION_ROOT.contains("external_pool_adapter_broker_tls"));
}
