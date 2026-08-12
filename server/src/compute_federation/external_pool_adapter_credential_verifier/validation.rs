use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use super::{canonical::*, types::*};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub(crate) fn validate_credential_verifier_record(
    value: &ExternalPoolAdapterCredentialVerifierRecord,
) -> Result<()> {
    if value.schema != CREDENTIAL_VERIFIER_RECORD_SCHEMA
        || value.canonicalization != CREDENTIAL_VERIFIER_CANONICALIZATION
        || value.digest_algorithm != CREDENTIAL_VERIFIER_DIGEST_ALGORITHM
    {
        bail!("credential-verifier record metadata is unsupported");
    }
    identifier(&value.verifier_record_id, 160)?;
    digest(&value.verifier_record_digest)?;
    digest(&value.registration_material_digest)?;
    registration(&value.registration)?;
    if credential_verifier_registration_digest(&value.registration)?
        != value.registration_material_digest
        || credential_verifier_record_json_and_digest(value)?.1 != value.verifier_record_digest
    {
        bail!("credential-verifier record is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_credential_verifier_transition(
    value: &ExternalPoolAdapterCredentialVerifierTransitionReceipt,
) -> Result<()> {
    if value.canonicalization != CREDENTIAL_VERIFIER_CANONICALIZATION
        || value.digest_algorithm != CREDENTIAL_VERIFIER_DIGEST_ALGORITHM
        || !matches!(
            value.schema.as_str(),
            CREDENTIAL_VERIFIER_ACTIVATION_SCHEMA | CREDENTIAL_VERIFIER_REVOCATION_SCHEMA
        )
    {
        bail!("credential-verifier transition metadata is unsupported");
    }
    identifier(&value.transition_receipt_id, 160)?;
    digest(&value.transition_receipt_digest)?;
    digest(&value.transition_material_digest)?;
    transition(&value.transition, &value.schema)?;
    if credential_verifier_transition_digest(&value.transition)? != value.transition_material_digest
        || credential_verifier_transition_json_and_digest(value)?.1
            != value.transition_receipt_digest
    {
        bail!("credential-verifier transition is not canonical");
    }
    Ok(())
}

fn registration(value: &ExternalPoolAdapterCredentialVerifierRegistration) -> Result<()> {
    text(&value.verifier_operator, 1, 160)?;
    text(&value.verifier_product, 1, 160)?;
    identifier(&value.verification_kind, 80)?;
    identifier(&value.verifier_id, 160)?;
    revision(value.verifier_revision)?;
    digest(&value.verifier_digest)?;
    identifier(&value.created_by_admin_user_id, 160)?;
    identifier(&value.idempotency_scope, 200)?;
    identifier(&value.idempotency_key, 160)?;
    timestamp_pair(&value.created_at, &value.recorded_at)?;
    if value.actor_kind != CREDENTIAL_VERIFIER_ACTOR_KIND
        || value.confirmation != CREDENTIAL_VERIFIER_REGISTER_CONFIRMATION
        || value.currentness_effect != CREDENTIAL_VERIFIER_STATUS_PENDING
        || !no_effects(
            &value.credential_receipt_effect,
            &value.adapter_adoption_effect,
            &value.route_effect,
            &value.execution_effect,
        )
    {
        bail!("credential-verifier registration is invalid");
    }
    Ok(())
}

fn transition(value: &ExternalPoolAdapterCredentialVerifierTransition, schema: &str) -> Result<()> {
    identifier(&value.verifier_record_id, 160)?;
    digest(&value.verifier_record_digest)?;
    identifier(&value.verification_kind, 80)?;
    identifier(&value.verifier_id, 160)?;
    revision(value.verifier_revision)?;
    digest(&value.verifier_digest)?;
    text(&value.verifier_operator, 1, 160)?;
    text(&value.verifier_product, 1, 160)?;
    identifier(&value.actor_user_id, 160)?;
    identifier(&value.idempotency_scope, 200)?;
    identifier(&value.idempotency_key, 160)?;
    timestamp_pair(&value.occurred_at, &value.recorded_at)?;
    let activation = schema == CREDENTIAL_VERIFIER_ACTIVATION_SCHEMA;
    if value.actor_kind != CREDENTIAL_VERIFIER_ACTOR_KIND
        || value.confirmation
            != if activation {
                CREDENTIAL_VERIFIER_ACTIVATE_CONFIRMATION
            } else {
                CREDENTIAL_VERIFIER_REVOKE_CONFIRMATION
            }
        || value.currentness_effect
            != if activation {
                CREDENTIAL_VERIFIER_STATUS_ACTIVE
            } else {
                CREDENTIAL_VERIFIER_STATUS_REVOKED
            }
        || activation != value.reason.is_none()
        || value
            .reason
            .as_deref()
            .is_some_and(|reason| text(reason, 8, 2_000).is_err())
        || !no_effects(
            &value.credential_receipt_effect,
            &value.adapter_adoption_effect,
            &value.route_effect,
            &value.execution_effect,
        )
    {
        bail!("credential-verifier transition is invalid");
    }
    Ok(())
}

fn no_effects(first: &str, second: &str, third: &str, fourth: &str) -> bool {
    [first, second, third, fourth]
        .into_iter()
        .all(|effect| effect == CREDENTIAL_VERIFIER_NO_EFFECT)
}

fn timestamp_pair(first: &str, second: &str) -> Result<()> {
    if first != second {
        bail!("credential-verifier timestamps differ");
    }
    let parsed = DateTime::parse_from_rfc3339(first)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != first
    {
        bail!("credential-verifier timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}

fn revision(value: i64) -> Result<()> {
    if !(1..=MAX_SAFE_INTEGER).contains(&value) {
        bail!("credential-verifier revision is invalid");
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
        bail!("credential-verifier text is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("credential-verifier digest is invalid");
    }
    Ok(())
}
