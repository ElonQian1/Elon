use anyhow::Result;

use crate::{
    compute_federation::external_pool_adapter_upstream_transport_target::{
        server_upstream_transport_target_policy_catalog, UPSTREAM_TRANSPORT_TARGET_EFFECT,
        UPSTREAM_TRANSPORT_TARGET_NO_EFFECT,
    },
    store::Store,
};

use super::types::{
    ExternalPoolAdapterUpstreamTransportTargetPolicySummary,
    UpstreamTransportTargetPolicyCatalogEntry,
};

impl Store {
    pub(crate) fn external_pool_adapter_upstream_transport_target_policy_summary(
        &self,
    ) -> Result<ExternalPoolAdapterUpstreamTransportTargetPolicySummary> {
        let entry = upstream_transport_target_policy_catalog()?;
        let p = entry.policy;
        Ok(ExternalPoolAdapterUpstreamTransportTargetPolicySummary {
            schema: "compute_federation.external_pool_adapter_upstream_transport_target_policy_summary.v1",
            policy_id: p.policy_id,
            policy_revision: p.policy_revision,
            policy_digest: entry.digest,
            transport_owner: p.transport_owner,
            transport_kind: p.transport_kind,
            hostname_policy: p.hostname_policy,
            port_policy: p.port_policy,
            dns_resolution_policy: p.dns_resolution_policy,
            address_selection_policy: p.address_selection_policy,
            tls_version_policy: p.tls_version_policy,
            tls_server_name_policy: p.tls_server_name_policy,
            tls_chain_policy: p.tls_chain_policy,
            tls_trust_anchor_policy: p.tls_trust_anchor_policy,
            tls_leaf_identity_policy: p.tls_leaf_identity_policy,
            proxy_policy: p.proxy_policy,
            redirect_policy: p.redirect_policy,
            zero_rtt_policy: p.zero_rtt_policy,
            client_certificate_policy: p.client_certificate_policy,
            adapter_network_policy: p.adapter_network_policy,
            max_hostname_bytes: p.max_hostname_bytes,
            max_dns_answers: p.max_dns_answers,
            dns_timeout_ms: p.dns_timeout_ms,
            connect_timeout_ms: p.connect_timeout_ms,
            tls_handshake_timeout_ms: p.tls_handshake_timeout_ms,
            max_connect_attempts: p.max_connect_attempts,
            target_effect: UPSTREAM_TRANSPORT_TARGET_EFFECT.into(),
            adapter_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
            runtime_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
            provider_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
            credential_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
            route_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
            execution_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
            usage_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
            market_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
            settlement_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
            broker_connect_ready: false,
            upstream_probe_observed: false,
            runtime_launch_ready: false,
            activation_ready: false,
        })
    }
}

pub(super) fn upstream_transport_target_policy_catalog(
) -> Result<UpstreamTransportTargetPolicyCatalogEntry> {
    let (policy, digest) = server_upstream_transport_target_policy_catalog()?;
    Ok(UpstreamTransportTargetPolicyCatalogEntry { policy, digest })
}
