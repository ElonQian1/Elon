use std::time::Duration;

use anyhow::{bail, Result};

use crate::compute_federation::external_pool_adapter_upstream_transport_target::{
    ExternalPoolAdapterUpstreamTransportTargetReceipt, UPSTREAM_TRANSPORT_TARGET_STATUS,
};

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ExternalPoolAdapterBrokerTlsTarget {
    target_id: String,
    target_digest: String,
    profile_id: String,
    profile_digest: String,
    target_policy_digest: String,
    hostname: String,
    port: u16,
    server_name: String,
    expected_leaf_spki_sha256: [u8; 32],
    max_dns_answers: usize,
    dns_timeout: Duration,
    connect_timeout: Duration,
    tls_handshake_timeout: Duration,
    max_connect_attempts: usize,
}

impl ExternalPoolAdapterBrokerTlsTarget {
    pub(crate) fn from_receipt(
        receipt: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
    ) -> Result<Self> {
        let target = &receipt.target;
        let policy = &target.target_policy;
        if target.target_status != UPSTREAM_TRANSPORT_TARGET_STATUS
            || target.dns_hostname != target.tls_server_name
            || policy.transport_owner != "server_broker_v1"
            || policy.transport_kind != "brokered_tls_tcp_v1"
            || policy.dns_resolution_policy != "fresh_a_aaaa_all_answers_public_unicast_v1"
            || policy.address_selection_policy
                != "broker_pins_one_validated_address_per_connect_attempt_v1"
            || policy.tls_version_policy != "tls_1_3_only_v1"
            || policy.tls_server_name_policy != "exact_dns_hostname_v1"
            || policy.tls_chain_policy
                != "future_broker_webpki_chain_hostname_and_time_at_connect_v1"
            || policy.tls_trust_anchor_policy != "server_webpki_roots_current_at_connect_v1"
            || policy.tls_leaf_identity_policy
                != "expected_leaf_spki_sha256_pin_and_future_webpki_observation_v1"
            || policy.proxy_policy != "disabled_v1"
            || policy.redirect_policy != "disabled_v1"
            || policy.zero_rtt_policy != "disabled_v1"
            || policy.client_certificate_policy != "disabled_v1"
            || policy.adapter_network_policy != "sidecar_no_network_server_broker_only_v1"
        {
            bail!("broker TLS target policy rejected");
        }
        Self::new(
            receipt.target_id.clone(),
            receipt.target_digest.clone(),
            target.profile_id.clone(),
            target.profile_digest.clone(),
            target.target_policy_digest.clone(),
            target.dns_hostname.clone(),
            target.port,
            target.tls_server_name.clone(),
            &target.expected_tls_leaf_spki_sha256,
            usize::try_from(policy.max_dns_answers)?,
            policy.dns_timeout_ms,
            policy.connect_timeout_ms,
            policy.tls_handshake_timeout_ms,
            usize::try_from(policy.max_connect_attempts)?,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        target_id: String,
        target_digest: String,
        profile_id: String,
        profile_digest: String,
        target_policy_digest: String,
        hostname: String,
        port: u16,
        server_name: String,
        expected_pin: &str,
        max_dns_answers: usize,
        dns_timeout_ms: u64,
        connect_timeout_ms: u64,
        tls_handshake_timeout_ms: u64,
        max_connect_attempts: usize,
    ) -> Result<Self> {
        let expected_leaf_spki_sha256 = decode_sha256(expected_pin)?;
        if target_id.trim().is_empty()
            || !is_sha256(&target_digest)
            || profile_id.trim().is_empty()
            || !is_sha256(&profile_digest)
            || !is_sha256(&target_policy_digest)
            || hostname.is_empty()
            || hostname != server_name
            || hostname.len() > 253
            || port == 0
            || max_dns_answers == 0
            || max_dns_answers > 32
            || max_connect_attempts == 0
            || max_connect_attempts > 2
            || !(1..=30_000).contains(&dns_timeout_ms)
            || !(1..=30_000).contains(&connect_timeout_ms)
            || !(1..=30_000).contains(&tls_handshake_timeout_ms)
        {
            bail!("broker TLS target rejected");
        }
        Ok(Self {
            target_id,
            target_digest,
            profile_id,
            profile_digest,
            target_policy_digest,
            hostname,
            port,
            server_name,
            expected_leaf_spki_sha256,
            max_dns_answers,
            dns_timeout: Duration::from_millis(dns_timeout_ms),
            connect_timeout: Duration::from_millis(connect_timeout_ms),
            tls_handshake_timeout: Duration::from_millis(tls_handshake_timeout_ms),
            max_connect_attempts,
        })
    }

    pub(crate) fn target_id(&self) -> &str {
        &self.target_id
    }

    pub(crate) fn target_digest(&self) -> &str {
        &self.target_digest
    }

    pub(crate) fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub(crate) fn profile_digest(&self) -> &str {
        &self.profile_digest
    }

    pub(crate) fn target_policy_digest(&self) -> &str {
        &self.target_policy_digest
    }

    pub(super) fn hostname(&self) -> &str {
        &self.hostname
    }

    pub(super) fn port(&self) -> u16 {
        self.port
    }

    pub(super) fn server_name(&self) -> &str {
        &self.server_name
    }

    pub(super) fn expected_leaf_spki_sha256(&self) -> &[u8; 32] {
        &self.expected_leaf_spki_sha256
    }

    pub(super) fn max_dns_answers(&self) -> usize {
        self.max_dns_answers
    }

    pub(super) fn dns_timeout(&self) -> Duration {
        self.dns_timeout
    }

    pub(super) fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    pub(super) fn tls_handshake_timeout(&self) -> Duration {
        self.tls_handshake_timeout
    }

    pub(super) fn max_connect_attempts(&self) -> usize {
        self.max_connect_attempts
    }

    #[cfg(test)]
    pub(super) fn for_test(server_name: &str, port: u16, expected_pin: &str) -> Result<Self> {
        Self::new(
            "target_test".into(),
            "11".repeat(32),
            "profile_test".into(),
            "22".repeat(32),
            "33".repeat(32),
            server_name.into(),
            port,
            server_name.into(),
            expected_pin,
            4,
            500,
            500,
            500,
            2,
        )
    }
}

fn decode_sha256(value: &str) -> Result<[u8; 32]> {
    if !is_sha256(value) {
        bail!("broker TLS SPKI pin rejected");
    }
    let mut digest = [0_u8; 32];
    hex::decode_to_slice(value, &mut digest)?;
    if digest.iter().all(|byte| *byte == 0) {
        bail!("broker TLS SPKI pin rejected");
    }
    Ok(digest)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
