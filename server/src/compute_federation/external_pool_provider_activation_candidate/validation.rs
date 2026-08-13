use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::provider::{
    PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_REGISTERING,
};

use super::*;

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(crate) fn validate_activation_delegation_receipt(
    receipt: &ExternalPoolProviderActivationDelegationReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        ACTIVATION_DELEGATION_SCHEMA,
        &receipt.delegation_id,
        &receipt.delegation_digest,
        &receipt.delegation_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    let d = &receipt.delegation;
    identifiers([
        &d.provider_binding_id,
        &d.registry_release_id,
        &d.route_adapter_projection_id,
        &d.provider_id,
        &d.provider_owner_account_id,
        &d.logical_adapter_id,
        &d.release_version,
        &d.service_actor_id,
        &d.issued_by_owner_user_id,
        &d.idempotency_scope,
        &d.idempotency_key,
    ])?;
    digests([
        &d.provider_binding_digest,
        &d.registry_release_digest,
        &d.provider_digest,
    ])?;
    optional_identifier_and_digest(
        &d.predecessor_delegation_id,
        &d.predecessor_delegation_digest,
    )?;
    if d.adapter_config_digest.is_empty()
        || d.adapter_config_digest.trim() != d.adapter_config_digest
        || d.adapter_config_digest.chars().count() > 512
        || d.adapter_config_digest.chars().any(char::is_control)
        || d.provider_status != PROVIDER_STATUS_REGISTERING
        || d.service_actor_kind != ACTIVATION_SERVICE_ACTOR_KIND
        || d.allowed_route_kinds != ["server_adapter"]
        || d.allowed_actor_phases != ["application", "dispatch"]
        || d.issued_by_owner_user_id != d.provider_owner_account_id
        || d.issued_at != d.recorded_at
        || d.confirmation != ACTIVATION_CANDIDATE_CONFIRMATION
        || d.delegation_effect != ACTIVATION_DELEGATION_EFFECT
        || d.route_effect != ACTIVATION_ROUTE_CANDIDATE_ONLY
        || !no_effects([
            &d.provider_effect,
            &d.credential_effect,
            &d.execution_effect,
            &d.market_effect,
            &d.settlement_effect,
        ])
        || !valid_revision(d.provider_policy_revision)
        || !valid_revision(d.adapter_config_revision)
        || !valid_sequence(d.sequence)
        || !paired(
            &d.predecessor_delegation_id,
            &d.predecessor_delegation_digest,
        )
    {
        bail!("activation delegation material is not exact");
    }
    canonical_nanos(&d.issued_at)?;
    validate_receipt_digests(
        activation_delegation_material_digest(d)?,
        &receipt.delegation_material_digest,
        canonical_activation_delegation_json_and_digest(receipt)?.1,
        &receipt.delegation_digest,
    )
}

pub(crate) fn validate_activation_candidate_receipt(
    receipt: &ExternalPoolProviderActivationCandidateReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        ACTIVATION_CANDIDATE_SCHEMA,
        &receipt.candidate_id,
        &receipt.candidate_digest,
        &receipt.candidate_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    let c = &receipt.candidate;
    identifiers([
        &c.delegation_id,
        &c.provider_binding_id,
        &c.registry_release_id,
        &c.installation_receipt_id,
        &c.route_adapter_projection_id,
        &c.provider_id,
        &c.provider_owner_account_id,
        &c.logical_adapter_id,
        &c.release_version,
        &c.service_actor_id,
    ])?;
    digests([
        &c.delegation_digest,
        &c.provider_binding_digest,
        &c.registry_release_digest,
        &c.installation_receipt_digest,
        &c.installation_content_digest,
        &c.provider_digest,
        &c.implementation_digest,
        &c.capability_set_digest,
        &c.credential_verifier_digest,
        &c.logical_adapter_binding_digest,
        &c.logical_projection_compatibility_digest,
    ])?;
    optional_identifier_and_digest(&c.predecessor_candidate_id, &c.predecessor_candidate_digest)?;
    if c.adapter_config_digest.is_empty()
        || c.adapter_config_digest.trim() != c.adapter_config_digest
        || c.adapter_config_digest.chars().count() > 512
        || c.adapter_config_digest.chars().any(char::is_control)
        || c.provider_status != PROVIDER_STATUS_REGISTERING
        || c.candidate_status != ACTIVATION_CANDIDATE_STATUS
        || c.activation_closure_status != ACTIVATION_CLOSURE_NOT_IMPLEMENTED
        || c.candidate_effect != ACTIVATION_CANDIDATE_EFFECT
        || c.route_effect != ACTIVATION_ROUTE_CANDIDATE_ONLY
        || !no_effects([
            &c.provider_effect,
            &c.credential_effect,
            &c.execution_effect,
            &c.market_effect,
            &c.settlement_effect,
        ])
        || c.checked_at != c.recorded_at
        || !valid_revision(c.provider_policy_revision)
        || !valid_revision(c.adapter_config_revision)
        || !valid_sequence(c.sequence)
        || !paired(&c.predecessor_candidate_id, &c.predecessor_candidate_digest)
    {
        bail!("activation candidate material is not exact");
    }
    canonical_nanos(&c.checked_at)?;
    validate_receipt_digests(
        activation_candidate_material_digest(c)?,
        &receipt.candidate_material_digest,
        canonical_activation_candidate_json_and_digest(receipt)?.1,
        &receipt.candidate_digest,
    )
}

pub(crate) fn validate_activation_delegation_revocation_receipt(
    receipt: &ExternalPoolProviderActivationDelegationRevocationReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        ACTIVATION_DELEGATION_REVOCATION_SCHEMA,
        &receipt.revocation_id,
        &receipt.revocation_digest,
        &receipt.revocation_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    let r = &receipt.revocation;
    identifiers([
        &r.delegation_id,
        &r.candidate_id,
        &r.provider_binding_id,
        &r.provider_id,
        &r.revoked_by_owner_user_id,
        &r.idempotency_scope,
        &r.idempotency_key,
    ])?;
    digests([
        &r.delegation_digest,
        &r.candidate_digest,
        &r.provider_binding_digest,
    ])?;
    if r.reason.is_empty()
        || r.reason.trim() != r.reason
        || r.reason.chars().count() > 500
        || r.revoked_at != r.recorded_at
        || r.confirmation != ACTIVATION_DELEGATION_REVOCATION_CONFIRMATION
        || r.revocation_effect != ACTIVATION_DELEGATION_REVOCATION_EFFECT
        || !no_effects([
            &r.provider_effect,
            &r.credential_effect,
            &r.route_effect,
            &r.execution_effect,
            &r.market_effect,
            &r.settlement_effect,
        ])
    {
        bail!("activation delegation revocation material is not exact");
    }
    canonical_nanos(&r.revoked_at)?;
    validate_receipt_digests(
        activation_delegation_revocation_material_digest(r)?,
        &receipt.revocation_material_digest,
        canonical_activation_delegation_revocation_json_and_digest(receipt)?.1,
        &receipt.revocation_digest,
    )
}

fn metadata(
    schema: &str,
    expected: &str,
    id: &str,
    digest: &str,
    material: &str,
    canonicalization: &str,
    algorithm: &str,
) -> Result<()> {
    if schema != expected
        || canonicalization != ACTIVATION_CANONICALIZATION
        || algorithm != ACTIVATION_DIGEST_ALGORITHM
    {
        bail!("activation receipt schema is unsupported");
    }
    identifier(id)?;
    digests([digest, material])
}

fn validate_receipt_digests(
    material: String,
    expected_material: &str,
    receipt: String,
    expected_receipt: &str,
) -> Result<()> {
    if material != expected_material || receipt != expected_receipt {
        bail!("activation receipt digest is not exact");
    }
    Ok(())
}

fn identifiers<const N: usize>(values: [&String; N]) -> Result<()> {
    values.into_iter().try_for_each(|value| identifier(value))
}
fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("activation identifier is invalid");
    }
    Ok(())
}
fn digests<const N: usize>(values: [&str; N]) -> Result<()> {
    if values.into_iter().any(|value| {
        value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) {
        bail!("activation digest is invalid");
    }
    Ok(())
}
fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("activation timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
fn paired(id: &Option<String>, digest: &Option<String>) -> bool {
    id.is_some() == digest.is_some()
}
fn optional_identifier_and_digest(id: &Option<String>, digest: &Option<String>) -> Result<()> {
    match (id, digest) {
        (Some(id), Some(digest)) => {
            identifier(id)?;
            digests([digest.as_str()])
        }
        (None, None) => Ok(()),
        _ => bail!("activation predecessor identity is incomplete"),
    }
}
fn valid_revision(value: i64) -> bool {
    value > 0 && value as u64 <= MAX_SAFE_INTEGER
}
fn valid_sequence(value: u64) -> bool {
    value > 0 && value <= MAX_SAFE_INTEGER
}
fn no_effects<const N: usize>(values: [&String; N]) -> bool {
    values
        .into_iter()
        .all(|value| value == ACTIVATION_NO_EFFECT)
}

pub(crate) fn validate_activation_provider_kind(kind: &str) -> Result<()> {
    if kind != PROVIDER_KIND_EXTERNAL_POOL {
        bail!("activation candidate requires an external_pool Provider");
    }
    Ok(())
}
