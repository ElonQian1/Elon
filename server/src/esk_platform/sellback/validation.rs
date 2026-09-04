use anyhow::Result;
use serde_json::json;

use super::{model::*, policy::validate_policy_integrity};
use crate::esk_asset::platform::payment_identity::fingerprint;

pub(crate) fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn label(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

pub(crate) fn positive_units(value: &str) -> Result<i64> {
    if value.is_empty()
        || value.len() > 19
        || value.starts_with('0')
        || !value.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(SellbackError::InvalidInput.into());
    }
    value
        .parse::<i64>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(|| SellbackError::InvalidInput.into())
}

pub(crate) fn validate_submit_body(body: SellbackSubmitBody) -> Result<SellbackSubmitInput> {
    if body.schema != SUBMIT_SCHEMA || body.confirmation != SUBMIT_CONFIRMATION {
        return Err(SellbackError::InvalidInput.into());
    }
    let input = SellbackSubmitInput {
        idempotency_key: body.idempotency_key,
        amount_base_units: positive_units(&body.amount_base_units)?,
        expected_snapshot_digest: body.expected_snapshot_digest,
        policy_digest: body.policy_digest,
        terms_digest: body.terms_digest,
    };
    validate_input(&input)?;
    Ok(input)
}

pub(crate) fn validate_input(input: &SellbackSubmitInput) -> Result<()> {
    if !label(&input.idempotency_key, 96)
        || input.amount_base_units <= 0
        || !digest(&input.expected_snapshot_digest)
        || !digest(&input.policy_digest)
        || !digest(&input.terms_digest)
    {
        return Err(SellbackError::InvalidInput.into());
    }
    Ok(())
}

pub(crate) fn request_digest(
    user_id: &str,
    policy: &SellbackPolicy,
    input: &SellbackSubmitInput,
) -> Result<String> {
    validate_input(input)?;
    validate_policy_integrity(policy)?;
    if !label(user_id, 96)
        || user_id == "local-owner"
        || !policy.body.eligible_user_ids.iter().any(|id| id == user_id)
        || input.policy_digest != policy.policy_digest
        || input.terms_digest != policy.body.terms_digest
        || input.amount_base_units < positive_units(&policy.body.min_request_base_units)?
        || input.amount_base_units > positive_units(&policy.body.max_request_base_units)?
    {
        return Err(SellbackError::InvalidInput.into());
    }
    fingerprint(&json!({"schema":"yilong.esk.platform_sellback_request.v1", "user_id":user_id, "input":input}))
        .map_err(|_| SellbackError::InvalidInput.into())
}

pub(crate) fn valid_request_id(value: &str) -> bool {
    fixed_id(value, "eskpsr_")
}
pub(crate) fn valid_cancel_id(value: &str) -> bool {
    fixed_id(value, "eskpsc_")
}
fn fixed_id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

pub(crate) fn parse_cursor(value: &str) -> Result<SellbackCursor> {
    let mut parts = value.split('.');
    let prefix = parts.next();
    let snapshot = parts.next().unwrap_or_default();
    let id = parts.next().unwrap_or_default();
    if prefix != Some("esbr1")
        || !digest(snapshot)
        || !valid_request_id(id)
        || parts.next().is_some()
    {
        return Err(SellbackError::InvalidInput.into());
    }
    Ok(SellbackCursor {
        snapshot_digest: snapshot.into(),
        after_request_id: id.into(),
    })
}

pub(crate) fn validate_stored_request(record: &SellbackRecord) -> Result<()> {
    let check = || -> Result<()> {
        if !valid_request_id(&record.request_id)
            || !valid_timestamp(&record.created_at)
            || request_digest(&record.user_id, &record.policy, &record.input)?
                != record.request_digest
        {
            return Err(SellbackError::Corrupt.into());
        }
        match (&record.canceled_at, &record.cancel_event_id) {
            (None, None) => Ok(()),
            (Some(at), Some(id))
                if timestamp_not_before(at, &record.created_at) && valid_cancel_id(id) =>
            {
                Ok(())
            }
            _ => Err(SellbackError::Corrupt.into()),
        }
    };
    check().map_err(|_| SellbackError::Corrupt.into())
}

/// Same UTC wire grammar as the Android parser, without a new harness dependency.
pub(crate) fn valid_timestamp(value: &str) -> bool {
    let Some(raw) = value
        .strip_suffix('Z')
        .or_else(|| value.strip_suffix("+00:00"))
    else {
        return false;
    };
    if !raw.is_ascii() || raw.len() < 19 || raw.len() > 29 {
        return false;
    }
    let b = raw.as_bytes();
    if [4, 7].iter().any(|i| b[*i] != b'-') || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return false;
    }
    let number = |from: usize, end: usize| raw[from..end].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0, 4),
        number(5, 7),
        number(8, 10),
        number(11, 13),
        number(14, 16),
        number(17, 19),
    ) else {
        return false;
    };
    if !b
        .iter()
        .take(19)
        .enumerate()
        .all(|(i, b)| [4, 7, 10, 13, 16].contains(&i) || b.is_ascii_digit())
    {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    day > 0
        && day <= days
        && hour < 24
        && minute < 60
        && second < 60
        && (raw.len() == 19
            || (raw.len() >= 21 && b[19] == b'.' && b[20..].iter().all(u8::is_ascii_digit)))
}

/// Compare UTC calendar components and zero-padded nanoseconds, not wire text.
pub(crate) fn timestamp_not_before(value: &str, earlier: &str) -> bool {
    fn components(value: &str) -> Option<[u32; 7]> {
        if !valid_timestamp(value) {
            return None;
        }
        let raw = value
            .strip_suffix('Z')
            .or_else(|| value.strip_suffix("+00:00"))?;
        let mut result = [0; 7];
        for (index, (start, end)) in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)]
            .iter()
            .enumerate()
        {
            result[index] = raw[*start..*end].parse().ok()?;
        }
        if raw.len() > 19 {
            let fraction = &raw[20..];
            result[6] = fraction.parse::<u32>().ok()? * 10u32.pow(9 - fraction.len() as u32);
        }
        Some(result)
    }
    match (components(value), components(earlier)) {
        (Some(value), Some(earlier)) => value >= earlier,
        _ => false,
    }
}
