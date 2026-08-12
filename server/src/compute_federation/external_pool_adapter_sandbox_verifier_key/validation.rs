use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};
use rsa::{
    pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding},
    traits::PublicKeyParts,
    RsaPublicKey,
};
use sha2::{Digest, Sha256};

use super::{canonical::*, types::*};

pub(crate) fn validate_sandbox_verifier_key_record(
    value: &ExternalPoolAdapterSandboxVerifierKeyRecord,
) -> Result<()> {
    if value.schema != SANDBOX_VERIFIER_KEY_RECORD_SCHEMA
        || value.canonicalization != SANDBOX_VERIFIER_KEY_CANONICALIZATION
        || value.digest_algorithm != SANDBOX_VERIFIER_KEY_DIGEST_ALGORITHM
    {
        bail!("sandbox-verifier-key record metadata is unsupported");
    }
    identifier(&value.key_record_id, 160)?;
    digest(&value.key_record_digest)?;
    digest(&value.registration_material_digest)?;
    registration(&value.registration)?;
    if sandbox_verifier_key_registration_digest(&value.registration)?
        != value.registration_material_digest
        || sandbox_verifier_key_record_json_and_digest(value)?.1 != value.key_record_digest
    {
        bail!("sandbox-verifier-key record is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_sandbox_verifier_key_transition(
    value: &ExternalPoolAdapterSandboxVerifierKeyTransitionReceipt,
) -> Result<()> {
    if value.canonicalization != SANDBOX_VERIFIER_KEY_CANONICALIZATION
        || value.digest_algorithm != SANDBOX_VERIFIER_KEY_DIGEST_ALGORITHM
        || !matches!(
            value.schema.as_str(),
            SANDBOX_VERIFIER_KEY_ACTIVATION_SCHEMA | SANDBOX_VERIFIER_KEY_REVOCATION_SCHEMA
        )
    {
        bail!("sandbox-verifier-key transition metadata is unsupported");
    }
    identifier(&value.transition_receipt_id, 160)?;
    digest(&value.transition_receipt_digest)?;
    digest(&value.transition_material_digest)?;
    transition(&value.transition, &value.schema)?;
    if sandbox_verifier_key_transition_digest(&value.transition)?
        != value.transition_material_digest
        || sandbox_verifier_key_transition_json_and_digest(value)?.1
            != value.transition_receipt_digest
    {
        bail!("sandbox-verifier-key transition is not canonical");
    }
    Ok(())
}

fn registration(value: &ExternalPoolAdapterSandboxVerifierKeyRegistration) -> Result<()> {
    text(&value.verifier_operator, 1, 160)?;
    text(&value.verifier_product, 1, 160)?;
    digest(&value.key_id)?;
    identifier(&value.created_by_admin_user_id, 160)?;
    identifier(&value.idempotency_scope, 200)?;
    identifier(&value.idempotency_key, 160)?;
    timestamp_pair(&value.created_at, &value.recorded_at)?;
    let public = RsaPublicKey::from_public_key_pem(&value.public_key_pem)?;
    let canonical_pem = public.to_public_key_pem(LineEnding::LF)?;
    let der = public.to_public_key_der()?;
    if value.algorithm != SANDBOX_VERIFIER_KEY_ALGORITHM
        || value.actor_kind != SANDBOX_VERIFIER_KEY_ACTOR_KIND
        || value.confirmation != SANDBOX_VERIFIER_KEY_REGISTER_CONFIRMATION
        || value.currentness_effect != SANDBOX_VERIFIER_KEY_STATUS_PENDING
        || !(2048..=8192).contains(&public.n().bits())
        || value.public_key_pem != canonical_pem
        || value.key_id != hex::encode(Sha256::digest(der.as_bytes()))
        || !no_effects(value)
    {
        bail!("sandbox-verifier-key registration is invalid");
    }
    Ok(())
}

fn transition(value: &ExternalPoolAdapterSandboxVerifierKeyTransition, schema: &str) -> Result<()> {
    identifier(&value.key_record_id, 160)?;
    digest(&value.key_record_digest)?;
    digest(&value.key_id)?;
    text(&value.verifier_operator, 1, 160)?;
    text(&value.verifier_product, 1, 160)?;
    identifier(&value.actor_user_id, 160)?;
    identifier(&value.idempotency_scope, 200)?;
    identifier(&value.idempotency_key, 160)?;
    timestamp_pair(&value.occurred_at, &value.recorded_at)?;
    let activation = schema == SANDBOX_VERIFIER_KEY_ACTIVATION_SCHEMA;
    if value.actor_kind != SANDBOX_VERIFIER_KEY_ACTOR_KIND
        || value.confirmation
            != if activation {
                SANDBOX_VERIFIER_KEY_ACTIVATE_CONFIRMATION
            } else {
                SANDBOX_VERIFIER_KEY_REVOKE_CONFIRMATION
            }
        || value.currentness_effect
            != if activation {
                SANDBOX_VERIFIER_KEY_STATUS_ACTIVE
            } else {
                SANDBOX_VERIFIER_KEY_STATUS_REVOKED
            }
        || activation != value.reason.is_none()
        || value
            .reason
            .as_deref()
            .is_some_and(|reason| text(reason, 8, 2_000).is_err())
        || [
            &value.conformance_report_effect,
            &value.vulnerability_report_effect,
            &value.adapter_effect,
            &value.route_effect,
        ]
        .into_iter()
        .any(|effect| effect != SANDBOX_VERIFIER_KEY_NO_EFFECT)
    {
        bail!("sandbox-verifier-key transition is invalid");
    }
    Ok(())
}

fn no_effects(value: &ExternalPoolAdapterSandboxVerifierKeyRegistration) -> bool {
    [
        &value.conformance_report_effect,
        &value.vulnerability_report_effect,
        &value.adapter_effect,
        &value.route_effect,
    ]
    .into_iter()
    .all(|effect| effect == SANDBOX_VERIFIER_KEY_NO_EFFECT)
}

fn timestamp_pair(first: &str, second: &str) -> Result<()> {
    if first != second {
        bail!("sandbox-verifier-key timestamps differ");
    }
    let parsed = DateTime::parse_from_rfc3339(first)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != first
    {
        bail!("sandbox-verifier-key timestamp is not canonical UTC nanoseconds");
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
        bail!("sandbox-verifier-key text is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("sandbox-verifier-key digest is invalid");
    }
    Ok(())
}
