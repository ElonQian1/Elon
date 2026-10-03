//! Bounded per-owner slots. Replacement never combines accounts or renews observation time.
use super::model::{Snapshot, MAX_BYTES, MAX_SOURCES, MAX_TOTAL, RETENTION_MS};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
pub(super) struct Source {
    pub source_id: String,
    pub snapshot: Snapshot,
}
fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS grid_device_sources_v1 (
        owner TEXT NOT NULL, platform TEXT NOT NULL, device TEXT NOT NULL,
        sequence INTEGER NOT NULL, digest TEXT NOT NULL, body TEXT NOT NULL,
        received INTEGER NOT NULL, PRIMARY KEY(owner,platform,device))",
    )?;
    Ok(())
}
pub(super) fn put(
    conn: &mut Connection,
    owner: &str,
    value: &Snapshot,
    now: u64,
) -> Result<&'static str> {
    value.validate(now).map_err(anyhow::Error::msg)?;
    migrate(conn)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let body = serde_json::to_string(value)?;
    if body.len() > MAX_BYTES {
        bail!("source_too_large");
    }
    let digest = hex::encode(Sha256::digest(body.as_bytes()));
    let prior: Option<(u64, String)> = tx.query_row(
        "SELECT sequence,digest FROM grid_device_sources_v1 WHERE owner=?1 AND platform=?2 AND device=?3",
        params![owner, value.platform, value.device_id], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
    if let Some((sequence, previous)) = prior {
        if value.sequence < sequence {
            bail!("source_out_of_order");
        }
        if value.sequence == sequence {
            if digest != previous {
                bail!("source_revision_conflict");
            }
            return Ok("unchanged"); // Replay never extends retention or source freshness.
        }
    }
    tx.execute(
        "DELETE FROM grid_device_sources_v1 WHERE owner=?1 AND received<?2
        AND NOT(platform=?3 AND device=?4)",
        params![
            owner,
            now.saturating_sub(RETENTION_MS),
            value.platform,
            value.device_id
        ],
    )?;
    let (count, bytes): (usize, usize) = tx.query_row(
        "SELECT count(*),coalesce(sum(length(CAST(body AS BLOB))),0) FROM grid_device_sources_v1
         WHERE owner=?1 AND NOT (platform=?2 AND device=?3)",
        params![owner, value.platform, value.device_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if count >= MAX_SOURCES || bytes + body.len() > MAX_TOTAL {
        bail!("source_capacity");
    }
    tx.execute("INSERT INTO grid_device_sources_v1(owner,platform,device,sequence,digest,body,received)
        VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(owner,platform,device) DO UPDATE SET
        sequence=excluded.sequence,digest=excluded.digest,body=excluded.body,received=excluded.received",
        params![owner, value.platform, value.device_id, value.sequence, digest, body, now])?;
    tx.commit()?;
    Ok("accepted")
}
pub(super) fn read(conn: &Connection, owner: &str, now: u64) -> Result<Vec<Source>> {
    migrate(conn)?;
    let mut stmt = conn.prepare(
        "SELECT platform,device,body FROM grid_device_sources_v1
        WHERE owner=?1 AND received>=?2 ORDER BY platform,device LIMIT 17",
    )?;
    let rows = stmt.query_map(params![owner, now.saturating_sub(RETENTION_MS)], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut result = Vec::new();
    let mut bytes = 0;
    for row in rows {
        let (platform, device, body) = row?;
        bytes += body.len();
        if result.len() >= MAX_SOURCES || bytes > MAX_TOTAL {
            bail!("source_capacity");
        }
        let identity = serde_json::to_vec(&[owner, &platform, &device])?;
        result.push(Source {
            source_id: hex::encode(Sha256::digest(identity)),
            snapshot: serde_json::from_str(&body)?,
        });
    }
    Ok(result)
}
