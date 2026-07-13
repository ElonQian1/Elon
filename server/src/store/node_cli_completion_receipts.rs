//! Durable server-side inbox for PC CLI terminal-event replay.

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{now, Store};

const MAX_IDENTIFIER_CHARS: usize = 200;
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
const MAX_REASON_CHARS: usize = 2_000;

#[derive(Debug, Clone)]
pub struct NodeCliCompletionReceiptInput<'a> {
    pub event_id: &'a str,
    pub req_id: &'a str,
    pub compute_call_id: &'a str,
    pub node_id: &'a str,
    pub user_id: &'a str,
    pub payload_json: &'a str,
    pub payload_sha256: &'a str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NodeCliCompletionReceipt {
    pub event_id: String,
    pub req_id: String,
    pub compute_call_id: String,
    pub node_id: String,
    pub user_id: String,
    pub payload_json: String,
    pub payload_sha256: String,
    pub status: String,
    pub token_usage_event_id: Option<String>,
    pub billing_event_id: Option<String>,
    pub node_transaction_id: Option<String>,
    pub reason: Option<String>,
    pub attempt_count: i64,
    pub received_at: String,
    pub updated_at: String,
    pub last_attempt_at: Option<String>,
    pub applied_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeCliCompletionIngestOutcome {
    Inserted(NodeCliCompletionReceipt),
    Duplicate(NodeCliCompletionReceipt),
    Conflict {
        existing: NodeCliCompletionReceipt,
        reason: String,
    },
}

impl NodeCliCompletionIngestOutcome {
    pub fn receipt(&self) -> &NodeCliCompletionReceipt {
        match self {
            Self::Inserted(receipt) | Self::Duplicate(receipt) => receipt,
            Self::Conflict { existing, .. } => existing,
        }
    }

    pub fn accepted(&self) -> bool {
        !matches!(self, Self::Conflict { .. })
    }

    pub fn deduplicated(&self) -> bool {
        matches!(self, Self::Duplicate(_))
    }
}

impl Store {
    pub fn ingest_node_cli_completion_receipt(
        &self,
        input: NodeCliCompletionReceiptInput<'_>,
    ) -> Result<NodeCliCompletionIngestOutcome> {
        let event_id = required_identifier(input.event_id, "event_id")?;
        let req_id = required_identifier(input.req_id, "req_id")?;
        let compute_call_id = required_identifier(input.compute_call_id, "compute_call_id")?;
        let node_id = required_identifier(input.node_id, "node_id")?;
        let user_id = required_identifier(input.user_id, "user_id")?;
        let payload_json = input.payload_json.trim();
        if payload_json.is_empty() {
            return Err(anyhow!("payload_json 不能为空"));
        }
        if payload_json.len() > MAX_PAYLOAD_BYTES {
            return Err(anyhow!("payload_json 超过 {} 字节限制", MAX_PAYLOAD_BYTES));
        }
        serde_json::from_str::<serde_json::Value>(payload_json)
            .map_err(|error| anyhow!("payload_json 不是有效 JSON: {error}"))?;
        let payload_sha256 = normalized_sha256(input.payload_sha256)?;

        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        if let Some(existing) = select_receipt_by_event_id(&tx, &event_id)? {
            let outcome = if existing.payload_sha256 != payload_sha256 {
                NodeCliCompletionIngestOutcome::Conflict {
                    existing,
                    reason: "event_payload_hash_mismatch".to_string(),
                }
            } else if existing.req_id != req_id
                || existing.compute_call_id != compute_call_id
                || existing.node_id != node_id
                || existing.user_id != user_id
            {
                NodeCliCompletionIngestOutcome::Conflict {
                    existing,
                    reason: "event_binding_mismatch".to_string(),
                }
            } else {
                NodeCliCompletionIngestOutcome::Duplicate(existing)
            };
            tx.commit()?;
            return Ok(outcome);
        }

        if let Some(existing) = select_receipt_by_request_or_call(&tx, &req_id, &compute_call_id)? {
            let reason = if existing.req_id == req_id {
                "request_already_bound_to_other_event"
            } else {
                "compute_call_already_bound_to_other_event"
            };
            tx.commit()?;
            return Ok(NodeCliCompletionIngestOutcome::Conflict {
                existing,
                reason: reason.to_string(),
            });
        }

        let ts = now();
        tx.execute(
            "INSERT INTO node_cli_completion_receipts (
               event_id, req_id, compute_call_id, node_id, user_id,
               payload_json, payload_sha256, status, attempt_count,
               received_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', 0, ?8, ?8)",
            params![
                event_id,
                req_id,
                compute_call_id,
                node_id,
                user_id,
                payload_json,
                payload_sha256,
                ts,
            ],
        )?;
        let inserted = select_receipt_by_event_id(&tx, &event_id)?
            .ok_or_else(|| anyhow!("CLI completion receipt 插入后无法读取"))?;
        tx.commit()?;
        Ok(NodeCliCompletionIngestOutcome::Inserted(inserted))
    }

    pub fn get_node_cli_completion_receipt(
        &self,
        event_id: &str,
    ) -> Result<Option<NodeCliCompletionReceipt>> {
        let event_id = event_id.trim();
        if event_id.is_empty() {
            return Ok(None);
        }
        let conn = self.conn.lock().unwrap();
        select_receipt_by_event_id(&conn, event_id)
    }

    pub fn get_node_cli_completion_receipt_by_compute_call(
        &self,
        compute_call_id: &str,
    ) -> Result<Option<NodeCliCompletionReceipt>> {
        let compute_call_id = compute_call_id.trim();
        if compute_call_id.is_empty() {
            return Ok(None);
        }
        self.conn()?
            .query_row(
                &format!("{} WHERE compute_call_id = ?1", receipt_select_sql()),
                params![compute_call_id],
                read_receipt,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_pending_node_cli_completion_receipts(
        &self,
        limit: usize,
    ) -> Result<Vec<NodeCliCompletionReceipt>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE status IN ('pending', 'retry')
                  OR (status = 'processing' AND last_attempt_at < ?1)
             ORDER BY COALESCE(last_attempt_at, received_at), event_id LIMIT ?2",
            receipt_select_sql()
        ))?;
        let rows = stmt.query_map(params![cutoff, limit.clamp(1, 500) as i64], read_receipt)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    /// Acquire an expiring processing lease. Only the lease owner may publish
    /// the final receipt state, preventing live replay and the background worker
    /// from applying/ACKing the same event concurrently.
    pub fn claim_node_cli_completion_receipt(
        &self,
        event_id: &str,
        claim_id: &str,
    ) -> Result<Option<NodeCliCompletionReceipt>> {
        let event_id = required_identifier(event_id, "event_id")?;
        let claim_id = required_identifier(claim_id, "claim_id")?;
        let ts = now();
        let cutoff = (chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE node_cli_completion_receipts
                SET status = 'processing',
                    reason = ?2,
                    attempt_count = attempt_count + 1,
                    last_attempt_at = ?3,
                    updated_at = ?3
              WHERE event_id = ?1
                AND (
                    status IN ('pending', 'retry')
                    OR (status = 'processing' AND last_attempt_at < ?4)
                )",
            params![event_id, claim_id, ts, cutoff],
        )?;
        let receipt = if changed > 0 {
            select_receipt_by_event_id(&tx, &event_id)?
        } else {
            None
        };
        tx.commit()?;
        Ok(receipt)
    }

    pub fn finish_node_cli_completion_claim_applied(
        &self,
        event_id: &str,
        claim_id: &str,
        token_usage_event_id: Option<&str>,
        billing_event_id: Option<&str>,
        node_transaction_id: Option<&str>,
    ) -> Result<bool> {
        self.finish_node_cli_completion_claim(
            event_id,
            claim_id,
            "applied",
            None,
            token_usage_event_id,
            billing_event_id,
            node_transaction_id,
        )
    }

    pub fn finish_node_cli_completion_claim_retry(
        &self,
        event_id: &str,
        claim_id: &str,
        reason: &str,
    ) -> Result<bool> {
        self.finish_node_cli_completion_claim(
            event_id,
            claim_id,
            "retry",
            Some(reason),
            None,
            None,
            None,
        )
    }

    pub fn finish_node_cli_completion_claim_rejected(
        &self,
        event_id: &str,
        claim_id: &str,
        reason: &str,
    ) -> Result<bool> {
        self.finish_node_cli_completion_claim(
            event_id,
            claim_id,
            "rejected",
            Some(reason),
            None,
            None,
            None,
        )
    }

    fn finish_node_cli_completion_claim(
        &self,
        event_id: &str,
        claim_id: &str,
        status: &str,
        reason: Option<&str>,
        token_usage_event_id: Option<&str>,
        billing_event_id: Option<&str>,
        node_transaction_id: Option<&str>,
    ) -> Result<bool> {
        let event_id = required_identifier(event_id, "event_id")?;
        let claim_id = required_identifier(claim_id, "claim_id")?;
        let status = match status {
            "applied" | "retry" | "rejected" => status,
            _ => return Err(anyhow!("不支持的 completion claim 终态")),
        };
        let reason = reason.map(|value| truncate_chars(value.trim(), MAX_REASON_CHARS));
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(
            "UPDATE node_cli_completion_receipts
                SET status = ?3,
                    token_usage_event_id = COALESCE(?4, token_usage_event_id),
                    billing_event_id = COALESCE(?5, billing_event_id),
                    node_transaction_id = COALESCE(?6, node_transaction_id),
                    reason = ?7,
                    applied_at = CASE WHEN ?3 = 'applied' THEN COALESCE(applied_at, ?8) ELSE applied_at END,
                    updated_at = ?8
              WHERE event_id = ?1
                AND status = 'processing'
                AND reason = ?2",
            params![
                event_id,
                claim_id,
                status,
                clean_optional(token_usage_event_id),
                clean_optional(billing_event_id),
                clean_optional(node_transaction_id),
                reason,
                ts,
            ],
        )?;
        Ok(changed > 0)
    }

    pub fn mark_node_cli_completion_applied(
        &self,
        event_id: &str,
        token_usage_event_id: Option<&str>,
        billing_event_id: Option<&str>,
        node_transaction_id: Option<&str>,
    ) -> Result<Option<NodeCliCompletionReceipt>> {
        let event_id = required_identifier(event_id, "event_id")?;
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE node_cli_completion_receipts
                SET status = 'applied',
                    token_usage_event_id = COALESCE(?2, token_usage_event_id),
                    billing_event_id = COALESCE(?3, billing_event_id),
                    node_transaction_id = COALESCE(?4, node_transaction_id),
                    reason = NULL,
                    attempt_count = attempt_count + 1,
                    last_attempt_at = ?5,
                    applied_at = COALESCE(applied_at, ?5),
                    updated_at = ?5
              WHERE event_id = ?1
                AND status IN ('pending', 'retry')",
            params![
                event_id,
                clean_optional(token_usage_event_id),
                clean_optional(billing_event_id),
                clean_optional(node_transaction_id),
                ts,
            ],
        )?;
        let receipt = select_receipt_by_event_id(&tx, &event_id)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub fn mark_node_cli_completion_retry(
        &self,
        event_id: &str,
        reason: &str,
    ) -> Result<Option<NodeCliCompletionReceipt>> {
        self.mark_node_cli_completion_nonterminal(event_id, "retry", reason)
    }

    pub fn mark_node_cli_completion_rejected(
        &self,
        event_id: &str,
        reason: &str,
    ) -> Result<Option<NodeCliCompletionReceipt>> {
        self.mark_node_cli_completion_nonterminal(event_id, "rejected", reason)
    }

    fn mark_node_cli_completion_nonterminal(
        &self,
        event_id: &str,
        status: &str,
        reason: &str,
    ) -> Result<Option<NodeCliCompletionReceipt>> {
        let event_id = required_identifier(event_id, "event_id")?;
        let status = match status {
            "retry" => "retry",
            "rejected" => "rejected",
            _ => return Err(anyhow!("不支持的 completion receipt 状态")),
        };
        let reason = truncate_chars(reason.trim(), MAX_REASON_CHARS);
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE node_cli_completion_receipts
                SET status = ?2,
                    reason = ?3,
                    attempt_count = attempt_count + 1,
                    last_attempt_at = ?4,
                    updated_at = ?4
              WHERE event_id = ?1
                AND status IN ('pending', 'retry')",
            params![event_id, status, reason, ts],
        )?;
        let receipt = select_receipt_by_event_id(&tx, &event_id)?;
        tx.commit()?;
        Ok(receipt)
    }
}

fn select_receipt_by_event_id(
    conn: &rusqlite::Connection,
    event_id: &str,
) -> Result<Option<NodeCliCompletionReceipt>> {
    conn.query_row(
        &format!("{} WHERE event_id = ?1", receipt_select_sql()),
        params![event_id],
        read_receipt,
    )
    .optional()
    .map_err(Into::into)
}

fn select_receipt_by_request_or_call(
    conn: &rusqlite::Connection,
    req_id: &str,
    compute_call_id: &str,
) -> Result<Option<NodeCliCompletionReceipt>> {
    conn.query_row(
        &format!(
            "{} WHERE req_id = ?1 OR compute_call_id = ?2 LIMIT 1",
            receipt_select_sql()
        ),
        params![req_id, compute_call_id],
        read_receipt,
    )
    .optional()
    .map_err(Into::into)
}

fn receipt_select_sql() -> &'static str {
    "SELECT event_id, req_id, compute_call_id, node_id, user_id,
            payload_json, payload_sha256, status,
            token_usage_event_id, billing_event_id, node_transaction_id,
            reason, attempt_count, received_at, updated_at,
            last_attempt_at, applied_at
       FROM node_cli_completion_receipts"
}

fn read_receipt(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeCliCompletionReceipt> {
    Ok(NodeCliCompletionReceipt {
        event_id: row.get(0)?,
        req_id: row.get(1)?,
        compute_call_id: row.get(2)?,
        node_id: row.get(3)?,
        user_id: row.get(4)?,
        payload_json: row.get(5)?,
        payload_sha256: row.get(6)?,
        status: row.get(7)?,
        token_usage_event_id: row.get(8)?,
        billing_event_id: row.get(9)?,
        node_transaction_id: row.get(10)?,
        reason: row.get(11)?,
        attempt_count: row.get(12)?,
        received_at: row.get(13)?,
        updated_at: row.get(14)?,
        last_attempt_at: row.get(15)?,
        applied_at: row.get(16)?,
    })
}

fn required_identifier(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("{field} 不能为空"));
    }
    if value.chars().count() > MAX_IDENTIFIER_CHARS {
        return Err(anyhow!("{field} 超过 {MAX_IDENTIFIER_CHARS} 字符限制"));
    }
    if value.chars().any(char::is_control) {
        return Err(anyhow!("{field} 包含控制字符"));
    }
    Ok(value.to_string())
}

fn normalized_sha256(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(anyhow!("payload_sha256 必须是 64 位十六进制 SHA-256"));
    }
    Ok(value)
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(MAX_IDENTIFIER_CHARS).collect())
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
#[path = "node_cli_completion_receipts_tests.rs"]
mod tests;
