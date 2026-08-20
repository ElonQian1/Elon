use anyhow::{bail, ensure, Result};
use chrono::{DateTime, Duration, SecondsFormat};

use crate::compute_federation::route_authority::{
    canonical_route_capability_set_digest, COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_ACK,
    COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_EVENTS, COMPUTE_ROUTE_CAPABILITY_CANCEL_NO_START,
    COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT, COMPUTE_ROUTE_CAPABILITY_PREPARE,
    COMPUTE_ROUTE_CAPABILITY_RECONCILE, COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT,
};

use super::*;

pub(crate) fn validate_external_pool_adapter_route_renewal_receipt(
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
) -> Result<()> {
    ensure!(
        receipt.schema == ROUTE_RENEWAL_RECEIPT_SCHEMA
            && receipt.canonicalization == ROUTE_RENEWAL_CANONICALIZATION
            && receipt.digest_algorithm == ROUTE_RENEWAL_DIGEST_ALGORITHM,
        "V278 receipt envelope is not exact"
    );
    let renewal = &receipt.renewal;
    let identity = &renewal.identity;
    ensure!(
        identity.renewal_sequence > 0,
        "V278 sequence is not positive"
    );
    ensure!(
        identity.predecessor_route_renewal_receipt_id.is_some()
            == identity.predecessor_route_renewal_receipt_digest.is_some()
            && (identity.renewal_sequence == 1)
                == identity.predecessor_route_renewal_receipt_id.is_none(),
        "V278 predecessor pair is not exact"
    );
    let route = &renewal.renewed_route;
    let expected = [
        COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_ACK,
        COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_EVENTS,
        COMPUTE_ROUTE_CAPABILITY_CANCEL_NO_START,
        COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT,
        COMPUTE_ROUTE_CAPABILITY_PREPARE,
        COMPUTE_ROUTE_CAPABILITY_RECONCILE,
    ];
    ensure!(
        route.route_capabilities.len() == COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT as usize
            && route
                .route_capabilities
                .iter()
                .zip(expected)
                .enumerate()
                .all(
                    |(ordinal, (capability, expected_id))| capability.ordinal == ordinal as i64
                        && capability.capability_id == expected_id
                        && capability.capability_revision > 0
                ),
        "V278 capability order is not exact"
    );
    ensure!(
        canonical_route_capability_set_digest(&route.route_capabilities)?
            == route.route_capability_set_digest,
        "V278 capability digest is not exact"
    );
    let timing = &renewal.timing;
    let checked = timestamp(&timing.evidence_checked_at)?;
    let authenticated = timestamp(&timing.authenticated_at)?;
    let authorized = timestamp(&timing.authorized_at)?;
    let expires = timestamp(&timing.expires_at)?;
    let cleanup = timestamp(&timing.cleanup_expires_at)?;
    let renew_at = checked
        .checked_add_signed(Duration::seconds(ROUTE_RENEWAL_RENEW_BEFORE_SECONDS))
        .ok_or_else(|| anyhow::anyhow!("V278 renew-before time overflow"))?;
    let fresh_max = checked
        .checked_add_signed(Duration::seconds(ROUTE_RENEWAL_FRESH_MAX_SECONDS))
        .ok_or_else(|| anyhow::anyhow!("V278 fresh maximum time overflow"))?;
    let cleanup_max = checked
        .checked_add_signed(Duration::seconds(ROUTE_RENEWAL_CLEANUP_MAX_SECONDS))
        .ok_or_else(|| anyhow::anyhow!("V278 cleanup maximum time overflow"))?;
    ensure!(
        timing.created_at == timing.evidence_checked_at
            && authenticated == checked
            && authorized == checked
            && renew_at < expires
            && expires <= fresh_max
            && expires < cleanup
            && cleanup <= cleanup_max,
        "V278 dual-time or TTL order is not exact"
    );
    ensure!(
        renewal.audit.renewed_by_actor_kind == ROUTE_RENEWAL_ACTOR_KIND
            && renewal.audit.renewed_by_service_actor_id == route.service_actor_id
            && renewal.audit.renewal_policy_digest
                == canonical_external_pool_adapter_route_renewal_policy_digest()?,
        "V278 policy or actor audit is not exact"
    );
    let idempotency = ExternalPoolAdapterRouteRenewalIdempotencyMaterial {
        provider_binding_id: identity.provider_binding_id.clone(),
        activation_receipt_id: renewal.activation_witness.activation_receipt_id.clone(),
        activation_root_digest: identity.activation_root_digest.clone(),
        renewal_sequence: identity.renewal_sequence,
        predecessor_route_renewal_receipt_id: identity.predecessor_route_renewal_receipt_id.clone(),
        predecessor_route_renewal_receipt_digest: identity
            .predecessor_route_renewal_receipt_digest
            .clone(),
        credential_reattestation_receipt_id: renewal
            .credential_evidence
            .credential_reattestation_receipt_id
            .clone(),
        credential_reattestation_receipt_digest: renewal
            .credential_evidence
            .credential_reattestation_receipt_digest
            .clone(),
        evidence_checked_at: timing.evidence_checked_at.clone(),
    };
    let (idempotency_json, idempotency_digest) =
        canonical_external_pool_adapter_route_renewal_idempotency_json_and_digest(&idempotency)?;
    ensure!(
        renewal.audit.idempotency_material_json == idempotency_json
            && renewal.audit.idempotency_digest == idempotency_digest
            && receipt.route_renewal_receipt_id
                == derive_external_pool_adapter_route_renewal_receipt_id(&idempotency_digest)?,
        "V278 idempotency or receipt identity is not exact"
    );
    let (canonical, digest) =
        canonical_external_pool_adapter_route_renewal_receipt_json_and_digest(receipt)?;
    ensure!(
        canonical.len() <= ROUTE_RENEWAL_MAX_JSON_BYTES
            && digest == receipt.route_renewal_receipt_digest,
        "V278 canonical receipt digest is not exact"
    );
    validate_scalar_roots(receipt)
}

fn validate_scalar_roots(receipt: &ExternalPoolAdapterRouteRenewalReceipt) -> Result<()> {
    let r = &receipt.renewal;
    let digests = [
        receipt.route_renewal_receipt_digest.as_str(),
        r.identity.provider_binding_digest.as_str(),
        r.identity.activation_root_digest.as_str(),
        r.activation_witness.activation_receipt_digest.as_str(),
        r.activation_witness
            .activation_genesis_successor_receipt_digest
            .as_str(),
        r.active_subject.active_provider_digest.as_str(),
        r.stable_binding.stable_executor_binding_digest.as_str(),
        r.stable_binding
            .projected_v211_adapter_binding_digest
            .as_str(),
        r.stable_binding.route_adapter_digest.as_str(),
        r.predecessor_route
            .service_actor_authorization_digest
            .as_str(),
        r.predecessor_route.route_credential_digest.as_str(),
        r.predecessor_route.route_authorization_digest.as_str(),
        r.predecessor_route.route_seal_digest.as_str(),
        r.credential_evidence
            .credential_reattestation_receipt_digest
            .as_str(),
        r.renewed_route.service_actor_authorization_digest.as_str(),
        r.renewed_route.route_credential_digest.as_str(),
        r.renewed_route.route_authorization_digest.as_str(),
        r.renewed_route.route_capability_set_digest.as_str(),
        r.renewed_route.route_seal_digest.as_str(),
        r.audit.delegation_digest.as_str(),
        r.audit.renewal_policy_digest.as_str(),
        r.audit.idempotency_digest.as_str(),
    ];
    if !digests.into_iter().all(is_digest) {
        bail!("V278 scalar digest is malformed")
    }
    ensure!(
        r.active_subject.active_provider_policy_revision > 0
            && r.stable_binding.route_adapter_revision > 0
            && r.predecessor_route.route_credential_revision > 0
            && r.predecessor_route.route_authorization_revision > 0
            && r.renewed_route.service_actor_authorization_revision > 0
            && r.renewed_route.route_credential_revision > 0
            && r.renewed_route.route_authorization_revision > 0,
        "V278 revision is not positive"
    );
    Ok(())
}

fn timestamp(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    ensure!(
        parsed.offset().local_minus_utc() == 0
            && parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == value,
        "V278 timestamp is not canonical"
    );
    Ok(parsed)
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
