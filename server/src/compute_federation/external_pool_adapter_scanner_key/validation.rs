use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};
use rsa::{
    pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding},
    traits::PublicKeyParts,
    RsaPublicKey,
};
use sha2::{Digest, Sha256};

use super::{canonical::*, types::*};

pub(crate) fn validate_scanner_key_record(
    value: &ExternalPoolAdapterScannerKeyRecord,
) -> Result<()> {
    if value.schema != SCANNER_KEY_RECORD_SCHEMA
        || value.canonicalization != SCANNER_KEY_CANONICALIZATION
        || value.digest_algorithm != SCANNER_KEY_DIGEST_ALGORITHM
    {
        bail!("scanner-key record metadata is unsupported");
    }
    identifier(&value.key_record_id, 160)?;
    digest(&value.key_record_digest)?;
    digest(&value.registration_material_digest)?;
    registration(&value.registration)?;
    if scanner_key_registration_digest(&value.registration)? != value.registration_material_digest
        || scanner_key_record_json_and_digest(value)?.1 != value.key_record_digest
    {
        bail!("scanner-key record is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_scanner_key_activation(
    value: &ExternalPoolAdapterScannerKeyActivationReceipt,
) -> Result<()> {
    if value.schema != SCANNER_KEY_ACTIVATION_RECEIPT_SCHEMA
        || value.canonicalization != SCANNER_KEY_CANONICALIZATION
        || value.digest_algorithm != SCANNER_KEY_DIGEST_ALGORITHM
    {
        bail!("scanner-key activation metadata is unsupported");
    }
    identifier(&value.activation_receipt_id, 160)?;
    digest(&value.activation_receipt_digest)?;
    digest(&value.activation_material_digest)?;
    common(
        &value.activation.key_record_id,
        &value.activation.key_record_digest,
        &value.activation.key_id,
        &value.activation.scanner_operator,
        &value.activation.scanner_product,
        &value.activation.actor_kind,
        &value.activation.activated_by_admin_user_id,
        &value.activation.idempotency_scope,
        &value.activation.idempotency_key,
        &value.activation.occurred_at,
        &value.activation.recorded_at,
    )?;
    if value.activation.confirmation != SCANNER_KEY_ACTIVATE_CONFIRMATION
        || value.activation.currentness_effect != SCANNER_KEY_STATUS_ACTIVE
        || !no_effects(
            &value.activation.vulnerability_report_effect,
            &value.activation.artifact_security_effect,
            &value.activation.conformance_effect,
            &value.activation.adapter_effect,
            &value.activation.route_effect,
        )
        || scanner_key_activation_digest(&value.activation)? != value.activation_material_digest
        || scanner_key_activation_json_and_digest(value)?.1 != value.activation_receipt_digest
    {
        bail!("scanner-key activation is not exact");
    }
    Ok(())
}

pub(crate) fn validate_scanner_key_revocation(
    value: &ExternalPoolAdapterScannerKeyRevocationReceipt,
) -> Result<()> {
    if value.schema != SCANNER_KEY_REVOCATION_RECEIPT_SCHEMA
        || value.canonicalization != SCANNER_KEY_CANONICALIZATION
        || value.digest_algorithm != SCANNER_KEY_DIGEST_ALGORITHM
    {
        bail!("scanner-key revocation metadata is unsupported");
    }
    identifier(&value.revocation_receipt_id, 160)?;
    digest(&value.revocation_receipt_digest)?;
    digest(&value.revocation_material_digest)?;
    common(
        &value.revocation.key_record_id,
        &value.revocation.key_record_digest,
        &value.revocation.key_id,
        &value.revocation.scanner_operator,
        &value.revocation.scanner_product,
        &value.revocation.actor_kind,
        &value.revocation.revoked_by_admin_user_id,
        &value.revocation.idempotency_scope,
        &value.revocation.idempotency_key,
        &value.revocation.occurred_at,
        &value.revocation.recorded_at,
    )?;
    text(&value.revocation.reason, 8, 2_000)?;
    if value.revocation.confirmation != SCANNER_KEY_REVOKE_CONFIRMATION
        || value.revocation.currentness_effect != SCANNER_KEY_STATUS_REVOKED
        || !no_effects(
            &value.revocation.vulnerability_report_effect,
            &value.revocation.artifact_security_effect,
            &value.revocation.conformance_effect,
            &value.revocation.adapter_effect,
            &value.revocation.route_effect,
        )
        || scanner_key_revocation_digest(&value.revocation)? != value.revocation_material_digest
        || scanner_key_revocation_json_and_digest(value)?.1 != value.revocation_receipt_digest
    {
        bail!("scanner-key revocation is not exact");
    }
    Ok(())
}

fn registration(value: &ExternalPoolAdapterScannerKeyRegistration) -> Result<()> {
    text(&value.scanner_operator, 1, 160)?;
    text(&value.scanner_product, 1, 160)?;
    digest(&value.key_id)?;
    identifier(&value.created_by_admin_user_id, 160)?;
    identifier(&value.idempotency_scope, 200)?;
    identifier(&value.idempotency_key, 160)?;
    timestamp_pair(&value.created_at, &value.recorded_at)?;
    let public = RsaPublicKey::from_public_key_pem(&value.public_key_pem)?;
    let canonical_pem = public.to_public_key_pem(LineEnding::LF)?;
    let der = public.to_public_key_der()?;
    if value.algorithm != SCANNER_KEY_ALGORITHM
        || value.actor_kind != SCANNER_KEY_ACTOR_KIND
        || value.confirmation != SCANNER_KEY_REGISTER_CONFIRMATION
        || value.currentness_effect != SCANNER_KEY_STATUS_PENDING
        || value.public_key_pem.is_empty()
        || value.public_key_pem.len() > 16 * 1024
        || !(2048..=8192).contains(&public.n().bits())
        || value.public_key_pem != canonical_pem
        || value.key_id != hex::encode(Sha256::digest(der.as_bytes()))
        || !no_effects(
            &value.vulnerability_report_effect,
            &value.artifact_security_effect,
            &value.conformance_effect,
            &value.adapter_effect,
            &value.route_effect,
        )
    {
        bail!("scanner-key registration is invalid");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn common(
    record_id: &str,
    record_digest: &str,
    key_id: &str,
    operator: &str,
    product: &str,
    actor_kind: &str,
    actor_id: &str,
    scope: &str,
    key: &str,
    occurred_at: &str,
    recorded_at: &str,
) -> Result<()> {
    identifier(record_id, 160)?;
    digest(record_digest)?;
    digest(key_id)?;
    text(operator, 1, 160)?;
    text(product, 1, 160)?;
    identifier(actor_id, 160)?;
    identifier(scope, 200)?;
    identifier(key, 160)?;
    timestamp_pair(occurred_at, recorded_at)?;
    if actor_kind != SCANNER_KEY_ACTOR_KIND {
        bail!("scanner-key actor kind is invalid");
    }
    Ok(())
}

fn no_effects(values: &str, security: &str, conformance: &str, adapter: &str, route: &str) -> bool {
    [values, security, conformance, adapter, route]
        .into_iter()
        .all(|value| value == SCANNER_KEY_NO_EFFECT)
}

fn timestamp_pair(first: &str, second: &str) -> Result<()> {
    if first != second {
        bail!("scanner-key timestamps differ");
    }
    let parsed = DateTime::parse_from_rfc3339(first)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != first
    {
        bail!("scanner-key timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}

fn identifier(value: &str, max: usize) -> Result<()> {
    text(value, 1, max)
}

fn text(value: &str, min: usize, max: usize) -> Result<()> {
    if value.trim() != value
        || !(min..=max).contains(&value.chars().count())
        || value.chars().any(char::is_control)
    {
        bail!("scanner-key text is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("scanner-key digest is invalid");
    }
    Ok(())
}
