use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;

use crate::{
    esk_asset::platform::{
        validate_policy_integrity, PlatformError, PlatformReconciliationSnapshot,
        PLATFORM_PAYMENT_SNAPSHOT_MAX_KEYS,
    },
    store::Store,
};

use super::{
    ensure_admin, policy_on,
    read::{checked_totals_on, ensure_recording_integrity, record_on},
};

impl Store {
    /// A complete, bounded read of formal payment claims; never an external-history proof.
    /// Uses only the pinned database policy, including while new writes are disabled.
    pub(crate) fn esk_platform_reconciliation_snapshot(
        &self,
        actor_user_id: &str,
        session_token: &str,
    ) -> Result<PlatformReconciliationSnapshot> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        // This query establishes the SQLite snapshot before sampling its observation time.
        ensure_admin(&tx, actor_user_id, session_token)?;
        let observed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        ensure_recording_integrity(&tx)?;
        let policy = match policy_on(&tx)? {
            Some(policy) => policy,
            None if any_formal_records(&tx)? => return Err(PlatformError::CorruptLedger.into()),
            None => return Err(PlatformError::InvalidPolicy.into()),
        };
        validate_policy_integrity(&policy).map_err(|_| PlatformError::CorruptLedger)?;
        let (global_total, _) = checked_totals_on(&tx, actor_user_id)?;
        if global_total > policy.issuance_limit_base_units {
            return Err(PlatformError::CorruptLedger.into());
        }
        let mut statement = tx.prepare(
            "SELECT a.allocation_id FROM esk_platform_allocations a
               WHERE NOT EXISTS (SELECT 1 FROM esk_platform_cancellations c
                                  WHERE c.allocation_id = a.allocation_id)
               ORDER BY a.payment_key COLLATE BINARY ASC",
        )?;
        let mut rows = statement.query([])?;
        let mut used_payment_keys: Vec<String> = Vec::new();
        let (mut prepared_count, mut recorded_count) = (0, 0);
        while let Some(row) = rows.next()? {
            // Do not buffer or silently truncate the 10001st current payment claim.
            if used_payment_keys.len() == PLATFORM_PAYMENT_SNAPSHOT_MAX_KEYS {
                return Err(PlatformError::LimitExceeded.into());
            }
            let allocation_id: String = row.get(0)?;
            let record =
                record_on(&tx, &allocation_id, &policy)?.ok_or(PlatformError::CorruptLedger)?;
            if record.canceled_at.is_some()
                || used_payment_keys
                    .last()
                    .is_some_and(|previous| previous >= &record.input.payment_key)
            {
                return Err(PlatformError::CorruptLedger.into());
            }
            if record.recorded_at.is_some() {
                recorded_count += 1;
            } else {
                prepared_count += 1;
            }
            used_payment_keys.push(record.input.payment_key);
        }
        drop(rows);
        drop(statement);
        let snapshot = PlatformReconciliationSnapshot::new(
            policy.source_fingerprint,
            policy.policy_digest,
            observed_at,
            used_payment_keys,
            prepared_count,
            recorded_count,
        )?;
        // Recheck time-sensitive session validity without leaving this read transaction.
        ensure_admin(&tx, actor_user_id, session_token)?;
        tx.commit()?;
        Ok(snapshot)
    }
}

fn any_formal_records(conn: &Connection) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM esk_platform_allocations)
          OR EXISTS(SELECT 1 FROM esk_platform_approvals)
          OR EXISTS(SELECT 1 FROM esk_platform_ledger_entries)
          OR EXISTS(SELECT 1 FROM esk_platform_cancellations)",
        [],
        |row| row.get(0),
    )?)
}
