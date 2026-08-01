//! Owner-only runtime health derived from durable shared inference runs.

use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{node_compute_runs::SERVER_NODE_LLM_LEASE_SECONDS, now, Store};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NodeComputeSharingRuntimeHealth {
    pub node_id: String,
    pub status: String,
    pub completed_runs_24h: i64,
    pub failed_runs_24h: i64,
    pub budget_overrun_runs_24h: i64,
    pub budget_overrun_tokens_24h: i64,
    pub expired_active_runs: i64,
    pub attention_codes: Vec<String>,
    pub evaluated_at: String,
}

impl Store {
    pub fn node_compute_sharing_runtime_health(
        &self,
        node_id: &str,
        owner_user_id: &str,
    ) -> Result<NodeComputeSharingRuntimeHealth> {
        let node_id = node_id.trim();
        let owner_user_id = owner_user_id.trim();
        if node_id.is_empty() || owner_user_id.is_empty() {
            bail!("节点和所有者不能为空");
        }
        let conn = self.conn.lock().unwrap();
        let owns_node = conn
            .query_row(
                "SELECT 1 FROM node_credentials
                  WHERE agent_id=?1 AND owner_user_id=?2",
                params![node_id, owner_user_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !owns_node {
            bail!("节点不存在或不属于当前用户");
        }

        let (completed_runs_24h, failed_runs_24h, budget_overrun_runs_24h, budget_overrun_tokens_24h) =
            conn.query_row(
                "SELECT
                   COUNT(*),
                   SUM(CASE WHEN status IN ('failed', 'settlement_failed', 'released_error') THEN 1 ELSE 0 END),
                   SUM(CASE
                         WHEN reserved_token_budget > 0
                          AND prompt_tokens + completion_tokens > reserved_token_budget
                         THEN 1 ELSE 0 END),
                   SUM(CASE
                         WHEN reserved_token_budget > 0
                          AND prompt_tokens + completion_tokens > reserved_token_budget
                         THEN prompt_tokens + completion_tokens - reserved_token_budget
                         ELSE 0 END)
                 FROM node_compute_runs
                WHERE node_id=?1
                  AND usage_mode='server_node_llm'
                  AND status NOT IN ('started', 'usage_received')
                  AND provider_user_id=?2
                  AND consumer_user_id <> provider_user_id
                  AND julianday(COALESCE(finished_at, updated_at)) >= julianday('now', '-24 hours')",
                params![node_id, owner_user_id],
                |row| {
                    Ok((
                        row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    ))
                },
            )?;
        let expired_active_runs = conn.query_row(
            "SELECT COUNT(*)
               FROM node_compute_runs
              WHERE node_id=?1
                AND usage_mode='server_node_llm'
                AND status='started'
                AND provider_user_id=?2
                AND consumer_user_id <> provider_user_id
                AND julianday(updated_at) < julianday('now', ?3)",
            params![
                node_id,
                owner_user_id,
                format!("-{SERVER_NODE_LLM_LEASE_SECONDS} seconds")
            ],
            |row| row.get::<_, i64>(0),
        )?;

        let mut attention_codes = Vec::new();
        if budget_overrun_runs_24h > 0 {
            attention_codes.push("token_budget_overrun".to_string());
        }
        if expired_active_runs > 0 {
            attention_codes.push("expired_active_run".to_string());
        }
        if failed_runs_24h > 0 {
            attention_codes.push("recent_execution_failure".to_string());
        }
        let status = if budget_overrun_runs_24h > 0 || expired_active_runs > 0 {
            "critical"
        } else if failed_runs_24h > 0 {
            "warning"
        } else {
            "healthy"
        };
        Ok(NodeComputeSharingRuntimeHealth {
            node_id: node_id.to_string(),
            status: status.to_string(),
            completed_runs_24h,
            failed_runs_24h,
            budget_overrun_runs_24h,
            budget_overrun_tokens_24h,
            expired_active_runs,
            attention_codes,
            evaluated_at: now(),
        })
    }
}

#[cfg(test)]
#[path = "node_compute_sharing_health_tests.rs"]
mod tests;
