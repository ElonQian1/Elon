use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{common::now, Store};

const MAX_REPORTS_PER_NODE: i64 = 100;

#[derive(Debug, Clone)]
pub struct RustCacheFleetReportInput {
    pub envelope_id: String,
    pub node_id: String,
    pub report_sha256: String,
    pub report_json: String,
    pub platform_health: String,
    pub gc_review_recommended: bool,
    pub active_writer_count: u64,
    pub managed_size_bytes: Option<u64>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeRustCacheFleetReport {
    pub envelope_id: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub report_sha256: String,
    #[serde(skip_serializing)]
    pub report_json: String,
    pub platform_health: String,
    pub gc_review_recommended: bool,
    pub active_writer_count: u64,
    pub managed_size_bytes: Option<u64>,
    pub generated_at: String,
    pub received_at: String,
}

#[derive(Debug, Clone)]
pub struct RustCacheFleetReportWrite {
    pub report: NodeRustCacheFleetReport,
    pub deduplicated: bool,
}

impl Store {
    pub fn record_rust_cache_fleet_report(
        &self,
        owner_user_id: &str,
        input: RustCacheFleetReportInput,
    ) -> Result<RustCacheFleetReportWrite> {
        validate_input(owner_user_id, &input)?;
        let received_at = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;

        if let Some(existing) = select_report_by_envelope(&tx, &input.envelope_id)? {
            require_same_report(&existing, owner_user_id, &input)?;
            tx.commit()?;
            return Ok(RustCacheFleetReportWrite {
                report: existing,
                deduplicated: true,
            });
        }
        if let Some(existing) = select_report_by_hash(&tx, &input.node_id, &input.report_sha256)? {
            require_same_report(&existing, owner_user_id, &input)?;
            tx.commit()?;
            return Ok(RustCacheFleetReportWrite {
                report: existing,
                deduplicated: true,
            });
        }

        tx.execute(
            "INSERT INTO node_rust_cache_fleet_reports (
               envelope_id, node_id, owner_user_id, report_sha256, report_json,
               platform_health, gc_review_recommended, active_writer_count,
               managed_size_bytes, generated_at, received_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                input.envelope_id,
                input.node_id,
                owner_user_id,
                input.report_sha256,
                input.report_json,
                input.platform_health,
                input.gc_review_recommended,
                u64_to_i64(input.active_writer_count, "active_writer_count")?,
                input
                    .managed_size_bytes
                    .map(|value| u64_to_i64(value, "managed_size_bytes"))
                    .transpose()?,
                input.generated_at,
                received_at,
            ],
        )?;
        tx.execute(
            "DELETE FROM node_rust_cache_fleet_reports
              WHERE node_id = ?1
                AND envelope_id NOT IN (
                  SELECT envelope_id
                    FROM node_rust_cache_fleet_reports
                   WHERE node_id = ?1
                   ORDER BY received_at DESC, envelope_id DESC
                   LIMIT ?2
                )",
            params![input.node_id, MAX_REPORTS_PER_NODE],
        )?;
        let stored = select_report_by_envelope(&tx, &input.envelope_id)?
            .ok_or_else(|| anyhow!("stored Rust cache fleet report disappeared"))?;
        tx.commit()?;
        Ok(RustCacheFleetReportWrite {
            report: stored,
            deduplicated: false,
        })
    }

    pub fn latest_rust_cache_fleet_report(
        &self,
        owner_user_id: &str,
        node_id: &str,
    ) -> Result<Option<NodeRustCacheFleetReport>> {
        let owner_user_id = owner_user_id.trim();
        let node_id = node_id.trim();
        if owner_user_id.is_empty() || node_id.is_empty() {
            return Ok(None);
        }
        let conn = self.conn()?;
        conn.query_row(
            "SELECT envelope_id, node_id, owner_user_id, report_sha256, report_json,
                    platform_health, gc_review_recommended, active_writer_count,
                    managed_size_bytes, generated_at, received_at
               FROM node_rust_cache_fleet_reports
              WHERE owner_user_id = ?1 AND node_id = ?2
              ORDER BY received_at DESC, envelope_id DESC
              LIMIT 1",
            params![owner_user_id, node_id],
            read_report,
        )
        .optional()
        .map_err(Into::into)
    }
}

fn validate_input(owner_user_id: &str, input: &RustCacheFleetReportInput) -> Result<()> {
    if owner_user_id.trim().is_empty()
        || input.node_id.trim().is_empty()
        || input.envelope_id.len() != 32
        || input.report_sha256.len() != 64
        || input.report_json.is_empty()
        || input.report_json.len() > 1_048_576
        || input.platform_health.trim().is_empty()
        || input.generated_at.trim().is_empty()
    {
        return Err(anyhow!("invalid Rust cache fleet report input"));
    }
    Ok(())
}

fn require_same_report(
    existing: &NodeRustCacheFleetReport,
    owner_user_id: &str,
    input: &RustCacheFleetReportInput,
) -> Result<()> {
    if existing.node_id != input.node_id
        || existing.owner_user_id != owner_user_id
        || existing.report_sha256 != input.report_sha256
        || existing.report_json != input.report_json
    {
        return Err(anyhow!("Rust cache fleet report identity conflict"));
    }
    Ok(())
}

fn select_report_by_envelope(
    conn: &rusqlite::Connection,
    envelope_id: &str,
) -> Result<Option<NodeRustCacheFleetReport>> {
    conn.query_row(
        "SELECT envelope_id, node_id, owner_user_id, report_sha256, report_json,
                platform_health, gc_review_recommended, active_writer_count,
                managed_size_bytes, generated_at, received_at
           FROM node_rust_cache_fleet_reports WHERE envelope_id = ?1",
        params![envelope_id],
        read_report,
    )
    .optional()
    .map_err(Into::into)
}

fn select_report_by_hash(
    conn: &rusqlite::Connection,
    node_id: &str,
    report_sha256: &str,
) -> Result<Option<NodeRustCacheFleetReport>> {
    conn.query_row(
        "SELECT envelope_id, node_id, owner_user_id, report_sha256, report_json,
                platform_health, gc_review_recommended, active_writer_count,
                managed_size_bytes, generated_at, received_at
           FROM node_rust_cache_fleet_reports
          WHERE node_id = ?1 AND report_sha256 = ?2",
        params![node_id, report_sha256],
        read_report,
    )
    .optional()
    .map_err(Into::into)
}

fn read_report(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeRustCacheFleetReport> {
    Ok(NodeRustCacheFleetReport {
        envelope_id: row.get(0)?,
        node_id: row.get(1)?,
        owner_user_id: row.get(2)?,
        report_sha256: row.get(3)?,
        report_json: row.get(4)?,
        platform_health: row.get(5)?,
        gc_review_recommended: row.get(6)?,
        active_writer_count: i64_to_u64(row.get(7)?, 7)?,
        managed_size_bytes: row
            .get::<_, Option<i64>>(8)?
            .map(|value| i64_to_u64(value, 8))
            .transpose()?,
        generated_at: row.get(9)?,
        received_at: row.get(10)?,
    })
}

fn u64_to_i64(value: u64, field: &str) -> Result<i64> {
    i64::try_from(value).map_err(|_| anyhow!("{field} exceeds SQLite INTEGER range"))
}

fn i64_to_u64(value: i64, column: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Integer,
            "negative unsigned Rust cache metric".into(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_write_is_idempotent_and_owner_scoped() {
        let path = std::env::temp_dir().join(format!(
            "elon-rust-cache-fleet-store-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let store = Store::open(&path).expect("store");
        let input = fixture("a".repeat(32), "1".repeat(64));
        let first = store
            .record_rust_cache_fleet_report("owner-a", input.clone())
            .expect("first report");
        assert!(!first.deduplicated);
        let replay = store
            .record_rust_cache_fleet_report("owner-a", input)
            .expect("replayed report");
        assert!(replay.deduplicated);
        assert!(store
            .latest_rust_cache_fleet_report("owner-b", "node-a")
            .unwrap()
            .is_none());
        assert_eq!(
            store
                .latest_rust_cache_fleet_report("owner-a", "node-a")
                .unwrap()
                .unwrap()
                .platform_health,
            "healthy"
        );
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn report_history_is_bounded_per_node() {
        let path = std::env::temp_dir().join(format!(
            "elon-rust-cache-fleet-history-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let store = Store::open(&path).expect("store");
        for index in 0..105_u64 {
            store
                .record_rust_cache_fleet_report(
                    "owner-a",
                    fixture(format!("{index:032x}"), format!("{index:064x}")),
                )
                .expect("report");
        }
        let count: i64 = store
            .conn()
            .expect("connection")
            .query_row(
                "SELECT COUNT(*) FROM node_rust_cache_fleet_reports WHERE node_id='node-a'",
                [],
                |row| row.get(0),
            )
            .expect("history count");
        assert_eq!(count, MAX_REPORTS_PER_NODE);
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    fn fixture(envelope_id: String, report_sha256: String) -> RustCacheFleetReportInput {
        RustCacheFleetReportInput {
            envelope_id,
            node_id: "node-a".into(),
            report_sha256,
            report_json: "{\"schema\":\"elon.rust_cache.fleet_report.v1\"}".into(),
            platform_health: "healthy".into(),
            gc_review_recommended: false,
            active_writer_count: 0,
            managed_size_bytes: Some(42),
            generated_at: "2026-08-16T00:00:00Z".into(),
        }
    }
}
