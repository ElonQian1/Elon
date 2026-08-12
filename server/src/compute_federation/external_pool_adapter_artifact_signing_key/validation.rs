use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use super::{
    canonical::{
        canonical_signing_key_activation_json_and_digest,
        canonical_signing_key_record_json_and_digest,
        canonical_signing_key_revocation_json_and_digest, signing_key_activation_material_digest,
        signing_key_registration_material_digest, signing_key_revocation_material_digest,
    },
    types::*,
};

pub(crate) fn validate_signing_key_record(
    record: &ExternalPoolAdapterArtifactSigningKeyRecord,
) -> Result<()> {
    if record.schema != SIGNING_KEY_RECORD_SCHEMA
        || record.canonicalization != SIGNING_KEY_CANONICALIZATION
        || record.digest_algorithm != SIGNING_KEY_DIGEST_ALGORITHM
    {
        bail!("signing-key record metadata is unsupported");
    }
    validate_identifier(&record.key_record_id, "key record ID", 160)?;
    validate_digest(&record.key_record_digest, "key record digest")?;
    validate_digest(
        &record.registration_material_digest,
        "registration material digest",
    )?;
    validate_registration(&record.registration)?;
    if signing_key_registration_material_digest(&record.registration)?
        != record.registration_material_digest
    {
        bail!("signing-key registration material digest is not canonical");
    }
    if canonical_signing_key_record_json_and_digest(record)?.1 != record.key_record_digest {
        bail!("signing-key record digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_signing_key_activation_receipt(
    receipt: &ExternalPoolAdapterArtifactSigningKeyActivationReceipt,
) -> Result<()> {
    if receipt.schema != SIGNING_KEY_ACTIVATION_RECEIPT_SCHEMA
        || receipt.canonicalization != SIGNING_KEY_CANONICALIZATION
        || receipt.digest_algorithm != SIGNING_KEY_DIGEST_ALGORITHM
    {
        bail!("signing-key activation metadata is unsupported");
    }
    validate_identifier(&receipt.activation_receipt_id, "activation receipt ID", 160)?;
    validate_digest(
        &receipt.activation_receipt_digest,
        "activation receipt digest",
    )?;
    validate_digest(
        &receipt.activation_material_digest,
        "activation material digest",
    )?;
    validate_common(
        &receipt.activation.key_record_id,
        &receipt.activation.key_record_digest,
        &receipt.activation.key_id,
        &receipt.activation.source_operator,
        &receipt.activation.actor_kind,
        &receipt.activation.activated_by_admin_user_id,
        &receipt.activation.idempotency_scope,
        &receipt.activation.idempotency_key,
        &receipt.activation.occurred_at,
        &receipt.activation.recorded_at,
    )?;
    if receipt.activation.confirmation != SIGNING_KEY_ACTIVATION_CONFIRMATION
        || receipt.activation.currentness_effect != SIGNING_KEY_STATUS_ACTIVE
        || !has_no_business_effect(
            &receipt.activation.artifact_signature_effect,
            &receipt.activation.adapter_effect,
            &receipt.activation.route_effect,
        )
    {
        bail!("signing-key activation effects are not exact");
    }
    if signing_key_activation_material_digest(&receipt.activation)?
        != receipt.activation_material_digest
        || canonical_signing_key_activation_json_and_digest(receipt)?.1
            != receipt.activation_receipt_digest
    {
        bail!("signing-key activation digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_signing_key_revocation_receipt(
    receipt: &ExternalPoolAdapterArtifactSigningKeyRevocationReceipt,
) -> Result<()> {
    if receipt.schema != SIGNING_KEY_REVOCATION_RECEIPT_SCHEMA
        || receipt.canonicalization != SIGNING_KEY_CANONICALIZATION
        || receipt.digest_algorithm != SIGNING_KEY_DIGEST_ALGORITHM
    {
        bail!("signing-key revocation metadata is unsupported");
    }
    validate_identifier(&receipt.revocation_receipt_id, "revocation receipt ID", 160)?;
    validate_digest(
        &receipt.revocation_receipt_digest,
        "revocation receipt digest",
    )?;
    validate_digest(
        &receipt.revocation_material_digest,
        "revocation material digest",
    )?;
    validate_common(
        &receipt.revocation.key_record_id,
        &receipt.revocation.key_record_digest,
        &receipt.revocation.key_id,
        &receipt.revocation.source_operator,
        &receipt.revocation.actor_kind,
        &receipt.revocation.revoked_by_admin_user_id,
        &receipt.revocation.idempotency_scope,
        &receipt.revocation.idempotency_key,
        &receipt.revocation.occurred_at,
        &receipt.revocation.recorded_at,
    )?;
    validate_text(&receipt.revocation.reason, "revocation reason", 8, 2_000)?;
    if receipt.revocation.confirmation != SIGNING_KEY_REVOCATION_CONFIRMATION
        || receipt.revocation.currentness_effect != SIGNING_KEY_STATUS_REVOKED
        || !has_no_business_effect(
            &receipt.revocation.artifact_signature_effect,
            &receipt.revocation.adapter_effect,
            &receipt.revocation.route_effect,
        )
    {
        bail!("signing-key revocation effects are not exact");
    }
    if signing_key_revocation_material_digest(&receipt.revocation)?
        != receipt.revocation_material_digest
        || canonical_signing_key_revocation_json_and_digest(receipt)?.1
            != receipt.revocation_receipt_digest
    {
        bail!("signing-key revocation digest is not canonical");
    }
    Ok(())
}

fn validate_registration(
    registration: &ExternalPoolAdapterArtifactSigningKeyRegistration,
) -> Result<()> {
    validate_text(&registration.source_operator, "source operator", 1, 160)?;
    validate_digest(&registration.key_id, "key ID")?;
    validate_identifier(
        &registration.created_by_admin_user_id,
        "registration actor ID",
        160,
    )?;
    validate_identifier(
        &registration.idempotency_scope,
        "registration idempotency scope",
        200,
    )?;
    validate_identifier(
        &registration.idempotency_key,
        "registration idempotency key",
        160,
    )?;
    validate_timestamps(&registration.created_at, &registration.recorded_at)?;
    if registration.algorithm != SIGNING_KEY_ALGORITHM
        || registration.actor_kind != SIGNING_KEY_ACTOR_KIND
        || registration.confirmation != SIGNING_KEY_REGISTRATION_CONFIRMATION
        || registration.currentness_effect != SIGNING_KEY_STATUS_PENDING_ACTIVATION
        || registration.public_key_pem.is_empty()
        || registration.public_key_pem.len() > 16 * 1024
        || !has_no_business_effect(
            &registration.artifact_signature_effect,
            &registration.adapter_effect,
            &registration.route_effect,
        )
    {
        bail!("signing-key registration material is invalid");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_common(
    key_record_id: &str,
    key_record_digest: &str,
    key_id: &str,
    source_operator: &str,
    actor_kind: &str,
    actor_id: &str,
    idempotency_scope: &str,
    idempotency_key: &str,
    occurred_at: &str,
    recorded_at: &str,
) -> Result<()> {
    validate_identifier(key_record_id, "key record ID", 160)?;
    validate_digest(key_record_digest, "key record digest")?;
    validate_digest(key_id, "key ID")?;
    validate_text(source_operator, "source operator", 1, 160)?;
    validate_identifier(actor_id, "signing-key actor ID", 160)?;
    validate_identifier(idempotency_scope, "signing-key idempotency scope", 200)?;
    validate_identifier(idempotency_key, "signing-key idempotency key", 160)?;
    validate_timestamps(occurred_at, recorded_at)?;
    if actor_kind != SIGNING_KEY_ACTOR_KIND {
        bail!("signing-key actor kind is invalid");
    }
    Ok(())
}

fn validate_timestamps(first: &str, second: &str) -> Result<()> {
    if first != second {
        bail!("signing-key timestamps must be identical");
    }
    let parsed = DateTime::parse_from_rfc3339(first)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != first
    {
        bail!("signing-key timestamp must use canonical UTC nanoseconds");
    }
    Ok(())
}

fn has_no_business_effect(artifact: &str, adapter: &str, route: &str) -> bool {
    artifact == SIGNING_KEY_ARTIFACT_EFFECT_NONE
        && adapter == SIGNING_KEY_ADAPTER_EFFECT_NONE
        && route == SIGNING_KEY_ROUTE_EFFECT_NONE
}

fn validate_identifier(value: &str, label: &str, maximum: usize) -> Result<()> {
    validate_text(value, label, 1, maximum)
}

fn validate_text(value: &str, label: &str, minimum: usize, maximum: usize) -> Result<()> {
    let length = value.chars().count();
    if value.trim() != value
        || !(minimum..=maximum).contains(&length)
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}
