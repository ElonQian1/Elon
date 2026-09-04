use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    esk_asset::platform::{
        validate_prepared_input, PlatformAccount, PlatformAllocationInput,
        PlatformAllocationRecord, PlatformError, PlatformPolicy,
    },
    store::Store,
};

impl Store {
    /// Keep V1's summary shape while using the same authenticated, fully
    /// validated SQLite snapshot as the complete history reader.
    pub(crate) fn esk_platform_account(
        &self,
        user_id: &str,
        session_token: &str,
        limit: usize,
    ) -> Result<PlatformAccount> {
        let page = self.esk_platform_history(user_id, session_token, limit.clamp(1, 100), None)?;
        Ok(PlatformAccount {
            total_base_units: page.total_base_units,
            entry_count: page.entry_count,
            updated_at: page.updated_at,
            entries: page.entries,
        })
    }
}

pub(super) fn record_on(
    conn: &Connection,
    allocation_id: &str,
    policy: &PlatformPolicy,
) -> Result<Option<PlatformAllocationRecord>> {
    let stored = conn
        .query_row(
            "SELECT input_json, prepared_by, prepared_at, payment_key, user_id,
                    amount_base_units, request_digest, policy_digest
               FROM esk_platform_allocations WHERE allocation_id = ?1",
            params![allocation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()?;
    let Some((json, prepared_by, prepared_at, payment_key, user_id, amount, digest, policy_digest)) =
        stored
    else {
        return Ok(None);
    };
    let input: PlatformAllocationInput =
        serde_json::from_str(&json).map_err(|_| PlatformError::CorruptLedger)?;
    validate_prepared_input(policy, &input).map_err(|_| PlatformError::CorruptLedger)?;
    if input.payment_key != payment_key
        || input.user_id != user_id
        || input.amount_base_units != amount
        || input.request_digest != digest
        || input.policy_digest != policy_digest
    {
        return Err(PlatformError::CorruptLedger.into());
    }
    let recorded_at = conn
        .query_row(
            "SELECT p.created_at FROM esk_platform_approvals p
               JOIN esk_platform_ledger_entries l ON l.approval_id = p.approval_id
              WHERE p.allocation_id = ?1 AND l.allocation_id = ?1",
            params![allocation_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let canceled_at = conn
        .query_row(
            "SELECT created_at FROM esk_platform_cancellations WHERE allocation_id = ?1",
            params![allocation_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if recorded_at.is_some() && canceled_at.is_some() {
        return Err(PlatformError::CorruptLedger.into());
    }
    Ok(Some(PlatformAllocationRecord {
        allocation_id: allocation_id.to_string(),
        input,
        prepared_by,
        prepared_at,
        recorded_at,
        canceled_at,
        replayed: false,
    }))
}

pub(super) fn ensure_recording_integrity(conn: &Connection) -> Result<()> {
    let broken: bool = conn.query_row(
        "SELECT EXISTS (
           SELECT 1 FROM esk_platform_approvals p
           LEFT JOIN esk_platform_ledger_entries l ON l.approval_id = p.approval_id
           WHERE l.entry_id IS NULL
         ) OR EXISTS (
           SELECT 1 FROM esk_platform_ledger_entries l
           LEFT JOIN esk_platform_allocations a ON a.allocation_id = l.allocation_id
           LEFT JOIN esk_platform_approvals p ON p.approval_id = l.approval_id
           LEFT JOIN esk_platform_policy policy ON policy.policy_digest = a.policy_digest
           WHERE a.allocation_id IS NULL OR p.approval_id IS NULL OR policy.singleton IS NULL
              OR p.allocation_id <> l.allocation_id OR p.request_digest <> a.request_digest
              OR l.user_id <> a.user_id OR l.amount_base_units <> a.amount_base_units
              OR l.amount_base_units <= 0 OR l.created_at <> p.created_at
         ) OR EXISTS (
           SELECT 1 FROM esk_platform_cancellations c
           LEFT JOIN esk_platform_allocations a ON a.allocation_id = c.allocation_id
           WHERE a.allocation_id IS NULL OR c.request_digest <> a.request_digest
              OR EXISTS (SELECT 1 FROM esk_platform_approvals p WHERE p.allocation_id = c.allocation_id)
              OR EXISTS (SELECT 1 FROM esk_platform_ledger_entries l WHERE l.allocation_id = c.allocation_id)
         ) OR EXISTS (
           SELECT a.payment_key FROM esk_platform_allocations a
            WHERE NOT EXISTS (SELECT 1 FROM esk_platform_cancellations c WHERE c.allocation_id = a.allocation_id)
            GROUP BY a.payment_key HAVING COUNT(*) > 1
         )",
        [],
        |row| row.get(0),
    )?;
    if broken {
        return Err(PlatformError::CorruptLedger.into());
    }
    Ok(())
}

pub(super) fn checked_totals_on(conn: &Connection, user_id: &str) -> Result<(i64, i64)> {
    let mut statement =
        conn.prepare("SELECT user_id, amount_base_units FROM esk_platform_ledger_entries")?;
    let mut rows = statement.query([])?;
    let (mut global, mut user) = (0_i64, 0_i64);
    while let Some(row) = rows.next()? {
        let owner: String = row.get(0)?;
        let amount: i64 = row.get(1)?;
        if amount <= 0 {
            return Err(PlatformError::CorruptLedger.into());
        }
        global = global
            .checked_add(amount)
            .ok_or(PlatformError::CorruptLedger)?;
        if owner == user_id {
            user = user
                .checked_add(amount)
                .ok_or(PlatformError::CorruptLedger)?;
        }
    }
    Ok((global, user))
}
