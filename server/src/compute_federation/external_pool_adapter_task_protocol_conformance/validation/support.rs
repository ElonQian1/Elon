use anyhow::{bail, Result};
use chrono::{DateTime, FixedOffset, SecondsFormat};

use super::super::*;

pub(super) fn metadata(
    schema: &str,
    expected_schema: &str,
    id: &str,
    receipt_digest: &str,
    material_digest: &str,
    canonicalization: &str,
    digest_algorithm: &str,
) -> Result<()> {
    identifier(id)?;
    digest(receipt_digest)?;
    digest(material_digest)?;
    if schema != expected_schema
        || canonicalization != TASK_PROTOCOL_CONFORMANCE_CANONICALIZATION
        || digest_algorithm != TASK_PROTOCOL_CONFORMANCE_DIGEST_ALGORITHM
    {
        bail!("task-protocol conformance receipt metadata is invalid")
    }
    Ok(())
}

pub(super) fn identifier(value: &str) -> Result<()> {
    text(value, 1, 240)
}

pub(super) fn text(value: &str, minimum: usize, maximum: usize) -> Result<()> {
    let count = value.chars().count();
    if !(minimum..=maximum).contains(&count)
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        bail!("task-protocol conformance text value is invalid")
    }
    Ok(())
}

pub(super) fn reason(value: &str) -> Result<()> {
    text(value, 12, 500)
}

pub(super) fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!("task-protocol conformance digest is invalid")
    }
    Ok(())
}

pub(super) fn canonical_nanos(value: &str) -> Result<DateTime<FixedOffset>> {
    if value.len() != 30 {
        bail!("task-protocol conformance timestamp is not canonical UTC nanos")
    }
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("task-protocol conformance timestamp is not canonical UTC nanos")
    }
    Ok(parsed)
}
