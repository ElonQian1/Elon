use serde::{Deserialize, Serialize};

pub(crate) const UPSTREAM_TRANSPORT_TARGET_SCHEMA: &str =
    "compute_federation.external_pool_adapter_upstream_transport_target.v1";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_REVOCATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_upstream_transport_target_revocation.v1";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_upstream_transport_target_currentness.v1";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_CONFIRMATION: &str =
    "confirm_external_pool_adapter_upstream_transport_target";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_upstream_transport_target_revocation";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_POLICY_ID: &str =
    "external_pool_adapter_upstream_transport_target_policy_v1";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_POLICY_REVISION: u64 = 1;
pub(crate) const UPSTREAM_TRANSPORT_TARGET_STATUS: &str = "upstream_transport_target_current_inert";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_EFFECT: &str =
    "upstream_transport_target_recorded_inert";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_REVOCATION_EFFECT: &str =
    "upstream_transport_target_revoked";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_NO_EFFECT: &str = "none";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_ACTOR_PROVIDER_OWNER: &str = "provider_owner";
pub(crate) const UPSTREAM_TRANSPORT_TARGET_ACTOR_PLATFORM_ADMIN: &str = "platform_admin";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetPolicy {
    pub policy_id: String,
    pub policy_revision: u64,
    pub transport_owner: String,
    pub transport_kind: String,
    pub hostname_policy: String,
    pub port_policy: String,
    pub dns_resolution_policy: String,
    pub address_selection_policy: String,
    pub tls_version_policy: String,
    pub tls_server_name_policy: String,
    pub tls_chain_policy: String,
    pub tls_trust_anchor_policy: String,
    pub tls_leaf_identity_policy: String,
    pub proxy_policy: String,
    pub redirect_policy: String,
    pub zero_rtt_policy: String,
    pub client_certificate_policy: String,
    pub adapter_network_policy: String,
    pub max_hostname_bytes: u64,
    pub max_dns_answers: u64,
    pub dns_timeout_ms: u64,
    pub connect_timeout_ms: u64,
    pub tls_handshake_timeout_ms: u64,
    pub max_connect_attempts: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetMaterial {
    pub profile_id: String,
    pub profile_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub delegation_id: String,
    pub delegation_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_content_digest: String,
    pub route_adapter_projection_id: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub logical_adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub implementation_digest: String,
    pub capability_set_digest: String,
    pub credential_verifier_digest: String,
    pub launch_policy_digest: String,
    pub network_egress_policy_id: String,
    pub network_egress_policy_revision: u64,
    pub network_egress_policy_digest: String,
    pub service_actor_id: String,
    pub target_policy_digest: String,
    pub target_policy: ExternalPoolAdapterUpstreamTransportTargetPolicy,
    pub dns_hostname: String,
    pub port: u16,
    pub tls_server_name: String,
    pub expected_tls_leaf_spki_sha256: String,
    pub sequence: u64,
    pub predecessor_target_id: Option<String>,
    pub predecessor_target_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_by_actor_user_id: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub target_status: String,
    pub target_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetReceipt {
    pub schema: String,
    pub target_id: String,
    pub target_digest: String,
    pub target_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub target: ExternalPoolAdapterUpstreamTransportTargetMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetRevocationMaterial {
    pub target_id: String,
    pub target_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_id: String,
    pub revoked_by_actor_kind: String,
    pub revoked_by_actor_user_id: String,
    pub reason: String,
    pub revoked_at: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub revocation_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetRevocationReceipt {
    pub schema: String,
    pub revocation_id: String,
    pub revocation_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterUpstreamTransportTargetRevocationMaterial,
}
