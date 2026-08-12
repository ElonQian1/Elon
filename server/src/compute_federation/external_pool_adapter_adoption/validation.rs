use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use super::{canonical::*, types::*};

pub(crate) fn validate_adoption_receipt(
    receipt: &ExternalPoolAdapterAdoptionReceipt,
) -> Result<()> {
    if receipt.schema != ADOPTION_RECEIPT_SCHEMA
        || receipt.canonicalization != ADOPTION_CANONICALIZATION
        || receipt.digest_algorithm != ADOPTION_DIGEST_ALGORITHM
    {
        bail!("Adapter adoption receipt metadata is unsupported");
    }
    identifier(&receipt.adoption_receipt_id, 200)?;
    digest(&receipt.adoption_receipt_digest)?;
    digest(&receipt.adoption_material_digest)?;
    let item = &receipt.adoption;
    validate_binding(&item.binding)?;
    identifier(&item.adopted_by_admin_user_id, 200)?;
    identifier(&item.idempotency_scope, 240)?;
    identifier(&item.idempotency_key, 240)?;
    canonical_nanos(&item.adopted_at)?;
    canonical_nanos(&item.recorded_at)?;
    if item.adopted_at != item.recorded_at
        || item.confirmation != ADOPTION_CONFIRMATION
        || item.adoption_effect != ADOPTION_AUTHORITY_EFFECT
        || item.install_effect != ADOPTION_INSTALL_EFFECT
        || item.provider_effect != ADOPTION_NO_EFFECT
        || item.route_effect != ADOPTION_NO_EFFECT
        || item.execution_effect != ADOPTION_NO_EFFECT
        || item.settlement_effect != ADOPTION_NO_EFFECT
        || adoption_material_digest(item)? != receipt.adoption_material_digest
    {
        bail!("Adapter adoption receipt effects or material digest are invalid");
    }
    Ok(())
}

pub(crate) fn validate_adoption_terminal_receipt(
    receipt: &ExternalPoolAdapterAdoptionTerminalReceipt,
) -> Result<()> {
    if receipt.schema != ADOPTION_TERMINAL_RECEIPT_SCHEMA
        || receipt.canonicalization != ADOPTION_CANONICALIZATION
        || receipt.digest_algorithm != ADOPTION_DIGEST_ALGORITHM
    {
        bail!("Adapter adoption terminal receipt metadata is unsupported");
    }
    identifier(&receipt.terminal_receipt_id, 200)?;
    digest(&receipt.terminal_receipt_digest)?;
    digest(&receipt.terminal_material_digest)?;
    let item = &receipt.terminal;
    identifier(&item.adoption_receipt_id, 200)?;
    digest(&item.adoption_receipt_digest)?;
    identifier(&item.revoked_by_admin_user_id, 200)?;
    identifier(&item.reason, 1000)?;
    identifier(&item.idempotency_scope, 240)?;
    identifier(&item.idempotency_key, 240)?;
    canonical_nanos(&item.revoked_at)?;
    if item.recorded_at != item.revoked_at
        || item.confirmation != ADOPTION_REVOCATION_CONFIRMATION
        || item.adoption_effect != ADOPTION_REVOKED_EFFECT
        || item.provider_effect != ADOPTION_NO_EFFECT
        || item.route_effect != ADOPTION_NO_EFFECT
        || item.execution_effect != ADOPTION_NO_EFFECT
        || item.settlement_effect != ADOPTION_NO_EFFECT
        || adoption_terminal_material_digest(item)? != receipt.terminal_material_digest
    {
        bail!("Adapter adoption terminal effects or material digest are invalid");
    }
    Ok(())
}

fn validate_binding(binding: &ExternalPoolAdapterAdoptionBinding) -> Result<()> {
    for value in [
        &binding.application_id,
        &binding.provider_id,
        &binding.provider_owner_account_id,
        &binding.admission_id,
        &binding.adapter_id,
        &binding.adapter_release_version,
        &binding.sandbox_conformance_receipt_id,
        &binding.credential_verification_receipt_id,
    ] {
        identifier(value, 200)?;
    }
    for value in [
        &binding.application_digest,
        &binding.provider_digest,
        &binding.admission_digest,
        &binding.declared_implementation_sha256,
        &binding.capability_set_digest,
        &binding.sandbox_conformance_receipt_digest,
        &binding.credential_verification_receipt_digest,
        &binding.credential_locator_commitment,
    ] {
        digest(value)?;
    }
    if binding.provider_policy_revision < 1 || binding.adapter_config_revision < 1 {
        bail!("Adapter adoption revision is invalid");
    }
    identifier(&binding.adapter_config_digest, 512)?;
    canonical_nanos(&binding.sandbox_report_expires_at)?;
    canonical_nanos(&binding.credential_report_expires_at)?;
    Ok(())
}

fn identifier(value: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("Adapter adoption identifier is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("Adapter adoption digest is invalid");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value {
        bail!("Adapter adoption timestamp is not canonical UTC nanoseconds");
    }
    Ok(parsed)
}
