use anyhow::{bail, Result};
use chrono::{DateTime, FixedOffset, SecondsFormat};

use super::super::*;

pub(super) fn metadata(
    schema: &str,
    expected_schema: &str,
    id: &str,
    digest_value: &str,
    canonicalization: &str,
    digest_algorithm: &str,
) -> Result<()> {
    identifier(id)?;
    digest(digest_value)?;
    if schema != expected_schema
        || canonicalization != TASK_PRODUCTION_CANONICALIZATION
        || digest_algorithm != TASK_PRODUCTION_DIGEST_ALGORITHM
    {
        bail!("task production durable metadata is invalid")
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
        bail!("task production text value is invalid")
    }
    Ok(())
}

pub(super) fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        || value.bytes().all(|byte| byte == b'0')
    {
        bail!("task production digest is invalid")
    }
    Ok(())
}

pub(super) fn optional_pair(id: Option<&str>, digest_value: Option<&str>) -> Result<()> {
    match (id, digest_value) {
        (None, None) => Ok(()),
        (Some(id), Some(digest_value)) => {
            identifier(id)?;
            digest(digest_value)
        }
        _ => bail!("task production optional lineage pair is partial"),
    }
}

pub(super) fn canonical_nanos(value: &str) -> Result<DateTime<FixedOffset>> {
    if value.len() != 30 {
        bail!("task production timestamp is not canonical UTC nanos")
    }
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("task production timestamp is not canonical UTC nanos")
    }
    Ok(parsed)
}

pub(super) fn boundary(value: &ExternalPoolAdapterTaskProductionBoundary) -> Result<()> {
    if value.authority_status != TASK_PRODUCTION_NO_V213_AUTHORITY
        || value.effects != ExternalPoolAdapterTaskProductionEffects::none()
        || value.readiness != ExternalPoolAdapterTaskProductionReadiness::none()
    {
        bail!("task production boundary would expand authority")
    }
    Ok(())
}

pub(super) fn poll_lineage(value: &ExternalPoolAdapterTaskPollLineage) -> Result<()> {
    optional_pair(
        value.predecessor_id.as_deref(),
        value.predecessor_digest.as_deref(),
    )?;
    if value.poll_ordinal == 0 || value.poll_ordinal > TASK_PRODUCTION_MAX_SAFE_INTEGER {
        bail!("task production poll ordinal is invalid")
    }
    if (value.poll_ordinal == 1) != value.predecessor_id.is_none() {
        bail!("task production poll lineage is not contiguous")
    }
    Ok(())
}

pub(super) fn remote(value: &ExternalPoolAdapterTaskRemoteIdentity) -> Result<()> {
    digest(&value.executor_binding_digest)?;
    if let Some(remote_execution_id) = value.remote_execution_id.as_deref() {
        identifier(remote_execution_id)?;
    }
    digest(&value.remote_identity_digest)?;
    if task_production_remote_identity_digest(
        &value.executor_binding_digest,
        value.remote_execution_id.as_deref(),
    )? != value.remote_identity_digest
    {
        bail!("task production remote identity digest is not exact")
    }
    if !matches!(
        value.remote_execution_state.as_str(),
        "absent"
            | "prepared"
            | "committed"
            | "running"
            | "terminal_no_start"
            | "terminal_after_run"
            | "unknown"
            | "rejected"
    ) || (value.remote_execution_id.is_none()
        && !matches!(
            value.remote_execution_state.as_str(),
            "absent" | "unknown" | "rejected"
        ))
        || (value.remote_execution_id.is_some() && value.remote_execution_state == "absent")
    {
        bail!("task production remote execution state is invalid")
    }
    Ok(())
}
