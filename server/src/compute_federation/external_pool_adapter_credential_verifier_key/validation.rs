use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};
use rsa::{
    pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding},
    traits::PublicKeyParts,
    RsaPublicKey,
};
use sha2::{Digest, Sha256};

use super::{canonical::*, types::*};

pub(crate) fn validate_record(value: &CredentialVerifierKeyRecord) -> Result<()> {
    if value.schema != RECORD_SCHEMA
        || value.canonicalization != CANONICALIZATION
        || value.digest_algorithm != DIGEST_ALGORITHM
    {
        bail!("credential-verifier-key metadata is unsupported");
    }
    identifier(&value.key_record_id, 160)?;
    digest(&value.key_record_digest)?;
    digest(&value.registration_material_digest)?;
    registration(&value.registration)?;
    if registration_digest(&value.registration)? != value.registration_material_digest
        || record_json_and_digest(value)?.1 != value.key_record_digest
    {
        bail!("credential-verifier-key record is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_revocation(value: &CredentialVerifierKeyRevocationReceipt) -> Result<()> {
    if value.schema != REVOCATION_SCHEMA
        || value.canonicalization != CANONICALIZATION
        || value.digest_algorithm != DIGEST_ALGORITHM
    {
        bail!("credential-verifier-key revocation metadata is unsupported");
    }
    identifier(&value.revocation_receipt_id, 160)?;
    digest(&value.revocation_receipt_digest)?;
    digest(&value.revocation_material_digest)?;
    let item = &value.revocation;
    identifier(&item.key_record_id, 160)?;
    digest(&item.key_record_digest)?;
    identifier(&item.verifier_record_id, 160)?;
    digest(&item.verifier_record_digest)?;
    digest(&item.key_id)?;
    identifier(&item.revoked_by_admin_user_id, 160)?;
    text(&item.reason, 8, 2_000)?;
    identifier(&item.idempotency_scope, 200)?;
    identifier(&item.idempotency_key, 160)?;
    timestamp_pair(&item.revoked_at, &item.recorded_at)?;
    if item.actor_kind != ACTOR_KIND
        || item.confirmation != REVOKE_CONFIRMATION
        || item.currentness_effect != STATUS_REVOKED
        || !no_effects(
            &item.credential_receipt_effect,
            &item.adapter_effect,
            &item.route_effect,
        )
        || revocation_digest(item)? != value.revocation_material_digest
        || revocation_json_and_digest(value)?.1 != value.revocation_receipt_digest
    {
        bail!("credential-verifier-key revocation is invalid");
    }
    Ok(())
}

fn registration(value: &CredentialVerifierKeyRegistration) -> Result<()> {
    identifier(&value.verifier_record_id, 160)?;
    digest(&value.verifier_record_digest)?;
    text(&value.verifier_operator, 1, 160)?;
    text(&value.verifier_product, 1, 160)?;
    identifier(&value.verification_kind, 80)?;
    identifier(&value.verifier_id, 160)?;
    if !(1..=9_007_199_254_740_991).contains(&value.verifier_revision) {
        bail!("verifier revision is invalid");
    }
    digest(&value.verifier_digest)?;
    digest(&value.key_id)?;
    identifier(&value.created_by_admin_user_id, 160)?;
    identifier(&value.idempotency_scope, 200)?;
    identifier(&value.idempotency_key, 160)?;
    timestamp_pair(&value.created_at, &value.recorded_at)?;
    let public = RsaPublicKey::from_public_key_pem(&value.public_key_pem)?;
    let canonical = public.to_public_key_pem(LineEnding::LF)?;
    let der = public.to_public_key_der()?;
    if value.algorithm != KEY_ALGORITHM
        || value.actor_kind != ACTOR_KIND
        || value.confirmation != REGISTER_CONFIRMATION
        || value.currentness_effect != STATUS_ACTIVE
        || !(2048..=8192).contains(&public.n().bits())
        || value.public_key_pem != canonical
        || value.key_id != hex::encode(Sha256::digest(der.as_bytes()))
        || !no_effects(
            &value.credential_receipt_effect,
            &value.adapter_effect,
            &value.route_effect,
        )
    {
        bail!("credential-verifier-key registration is invalid");
    }
    Ok(())
}

fn no_effects(a: &str, b: &str, c: &str) -> bool {
    [a, b, c].into_iter().all(|x| x == NO_EFFECT)
}
fn timestamp_pair(a: &str, b: &str) -> Result<()> {
    if a != b {
        bail!("credential-verifier-key timestamps differ");
    }
    let parsed = DateTime::parse_from_rfc3339(a)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != a
    {
        bail!("timestamp is not canonical");
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
        bail!("credential-verifier-key text is invalid");
    }
    Ok(())
}
fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        bail!("credential-verifier-key digest is invalid");
    }
    Ok(())
}
