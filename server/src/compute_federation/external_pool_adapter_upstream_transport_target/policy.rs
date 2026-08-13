use anyhow::Result;

use super::{
    upstream_transport_target_policy_digest, validate_upstream_transport_target_policy,
    ExternalPoolAdapterUpstreamTransportTargetPolicy, UPSTREAM_TRANSPORT_TARGET_POLICY_ID,
    UPSTREAM_TRANSPORT_TARGET_POLICY_REVISION,
};

pub(crate) fn server_upstream_transport_target_policy_catalog(
) -> Result<(ExternalPoolAdapterUpstreamTransportTargetPolicy, String)> {
    let policy = ExternalPoolAdapterUpstreamTransportTargetPolicy {
        policy_id: UPSTREAM_TRANSPORT_TARGET_POLICY_ID.into(),
        policy_revision: UPSTREAM_TRANSPORT_TARGET_POLICY_REVISION,
        transport_owner: "server_broker_v1".into(),
        transport_kind: "brokered_tls_tcp_v1".into(),
        hostname_policy: "lowercase_ascii_dns_hostname_no_ip_literal_v1".into(),
        port_policy: "explicit_nonzero_u16_v1".into(),
        dns_resolution_policy: "fresh_a_aaaa_all_answers_public_unicast_v1".into(),
        address_selection_policy: "broker_pins_one_validated_address_per_connect_attempt_v1".into(),
        tls_version_policy: "tls_1_3_only_v1".into(),
        tls_server_name_policy: "exact_dns_hostname_v1".into(),
        tls_chain_policy: "future_broker_webpki_chain_hostname_and_time_at_connect_v1".into(),
        tls_trust_anchor_policy: "server_webpki_roots_current_at_connect_v1".into(),
        tls_leaf_identity_policy: "expected_leaf_spki_sha256_pin_and_future_webpki_observation_v1"
            .into(),
        proxy_policy: "disabled_v1".into(),
        redirect_policy: "disabled_v1".into(),
        zero_rtt_policy: "disabled_v1".into(),
        client_certificate_policy: "disabled_v1".into(),
        adapter_network_policy: "sidecar_no_network_server_broker_only_v1".into(),
        max_hostname_bytes: 253,
        max_dns_answers: 32,
        dns_timeout_ms: 3_000,
        connect_timeout_ms: 5_000,
        tls_handshake_timeout_ms: 5_000,
        max_connect_attempts: 2,
    };
    validate_upstream_transport_target_policy(&policy)?;
    let digest = upstream_transport_target_policy_digest(&policy)?;
    Ok((policy, digest))
}
