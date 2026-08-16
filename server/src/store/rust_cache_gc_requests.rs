use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::super::{common::now, Store};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct RustCacheGcOptions {
    pub force_aged: bool,
    pub workspace_only: bool,
    pub recover_missing_workspaces: bool,
    pub shared_aliases_only: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeRustCacheGcRequest {
    pub request_id: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub status: String,
    pub options: RustCacheGcOptions,
    pub plan_id: Option<String>,
    pub plan_digest: Option<String>,
    #[serde(skip_serializing)]
    pub plan_summary_json: Option<String>,
    #[serde(skip_serializing)]
    pub result_summary_json: Option<String>,
    pub failure_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: String,
}

impl Store {
    pub fn create_rust_cache_gc_request(
        &self,
        owner_user_id: &str,
        node_id: &str,
        options: RustCacheGcOptions,
    ) -> Result<NodeRustCacheGcRequest> {
        validate_owner_node(owner_user_id, node_id)?;
        validate_options(options)?;
        let request_id = uuid::Uuid::new_v4().simple().to_string();
        let created_at = now();
        let expires_at = (Utc::now() + Duration::hours(24)).to_rfc3339();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        expire_stale_requests(&tx, node_id)?;
        let active_count: u64 = tx.query_row(
            "SELECT COUNT(*) FROM node_rust_cache_gc_requests
              WHERE node_id=?1 AND status IN ('requested','plan_ready','approved','executing')",
            params![node_id.trim()],
            |row| row.get(0),
        )?;
        if active_count > 0 {
            return Err(anyhow!("node already has an active GC request"));
        }
        tx.execute(
            "INSERT INTO node_rust_cache_gc_requests (
               request_id, node_id, owner_user_id, status, force_aged,
               workspace_only, recover_missing, shared_aliases_only,
               created_at, updated_at, expires_at
             ) VALUES (?1, ?2, ?3, 'requested', ?4, ?5, ?6, ?7, ?8, ?8, ?9)",
            params![
                request_id,
                node_id.trim(),
                owner_user_id.trim(),
                options.force_aged,
                options.workspace_only,
                options.recover_missing_workspaces,
                options.shared_aliases_only,
                created_at,
                expires_at,
            ],
        )?;
        prune_terminal_requests(&tx, node_id, 100)?;
        let request =
            select_request(&tx, &request_id)?.ok_or_else(|| anyhow!("GC request disappeared"))?;
        tx.commit()?;
        Ok(request)
    }

    pub fn latest_rust_cache_gc_request(
        &self,
        owner_user_id: &str,
        node_id: &str,
    ) -> Result<Option<NodeRustCacheGcRequest>> {
        let conn = self.conn()?;
        expire_stale_requests(&conn, node_id)?;
        conn.query_row(
            "SELECT request_id, node_id, owner_user_id, status, force_aged,
                    workspace_only, recover_missing, shared_aliases_only,
                    plan_id, plan_digest, plan_summary_json, result_summary_json,
                    failure_code, created_at, updated_at, expires_at
               FROM node_rust_cache_gc_requests
              WHERE owner_user_id = ?1 AND node_id = ?2
              ORDER BY updated_at DESC, request_id DESC LIMIT 1",
            params![owner_user_id.trim(), node_id.trim()],
            read_request,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn rust_cache_gc_request_for_node(
        &self,
        node_id: &str,
        request_id: &str,
    ) -> Result<Option<NodeRustCacheGcRequest>> {
        let conn = self.conn()?;
        let request = select_request(&conn, request_id)?;
        Ok(request.filter(|request| request.node_id == node_id.trim()))
    }

    pub fn next_rust_cache_gc_request_for_node(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeRustCacheGcRequest>> {
        let conn = self.conn()?;
        expire_stale_requests(&conn, node_id)?;
        let mut request = conn
            .query_row(
                "SELECT request_id, node_id, owner_user_id, status, force_aged,
                        workspace_only, recover_missing, shared_aliases_only,
                        plan_id, plan_digest, plan_summary_json, result_summary_json,
                        failure_code, created_at, updated_at, expires_at
                   FROM node_rust_cache_gc_requests
                  WHERE node_id = ?1 AND status IN ('requested', 'approved', 'executing')
                  ORDER BY CASE status WHEN 'executing' THEN 0 WHEN 'approved' THEN 1 ELSE 2 END,
                           created_at, request_id LIMIT 1",
                params![node_id.trim()],
                read_request,
            )
            .optional()?;
        if let Some(item) = request.as_mut().filter(|item| item.status == "approved") {
            let updated_at = now();
            conn.execute(
                "UPDATE node_rust_cache_gc_requests SET status='executing', updated_at=?2
                  WHERE request_id=?1 AND status='approved'",
                params![item.request_id, updated_at],
            )?;
            item.status = "executing".into();
            item.updated_at = updated_at;
        }
        Ok(request)
    }

    pub fn record_rust_cache_gc_plan(
        &self,
        node_id: &str,
        request_id: &str,
        plan_id: &str,
        plan_digest: &str,
        plan_summary_json: &str,
    ) -> Result<NodeRustCacheGcRequest> {
        validate_plan_identity(plan_id, plan_digest, plan_summary_json)?;
        let conn = self.conn()?;
        let current =
            select_request(&conn, request_id)?.ok_or_else(|| anyhow!("GC request not found"))?;
        require_node(&current, node_id)?;
        if current.status == "plan_ready" {
            if current.plan_id.as_deref() == Some(plan_id)
                && current.plan_digest.as_deref() == Some(plan_digest)
                && current.plan_summary_json.as_deref() == Some(plan_summary_json)
            {
                return Ok(current);
            }
            return Err(anyhow!("GC plan identity conflict"));
        }
        if current.status != "requested" {
            return Err(anyhow!("GC request no longer accepts a plan"));
        }
        let updated_at = now();
        let changed = conn.execute(
            "UPDATE node_rust_cache_gc_requests
                SET status='plan_ready', plan_id=?2, plan_digest=?3,
                    plan_summary_json=?4, updated_at=?5
              WHERE request_id=?1 AND status='requested'",
            params![
                request_id,
                plan_id,
                plan_digest,
                plan_summary_json,
                updated_at
            ],
        )?;
        let updated =
            select_request(&conn, request_id)?.ok_or_else(|| anyhow!("GC request disappeared"))?;
        if changed == 1
            || (updated.status == "plan_ready"
                && updated.plan_id.as_deref() == Some(plan_id)
                && updated.plan_digest.as_deref() == Some(plan_digest)
                && updated.plan_summary_json.as_deref() == Some(plan_summary_json))
        {
            Ok(updated)
        } else {
            Err(anyhow!("GC request changed before plan recording"))
        }
    }

    pub fn approve_rust_cache_gc_request(
        &self,
        owner_user_id: &str,
        request_id: &str,
        plan_id: &str,
        plan_digest: &str,
    ) -> Result<NodeRustCacheGcRequest> {
        let conn = self.conn()?;
        let current =
            select_request(&conn, request_id)?.ok_or_else(|| anyhow!("GC request not found"))?;
        require_owner(&current, owner_user_id)?;
        if current.status == "approved" || current.status == "executing" {
            require_plan_match(&current, plan_id, plan_digest)?;
            return Ok(current);
        }
        if current.status != "plan_ready" {
            return Err(anyhow!("GC request is not ready for approval"));
        }
        if request_expired(&current)? {
            return Err(anyhow!("GC request has expired"));
        }
        require_plan_match(&current, plan_id, plan_digest)?;
        let updated_at = now();
        let changed = conn.execute(
            "UPDATE node_rust_cache_gc_requests SET status='approved', updated_at=?2
              WHERE request_id=?1 AND status='plan_ready'",
            params![request_id, updated_at],
        )?;
        if changed != 1 {
            return Err(anyhow!("GC request changed before approval"));
        }
        select_request(&conn, request_id)?.ok_or_else(|| anyhow!("GC request disappeared"))
    }

    pub fn reject_rust_cache_gc_request(
        &self,
        owner_user_id: &str,
        request_id: &str,
    ) -> Result<NodeRustCacheGcRequest> {
        let conn = self.conn()?;
        let current =
            select_request(&conn, request_id)?.ok_or_else(|| anyhow!("GC request not found"))?;
        require_owner(&current, owner_user_id)?;
        if current.status == "rejected" {
            return Ok(current);
        }
        if !matches!(
            current.status.as_str(),
            "requested" | "plan_ready" | "approved"
        ) {
            return Err(anyhow!("GC request can no longer be rejected"));
        }
        let updated_at = now();
        let changed = conn.execute(
            "UPDATE node_rust_cache_gc_requests SET status='rejected', updated_at=?2
              WHERE request_id=?1 AND status IN ('requested','plan_ready','approved')",
            params![request_id, updated_at],
        )?;
        if changed != 1 {
            return Err(anyhow!("GC request changed before rejection"));
        }
        select_request(&conn, request_id)?.ok_or_else(|| anyhow!("GC request disappeared"))
    }

    pub fn finish_rust_cache_gc_request(
        &self,
        node_id: &str,
        request_id: &str,
        status: &str,
        result_summary_json: &str,
        failure_code: Option<&str>,
    ) -> Result<NodeRustCacheGcRequest> {
        if !matches!(status, "completed" | "partial" | "failed")
            || result_summary_json.is_empty()
            || result_summary_json.len() > 131_072
            || failure_code.is_some_and(|value| value.len() > 120)
        {
            return Err(anyhow!("invalid GC result"));
        }
        let conn = self.conn()?;
        let current =
            select_request(&conn, request_id)?.ok_or_else(|| anyhow!("GC request not found"))?;
        require_node(&current, node_id)?;
        if matches!(current.status.as_str(), "completed" | "partial" | "failed") {
            if current.status == status
                && current.result_summary_json.as_deref() == Some(result_summary_json)
                && current.failure_code.as_deref() == failure_code
            {
                return Ok(current);
            }
            return Err(anyhow!("GC result identity conflict"));
        }
        let allowed =
            current.status == "executing" || (current.status == "requested" && status == "failed");
        if !allowed {
            return Err(anyhow!("GC request cannot accept this result"));
        }
        let updated_at = now();
        let expected_status = if current.status == "executing" {
            "executing"
        } else {
            "requested"
        };
        let changed = conn.execute(
            "UPDATE node_rust_cache_gc_requests
                SET status=?2, result_summary_json=?3, failure_code=?4, updated_at=?5
              WHERE request_id=?1 AND status=?6",
            params![
                request_id,
                status,
                result_summary_json,
                failure_code,
                updated_at,
                expected_status
            ],
        )?;
        if changed != 1 {
            let updated = select_request(&conn, request_id)?
                .ok_or_else(|| anyhow!("GC request disappeared"))?;
            if updated.status == status
                && updated.result_summary_json.as_deref() == Some(result_summary_json)
                && updated.failure_code.as_deref() == failure_code
            {
                return Ok(updated);
            }
            return Err(anyhow!("GC request changed before result recording"));
        }
        select_request(&conn, request_id)?.ok_or_else(|| anyhow!("GC request disappeared"))
    }
}

fn validate_owner_node(owner: &str, node: &str) -> Result<()> {
    if owner.trim().is_empty() || node.trim().is_empty() || node.len() > 160 {
        return Err(anyhow!("invalid GC request owner or node"));
    }
    Ok(())
}

fn validate_options(options: RustCacheGcOptions) -> Result<()> {
    if options.workspace_only || options.recover_missing_workspaces || options.shared_aliases_only {
        return Err(anyhow!(
            "project-specific GC filters must be reviewed and run locally"
        ));
    }
    Ok(())
}

fn validate_plan_identity(id: &str, digest: &str, json: &str) -> Result<()> {
    if !valid_hex(id, 32) || !valid_hex(digest, 64) || json.is_empty() || json.len() > 131_072 {
        return Err(anyhow!("invalid GC plan identity"));
    }
    Ok(())
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value == value.to_ascii_lowercase()
        && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn require_owner(request: &NodeRustCacheGcRequest, owner: &str) -> Result<()> {
    if request.owner_user_id != owner.trim() {
        return Err(anyhow!("GC request owner mismatch"));
    }
    Ok(())
}

fn require_node(request: &NodeRustCacheGcRequest, node: &str) -> Result<()> {
    if request.node_id != node.trim() {
        return Err(anyhow!("GC request node mismatch"));
    }
    Ok(())
}

fn require_plan_match(request: &NodeRustCacheGcRequest, id: &str, digest: &str) -> Result<()> {
    if request.plan_id.as_deref() != Some(id) || request.plan_digest.as_deref() != Some(digest) {
        return Err(anyhow!("GC approval is not bound to the current plan"));
    }
    Ok(())
}

fn request_expired(request: &NodeRustCacheGcRequest) -> Result<bool> {
    Ok(chrono::DateTime::parse_from_rfc3339(&request.expires_at)? <= Utc::now())
}

fn expire_stale_requests(conn: &rusqlite::Connection, node_id: &str) -> Result<()> {
    let current = now();
    conn.execute(
        "UPDATE node_rust_cache_gc_requests SET status='expired', updated_at=?2
          WHERE node_id=?1 AND status IN ('requested','plan_ready','approved') AND expires_at <= ?2",
        params![node_id.trim(), current],
    )?;
    let execution_cutoff = (Utc::now() - Duration::hours(7)).to_rfc3339();
    conn.execute(
        "UPDATE node_rust_cache_gc_requests
            SET status='failed', failure_code='execution-timeout', updated_at=?2
          WHERE node_id=?1 AND status='executing' AND updated_at <= ?3",
        params![node_id.trim(), current, execution_cutoff],
    )?;
    Ok(())
}

fn prune_terminal_requests(conn: &rusqlite::Connection, node_id: &str, keep: u64) -> Result<()> {
    conn.execute(
        "DELETE FROM node_rust_cache_gc_requests
          WHERE node_id=?1
            AND status IN ('rejected','completed','partial','failed','expired')
            AND request_id NOT IN (
              SELECT request_id FROM node_rust_cache_gc_requests
               WHERE node_id=?1
                 AND status IN ('rejected','completed','partial','failed','expired')
               ORDER BY updated_at DESC, request_id DESC
               LIMIT ?2
            )",
        params![node_id.trim(), keep],
    )?;
    Ok(())
}

fn select_request(conn: &rusqlite::Connection, id: &str) -> Result<Option<NodeRustCacheGcRequest>> {
    conn.query_row(
        "SELECT request_id, node_id, owner_user_id, status, force_aged,
                workspace_only, recover_missing, shared_aliases_only,
                plan_id, plan_digest, plan_summary_json, result_summary_json,
                failure_code, created_at, updated_at, expires_at
           FROM node_rust_cache_gc_requests WHERE request_id=?1",
        params![id],
        read_request,
    )
    .optional()
    .map_err(Into::into)
}

fn read_request(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeRustCacheGcRequest> {
    Ok(NodeRustCacheGcRequest {
        request_id: row.get(0)?,
        node_id: row.get(1)?,
        owner_user_id: row.get(2)?,
        status: row.get(3)?,
        options: RustCacheGcOptions {
            force_aged: row.get(4)?,
            workspace_only: row.get(5)?,
            recover_missing_workspaces: row.get(6)?,
            shared_aliases_only: row.get(7)?,
        },
        plan_id: row.get(8)?,
        plan_digest: row.get(9)?,
        plan_summary_json: row.get(10)?,
        result_summary_json: row.get(11)?,
        failure_code: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        expires_at: row.get(15)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_plan_approval_and_result_are_idempotent() {
        let path = std::env::temp_dir().join(format!(
            "elon-cache-gc-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let store = Store::open(&path).unwrap();
        let request = store
            .create_rust_cache_gc_request("owner", "node", RustCacheGcOptions::default())
            .unwrap();
        assert!(store
            .create_rust_cache_gc_request("owner", "node", RustCacheGcOptions::default())
            .is_err());
        let summary = serde_json::json!({"safe": true}).to_string();
        let planned = store
            .record_rust_cache_gc_plan(
                "node",
                &request.request_id,
                &"a".repeat(32),
                &"b".repeat(64),
                &summary,
            )
            .unwrap();
        assert_eq!(planned.status, "plan_ready");
        assert!(store
            .approve_rust_cache_gc_request(
                "owner",
                &request.request_id,
                &"c".repeat(32),
                &"b".repeat(64)
            )
            .is_err());
        let approved = store
            .approve_rust_cache_gc_request(
                "owner",
                &request.request_id,
                &"a".repeat(32),
                &"b".repeat(64),
            )
            .unwrap();
        assert_eq!(approved.status, "approved");
        assert_eq!(
            store
                .next_rust_cache_gc_request_for_node("node")
                .unwrap()
                .unwrap()
                .status,
            "executing"
        );
        let result = serde_json::json!({"status":"completed"}).to_string();
        let completed = store
            .finish_rust_cache_gc_request("node", &request.request_id, "completed", &result, None)
            .unwrap();
        assert_eq!(completed.status, "completed");
        assert_eq!(
            store
                .finish_rust_cache_gc_request(
                    "node",
                    &request.request_id,
                    "completed",
                    &result,
                    None
                )
                .unwrap()
                .status,
            "completed"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rust_cache_gc_stale_execution_releases_node() {
        let path = std::env::temp_dir().join(format!(
            "elon-cache-gc-stale-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let store = Store::open(&path).unwrap();
        let request = store
            .create_rust_cache_gc_request("owner", "node", RustCacheGcOptions::default())
            .unwrap();
        store
            .record_rust_cache_gc_plan(
                "node",
                &request.request_id,
                &"a".repeat(32),
                &"b".repeat(64),
                &serde_json::json!({"safe": true}).to_string(),
            )
            .unwrap();
        store
            .approve_rust_cache_gc_request(
                "owner",
                &request.request_id,
                &"a".repeat(32),
                &"b".repeat(64),
            )
            .unwrap();
        store
            .next_rust_cache_gc_request_for_node("node")
            .unwrap()
            .unwrap();
        store
            .conn()
            .unwrap()
            .execute(
                "UPDATE node_rust_cache_gc_requests SET updated_at=?2 WHERE request_id=?1",
                params![
                    request.request_id,
                    (Utc::now() - Duration::hours(8)).to_rfc3339()
                ],
            )
            .unwrap();

        let stale = store
            .latest_rust_cache_gc_request("owner", "node")
            .unwrap()
            .unwrap();
        assert_eq!(stale.status, "failed");
        assert_eq!(stale.failure_code.as_deref(), Some("execution-timeout"));
        assert!(store
            .create_rust_cache_gc_request("owner", "node", RustCacheGcOptions::default())
            .is_ok());
        let _ = std::fs::remove_file(path);
    }
}
