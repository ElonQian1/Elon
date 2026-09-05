use anyhow::Result;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

use crate::{
    esk_asset::platform::{
        valid_history_entry_id, validate_policy_integrity, PlatformEntry, PlatformError,
        PlatformHistoryCursor, PlatformHistoryPage,
    },
    store::Store,
};

use super::{
    ensure_session, policy_on,
    read::{ensure_recording_integrity, record_on},
};

impl Store {
    /// Full validation and fingerprinting share one authenticated SQLite snapshot.
    /// Only the requested page is retained; the cursor never selects a user.
    pub(crate) fn esk_platform_history(
        &self,
        user_id: &str,
        session_token: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<PlatformHistoryPage> {
        if !(1..=100).contains(&limit) {
            return Err(PlatformError::InvalidInput.into());
        }
        let cursor = cursor.map(PlatformHistoryCursor::parse).transpose()?;
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let page =
            scan_authenticated_history_on(&tx, user_id, session_token, limit, cursor.as_ref())?;
        tx.commit()?;
        Ok(page)
    }
}

/// The caller owns the transaction; scanning never opens another connection.
/// Every use still validates the real session in that same SQLite snapshot.
pub(super) fn scan_authenticated_history_on(
    conn: &Connection,
    user_id: &str,
    session_token: &str,
    limit: usize,
    cursor: Option<&PlatformHistoryCursor>,
) -> Result<PlatformHistoryPage> {
    ensure_session(conn, user_id, session_token, false)?;
    scan_history_on(conn, user_id, limit, cursor)
}

/// Private scan core. Callers must validate their session or delegated read
/// authority on this connection inside the same transaction before entering.
pub(super) fn scan_history_on(
    conn: &Connection,
    user_id: &str,
    limit: usize,
    cursor: Option<&PlatformHistoryCursor>,
) -> Result<PlatformHistoryPage> {
    ensure_recording_integrity(conn)?;
    let policy = policy_on(conn)?;
    if let Some(policy) = policy.as_ref() {
        validate_policy_integrity(policy).map_err(|_| PlatformError::CorruptLedger)?;
    }
    let mut fingerprint = SnapshotFingerprint::new(
        user_id,
        policy.as_ref().map(|value| value.policy_digest.as_str()),
    );
    let mut page = PlatformHistoryPage {
        snapshot_digest: String::new(),
        total_base_units: 0,
        entry_count: 0,
        range_start: 0,
        range_end: 0,
        updated_at: None,
        entries: Vec::with_capacity(limit),
        has_more: false,
        next_cursor: None,
    };
    let mut statement = conn.prepare(
        "SELECT entry_id, allocation_id, amount_base_units, created_at
               FROM esk_platform_ledger_entries WHERE user_id = ?1
              ORDER BY created_at DESC, entry_id DESC",
    )?;
    let mut rows = statement.query(params![user_id])?;
    let mut anchor_position = None;
    while let Some(row) = rows.next()? {
        let entry = PlatformEntry {
            entry_id: row.get(0)?,
            allocation_id: row.get(1)?,
            amount_base_units: row.get(2)?,
            created_at: row.get(3)?,
        };
        let policy = policy.as_ref().ok_or(PlatformError::CorruptLedger)?;
        let allocation =
            record_on(conn, &entry.allocation_id, policy)?.ok_or(PlatformError::CorruptLedger)?;
        if !valid_history_entry_id(&entry.entry_id)
            || entry.amount_base_units <= 0
            || allocation.input.user_id != user_id
            || allocation.input.amount_base_units != entry.amount_base_units
            || allocation.recorded_at.as_deref() != Some(entry.created_at.as_str())
        {
            return Err(PlatformError::CorruptLedger.into());
        }
        page.total_base_units = page
            .total_base_units
            .checked_add(entry.amount_base_units)
            .ok_or(PlatformError::CorruptLedger)?;
        page.entry_count = page
            .entry_count
            .checked_add(1)
            .ok_or(PlatformError::CorruptLedger)?;
        if page.total_base_units > policy.issuance_limit_base_units {
            return Err(PlatformError::CorruptLedger.into());
        }
        if page.updated_at.is_none() {
            page.updated_at = Some(entry.created_at.clone());
        }
        fingerprint.entry(&entry);
        // Keep scanning after the page fills: an off-page change or corrupt
        // record must invalidate the complete snapshot, not become invisible.
        let after_anchor = cursor.is_none() || anchor_position.is_some();
        if after_anchor && page.entries.len() < limit {
            if page.entries.is_empty() {
                page.range_start = page.entry_count;
            }
            page.range_end = page.entry_count;
            page.entries.push(entry.clone());
        }
        if cursor.is_some_and(|value| value.after_entry_id == entry.entry_id) {
            anchor_position = Some(page.entry_count);
        }
    }
    drop(rows);
    drop(statement);
    page.snapshot_digest = fingerprint.finish();
    if let Some(cursor) = cursor {
        if cursor.snapshot_digest != page.snapshot_digest
            || anchor_position.is_none()
            || anchor_position == Some(page.entry_count)
        {
            return Err(PlatformError::HistoryChanged.into());
        }
    }
    page.has_more = page.range_end < page.entry_count;
    if page.has_more {
        let last = page.entries.last().ok_or(PlatformError::CorruptLedger)?;
        page.next_cursor = Some(format!("ephp1.{}.{}", page.snapshot_digest, last.entry_id));
    }
    Ok(page)
}

/// v1 encoding: domain, user, policy marker [+ digest], then four text fields
/// per ordered entry (ID, allocation ID, canonical base units, timestamp).
/// Every text is prefixed with its UTF-8 byte length as an unsigned u64 BE.
struct SnapshotFingerprint(Sha256);

impl SnapshotFingerprint {
    fn new(user_id: &str, policy_digest: Option<&str>) -> Self {
        let mut result = Self(Sha256::new());
        result.text("yilong.esk.platform_history.snapshot.v1");
        result.text(user_id);
        match policy_digest {
            Some(digest) => {
                result.text("policy-present");
                result.text(digest);
            }
            None => result.text("policy-absent"),
        }
        result
    }

    fn entry(&mut self, entry: &PlatformEntry) {
        self.text(&entry.entry_id);
        self.text(&entry.allocation_id);
        self.text(&entry.amount_base_units.to_string());
        self.text(&entry.created_at);
    }

    fn text(&mut self, value: &str) {
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value.as_bytes());
    }

    fn finish(self) -> String {
        format!("{:x}", self.0.finalize())
    }
}
