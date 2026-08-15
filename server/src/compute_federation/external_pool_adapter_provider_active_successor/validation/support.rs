use anyhow::{bail, Result};
use chrono::{DateTime, FixedOffset, SecondsFormat};

pub(super) fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("provider active successor identifier is invalid")
    }
    Ok(())
}

pub(super) fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!("provider active successor digest is invalid")
    }
    Ok(())
}

pub(super) fn canonical_nanos(value: &str) -> Result<DateTime<FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("provider active successor timestamp is not canonical UTC nanos")
    }
    Ok(parsed)
}

pub(super) fn reason_code(value: &str) -> Result<()> {
    if !(3..=120).contains(&value.len())
        || value.trim() != value
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        bail!("provider active successor reason code is invalid")
    }
    Ok(())
}
