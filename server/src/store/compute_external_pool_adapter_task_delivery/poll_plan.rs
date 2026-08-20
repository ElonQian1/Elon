//! Exact OLD/NEW projections consumed by the V278 poll-CAS triggers.

use anyhow::Result;
use rusqlite::types::Value;

use super::types::PollClaimProjection;

pub(super) fn poll_cas_values(
    poll_id: &str,
    poll_digest: &str,
    before: &PollClaimProjection,
    after: &PollClaimProjection,
) -> Result<Vec<Value>> {
    Ok(vec![
        Value::Text(poll_id.to_string()),
        Value::Text(poll_digest.to_string()),
        Value::Text(before.status.clone()),
        Value::Text(after.status.clone()),
        Value::Integer(i64::try_from(before.revision)?),
        Value::Integer(i64::try_from(after.revision)?),
        Value::Integer(i64::try_from(before.generation)?),
        Value::Integer(i64::try_from(after.generation)?),
        optional_text(before.owner_id.as_deref()),
        optional_text(after.owner_id.as_deref()),
        optional_text(before.token_digest.as_deref()),
        optional_text(after.token_digest.as_deref()),
        optional_text(before.expires_at.as_deref()),
        optional_text(after.expires_at.as_deref()),
    ])
}

fn optional_text(value: Option<&str>) -> Value {
    value
        .map(|value| Value::Text(value.to_string()))
        .unwrap_or(Value::Null)
}
