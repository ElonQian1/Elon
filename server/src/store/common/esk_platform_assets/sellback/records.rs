use anyhow::Result;
use rusqlite::{params, Connection, Row};

use crate::esk_asset::platform::{sellback::*, PlatformPolicy};

use super::add;

/// The global cap uses exactly the same per-record validation as private reads.
/// Retain no global record collection, only checked accumulators.
pub(super) fn global_reserved_on(conn: &Connection, policy: &PlatformPolicy) -> Result<i64> {
    let mut reserved = 0;
    visit_on(conn, None, Some(policy), |record| {
        if record.canceled_at.is_none() {
            reserved = add(reserved, record.input.amount_base_units)?;
        }
        Ok(())
    })?;
    Ok(reserved)
}

pub(super) fn visit_on(
    conn: &Connection,
    user: Option<&str>,
    policy: Option<&PlatformPolicy>,
    mut visit: impl FnMut(SellbackRecord) -> Result<()>,
) -> Result<()> {
    // Fail closed even if a damaged DB was written with foreign_keys disabled.
    let broken: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM esk_platform_sellback_cancellations c
          LEFT JOIN esk_platform_sellback_requests r ON r.request_id = c.request_id
          WHERE r.request_id IS NULL)
         OR EXISTS(SELECT 1 FROM esk_platform_sellback_requests r
          LEFT JOIN users u ON u.id = r.user_id WHERE u.id IS NULL)",
        [],
        |row| row.get(0),
    )?;
    if broken {
        return Err(SellbackError::Corrupt.into());
    }
    let mut statement = conn.prepare(
        "SELECT r.request_id, r.user_id, r.idempotency_key, r.amount_base_units,
                r.request_digest, r.input_json, r.policy_json, r.platform_policy_digest,
                r.source_fingerprint, r.created_at, c.cancel_event_id, c.canceled_by,
                c.request_digest, c.created_at
           FROM esk_platform_sellback_requests r
           LEFT JOIN esk_platform_sellback_cancellations c ON c.request_id = r.request_id
          WHERE (?1 IS NULL OR r.user_id = ?1)
          ORDER BY r.created_at DESC, r.request_id DESC",
    )?;
    let mut rows = statement.query(params![user])?;
    while let Some(row) = rows.next()? {
        let record = read_record(row, policy)?;
        visit(record)?;
    }
    Ok(())
}

fn read_record(row: &Row<'_>, policy: Option<&PlatformPolicy>) -> Result<SellbackRecord> {
    let policy = policy.ok_or(SellbackError::Corrupt)?;
    let input_json: String = row.get(5)?;
    let policy_json: String = row.get(6)?;
    if input_json.len() > 4096 || policy_json.len() > 131072 {
        return Err(SellbackError::Corrupt.into());
    }
    let input: SellbackSubmitInput =
        serde_json::from_str(&input_json).map_err(|_| SellbackError::Corrupt)?;
    let saved_policy: SellbackPolicy =
        serde_json::from_str(&policy_json).map_err(|_| SellbackError::Corrupt)?;
    let record = SellbackRecord {
        request_id: row.get(0)?,
        user_id: row.get(1)?,
        input,
        request_digest: row.get(4)?,
        policy: saved_policy,
        created_at: row.get(9)?,
        cancel_event_id: row.get(10)?,
        canceled_at: row.get(13)?,
    };
    if row.get::<_, String>(2)? != record.input.idempotency_key
        || row.get::<_, i64>(3)? != record.input.amount_base_units
        || row.get::<_, String>(7)? != policy.policy_digest
        || row.get::<_, String>(8)? != policy.source_fingerprint
        || record.policy.body.source_fingerprint != policy.source_fingerprint
    {
        return Err(SellbackError::Corrupt.into());
    }
    let canceled_by: Option<String> = row.get(11)?;
    let cancel_digest: Option<String> = row.get(12)?;
    match (
        &record.cancel_event_id,
        canceled_by,
        cancel_digest,
        &record.canceled_at,
    ) {
        (None, None, None, None) => {}
        (Some(_), Some(user), Some(digest), Some(_))
            if user == record.user_id && digest == record.request_digest => {}
        _ => return Err(SellbackError::Corrupt.into()),
    }
    validate_stored_request(&record)?;
    Ok(record)
}
