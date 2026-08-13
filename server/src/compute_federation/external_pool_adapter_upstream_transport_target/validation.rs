use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::provider::PROVIDER_STATUS_REGISTERING;

use super::*;

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(crate) fn validate_upstream_transport_target_receipt(
    receipt: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        UPSTREAM_TRANSPORT_TARGET_SCHEMA,
        &receipt.target_id,
        &receipt.target_digest,
        &receipt.target_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    let t = &receipt.target;
    identifiers([
        &t.profile_id,
        &t.candidate_id,
        &t.delegation_id,
        &t.provider_binding_id,
        &t.registry_release_id,
        &t.installation_receipt_id,
        &t.route_adapter_projection_id,
        &t.provider_id,
        &t.provider_owner_account_id,
        &t.logical_adapter_id,
        &t.release_version,
        &t.service_actor_id,
        &t.recorded_by_actor_user_id,
        &t.idempotency_scope,
        &t.idempotency_key,
    ])?;
    digests([
        &t.profile_digest,
        &t.candidate_digest,
        &t.delegation_digest,
        &t.provider_binding_digest,
        &t.registry_release_digest,
        &t.installation_receipt_digest,
        &t.installation_content_digest,
        &t.provider_digest,
        &t.implementation_digest,
        &t.capability_set_digest,
        &t.credential_verifier_digest,
        &t.launch_policy_digest,
        &t.network_egress_policy_digest,
        &t.target_policy_digest,
        &t.expected_tls_leaf_spki_sha256,
    ])?;
    optional_identifier_and_digest(&t.predecessor_target_id, &t.predecessor_target_digest)?;
    validate_dns_hostname(&t.dns_hostname)?;
    if t.tls_server_name != t.dns_hostname
        || t.port == 0
        || t.provider_status != PROVIDER_STATUS_REGISTERING
        || t.target_status != UPSTREAM_TRANSPORT_TARGET_STATUS
        || t.target_effect != UPSTREAM_TRANSPORT_TARGET_EFFECT
        || t.confirmation != UPSTREAM_TRANSPORT_TARGET_CONFIRMATION
        || !actor(&t.recorded_by_actor_kind)
        || !opaque_digest(&t.adapter_config_digest)
        || t.provider_policy_revision <= 0
        || t.adapter_config_revision <= 0
        || !safe_positive(t.network_egress_policy_revision)
        || !safe_positive(t.sequence)
        || !no_effects([
            &t.adapter_effect,
            &t.runtime_effect,
            &t.provider_effect,
            &t.credential_effect,
            &t.route_effect,
            &t.execution_effect,
            &t.usage_effect,
            &t.market_effect,
            &t.settlement_effect,
        ])
        || t.broker_connect_ready
        || t.upstream_probe_observed
        || t.runtime_launch_ready
        || t.activation_ready
    {
        bail!("upstream transport target material is not exact");
    }
    canonical_nanos(&t.recorded_at)?;
    validate_policy(&t.target_policy)?;
    if upstream_transport_target_policy_digest(&t.target_policy)? != t.target_policy_digest {
        bail!("upstream transport target policy digest is not exact");
    }
    exact_digests(
        upstream_transport_target_material_digest(t)?,
        &receipt.target_material_digest,
        canonical_upstream_transport_target_json_and_digest(receipt)?.1,
        &receipt.target_digest,
    )
}

pub(crate) fn validate_upstream_transport_target_revocation_receipt(
    receipt: &ExternalPoolAdapterUpstreamTransportTargetRevocationReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        UPSTREAM_TRANSPORT_TARGET_REVOCATION_SCHEMA,
        &receipt.revocation_id,
        &receipt.revocation_digest,
        &receipt.revocation_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    let r = &receipt.revocation;
    identifiers([
        &r.target_id,
        &r.profile_id,
        &r.provider_binding_id,
        &r.provider_id,
        &r.revoked_by_actor_user_id,
        &r.idempotency_scope,
        &r.idempotency_key,
    ])?;
    digests([
        &r.target_digest,
        &r.profile_digest,
        &r.provider_binding_digest,
    ])?;
    if !actor(&r.revoked_by_actor_kind)
        || !reason(&r.reason)
        || r.revoked_at != r.recorded_at
        || r.confirmation != UPSTREAM_TRANSPORT_TARGET_REVOCATION_CONFIRMATION
        || r.revocation_effect != UPSTREAM_TRANSPORT_TARGET_REVOCATION_EFFECT
        || !no_effects([
            &r.adapter_effect,
            &r.runtime_effect,
            &r.provider_effect,
            &r.credential_effect,
            &r.route_effect,
            &r.execution_effect,
            &r.usage_effect,
            &r.market_effect,
            &r.settlement_effect,
        ])
        || r.broker_connect_ready
        || r.upstream_probe_observed
        || r.runtime_launch_ready
        || r.activation_ready
    {
        bail!("upstream transport target revocation material is not exact");
    }
    canonical_nanos(&r.revoked_at)?;
    exact_digests(
        upstream_transport_target_revocation_material_digest(r)?,
        &receipt.revocation_material_digest,
        canonical_upstream_transport_target_revocation_json_and_digest(receipt)?.1,
        &receipt.revocation_digest,
    )
}

pub(crate) fn validate_upstream_transport_target_policy(
    policy: &ExternalPoolAdapterUpstreamTransportTargetPolicy,
) -> Result<()> {
    validate_policy(policy)
}

pub(crate) fn validate_upstream_transport_dns_hostname(value: &str) -> Result<()> {
    validate_dns_hostname(value)
}

fn validate_policy(p: &ExternalPoolAdapterUpstreamTransportTargetPolicy) -> Result<()> {
    identifiers([
        &p.policy_id,
        &p.transport_owner,
        &p.transport_kind,
        &p.hostname_policy,
        &p.port_policy,
        &p.dns_resolution_policy,
        &p.address_selection_policy,
        &p.tls_version_policy,
        &p.tls_server_name_policy,
        &p.tls_chain_policy,
        &p.tls_trust_anchor_policy,
        &p.tls_leaf_identity_policy,
        &p.proxy_policy,
        &p.redirect_policy,
        &p.zero_rtt_policy,
        &p.client_certificate_policy,
        &p.adapter_network_policy,
    ])?;
    if p.policy_id != UPSTREAM_TRANSPORT_TARGET_POLICY_ID
        || p.policy_revision != UPSTREAM_TRANSPORT_TARGET_POLICY_REVISION
        || p.transport_owner != "server_broker_v1"
        || p.transport_kind != "brokered_tls_tcp_v1"
        || p.hostname_policy != "lowercase_ascii_dns_hostname_no_ip_literal_v1"
        || p.port_policy != "explicit_nonzero_u16_v1"
        || p.dns_resolution_policy != "fresh_a_aaaa_all_answers_public_unicast_v1"
        || p.address_selection_policy != "broker_pins_one_validated_address_per_connect_attempt_v1"
        || p.tls_version_policy != "tls_1_3_only_v1"
        || p.tls_server_name_policy != "exact_dns_hostname_v1"
        || p.tls_chain_policy != "future_broker_webpki_chain_hostname_and_time_at_connect_v1"
        || p.tls_trust_anchor_policy != "server_webpki_roots_current_at_connect_v1"
        || p.tls_leaf_identity_policy
            != "expected_leaf_spki_sha256_pin_and_future_webpki_observation_v1"
        || p.proxy_policy != "disabled_v1"
        || p.redirect_policy != "disabled_v1"
        || p.zero_rtt_policy != "disabled_v1"
        || p.client_certificate_policy != "disabled_v1"
        || p.adapter_network_policy != "sidecar_no_network_server_broker_only_v1"
        || p.max_hostname_bytes != 253
        || p.max_dns_answers != 32
        || p.dns_timeout_ms != 3_000
        || p.connect_timeout_ms != 5_000
        || p.tls_handshake_timeout_ms != 5_000
        || p.max_connect_attempts != 2
    {
        bail!("upstream transport target policy differs from the server-fixed catalog");
    }
    Ok(())
}

fn validate_dns_hostname(value: &str) -> Result<()> {
    if !(1..=253).contains(&value.len())
        || !value.is_ascii()
        || value.to_ascii_lowercase() != value
        || !value.contains('.')
        || !value.bytes().any(|byte| byte.is_ascii_lowercase())
        || value.ends_with('.')
    {
        bail!("upstream transport target hostname is not a canonical lowercase A-label DNS name");
    }
    for label in value.split('.') {
        let bytes = label.as_bytes();
        if !(1..=63).contains(&bytes.len())
            || !bytes[0].is_ascii_alphanumeric()
            || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        {
            bail!("upstream transport target hostname label is invalid");
        }
    }
    Ok(())
}

fn metadata(
    schema: &str,
    expected: &str,
    id: &str,
    digest: &str,
    material_digest: &str,
    canonicalization: &str,
    algorithm: &str,
) -> Result<()> {
    identifiers([id])?;
    digests([digest, material_digest])?;
    if schema != expected
        || canonicalization != UPSTREAM_TRANSPORT_TARGET_CANONICALIZATION
        || algorithm != UPSTREAM_TRANSPORT_TARGET_DIGEST_ALGORITHM
    {
        bail!("upstream transport target receipt metadata is not exact");
    }
    Ok(())
}

fn identifiers<S: AsRef<str>>(values: impl IntoIterator<Item = S>) -> Result<()> {
    for value in values {
        let value = value.as_ref();
        if value.is_empty()
            || value.trim() != value
            || value.chars().count() > 240
            || value.chars().any(char::is_control)
        {
            bail!("upstream transport target identifier is invalid");
        }
    }
    Ok(())
}

fn digests<S: AsRef<str>>(values: impl IntoIterator<Item = S>) -> Result<()> {
    for value in values {
        let value = value.as_ref();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("upstream transport target digest is invalid");
        }
    }
    Ok(())
}

fn optional_identifier_and_digest(id: &Option<String>, digest: &Option<String>) -> Result<()> {
    if id.is_some() != digest.is_some() {
        bail!("upstream transport target predecessor identity is incomplete");
    }
    if let Some(id) = id {
        identifiers([id])?;
    }
    if let Some(digest) = digest {
        digests([digest])?;
    }
    Ok(())
}

fn exact_digests(a: String, b: &str, c: String, d: &str) -> Result<()> {
    if a != b || c != d {
        bail!("upstream transport target receipt digest is not exact");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("upstream transport target timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}

fn actor(value: &str) -> bool {
    matches!(
        value,
        UPSTREAM_TRANSPORT_TARGET_ACTOR_PROVIDER_OWNER
            | UPSTREAM_TRANSPORT_TARGET_ACTOR_PLATFORM_ADMIN
    )
}

fn opaque_digest(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.chars().count() <= 512
        && !value.chars().any(char::is_control)
}

fn safe_positive(value: u64) -> bool {
    (1..=MAX_SAFE_INTEGER).contains(&value)
}

fn reason(value: &str) -> bool {
    value.trim() == value
        && (12..=500).contains(&value.chars().count())
        && !value.chars().any(char::is_control)
}

fn no_effects<'a>(values: impl IntoIterator<Item = &'a String>) -> bool {
    values
        .into_iter()
        .all(|value| value == UPSTREAM_TRANSPORT_TARGET_NO_EFFECT)
}
