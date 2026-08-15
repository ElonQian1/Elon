use anyhow::{bail, Result};
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};

use super::{
    ExternalPoolAdapterProviderActiveSuccessorProcessSealInput,
    PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
    PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_PROCESS_KIND,
};
use crate::store::compute_external_pool_adapter_runtime_bundle::runtime::custody::support::is_lower_hex_sha256;

const MAX_PROCESS_SEAL_TTL_MS: i64 = 15_000;

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_registry_tuple(
    kind: &str,
    entity_id: &str,
    entity_digest: &str,
    epoch: &str,
    nonce: &str,
    seal: &str,
    integrity: &str,
    expires_at: &str,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>> {
    if !valid_tuple(
        kind,
        entity_id,
        entity_digest,
        epoch,
        nonce,
        seal,
        integrity,
    ) {
        bail!("active-successor process seal tuple is invalid");
    }
    let expires = canonical_time(expires_at)?;
    if expires <= now || expires > now + ChronoDuration::milliseconds(MAX_PROCESS_SEAL_TTL_MS) {
        bail!("active-successor process seal expiry is outside its fixed live window");
    }
    Ok(expires)
}

pub(super) fn validate_input(
    input: &ExternalPoolAdapterProviderActiveSuccessorProcessSealInput<'_>,
    now: DateTime<Utc>,
) -> Result<()> {
    if !valid_kind(input.kind)
        || !valid_id(input.entity_id)
        || !valid_id(input.provider_binding_id)
        || !is_lower_hex_sha256(input.entity_digest)
        || !is_lower_hex_sha256(input.activation_root_digest)
    {
        bail!("active-successor process seal input is invalid");
    }
    let checked = canonical_time(input.checked_at)?;
    let expires = canonical_time(input.expires_at)?;
    if checked > now
        || expires <= now
        || expires <= checked
        || expires > checked + ChronoDuration::milliseconds(MAX_PROCESS_SEAL_TTL_MS)
    {
        bail!("active-successor process seal timestamps are outside the fixed live window");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn valid_tuple(
    kind: &str,
    entity_id: &str,
    entity_digest: &str,
    epoch: &str,
    nonce: &str,
    seal: &str,
    integrity: &str,
) -> bool {
    valid_kind(kind)
        && valid_id(entity_id)
        && [entity_digest, epoch, nonce, seal, integrity]
            .into_iter()
            .all(is_lower_hex_sha256)
}

pub(super) fn valid_kind(kind: &str) -> bool {
    matches!(
        kind,
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND
            | PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_PROCESS_KIND
    )
}

pub(super) fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn canonical_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("active-successor process seal time is not canonical UTC nanoseconds");
    }
    Ok(parsed.with_timezone(&Utc))
}
