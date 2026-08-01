//! 节点算力执行证明：记录每次派发、完成、真实扣费与节点收益。

use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use std::collections::HashMap;

use super::{clean_optional, new_id, now, Store};

pub(crate) const SERVER_NODE_LLM_LEASE_SECONDS: i64 = 120;

#[derive(Debug, Clone, Serialize)]
pub struct NodeComputeRun {
    pub id: String,
    pub compute_call_id: String,
    pub consumer_user_id: String,
    pub provider_user_id: Option<String>,
    pub node_id: String,
    pub model_id: Option<String>,
    pub feature: String,
    pub usage_mode: String,
    pub billing_source: String,
    pub resource_owner_user_id: Option<String>,
    pub lease_id: Option<String>,
    pub offline_policy: String,
    pub replay_deadline: Option<String>,
    pub max_cost_rmb_fen: i64,
    pub allowance_id: Option<String>,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub billed_cost_rmb_fen: i64,
    pub provider_earned_fen: i64,
    pub settlement_status: Option<String>,
    pub route_reason: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NodeComputeRunStart<'a> {
    pub compute_call_id: &'a str,
    pub consumer_user_id: &'a str,
    pub provider_user_id: Option<&'a str>,
    pub node_id: &'a str,
    pub model_id: Option<&'a str>,
    pub feature: &'a str,
    pub usage_mode: &'a str,
    pub route_reason: Option<&'a str>,
}

pub struct NodeComputeRunFinish<'a> {
    pub provider_user_id: Option<&'a str>,
    pub status: &'a str,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub billed_cost_rmb_fen: i64,
    pub provider_earned_fen: i64,
    pub settlement_status: Option<&'a str>,
    pub error_message: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct NodeQualityScore {
    pub node_id: String,
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub avg_duration_ms: Option<i64>,
    pub last_finished_at: Option<String>,
    pub total_provider_earned_fen: i64,
    pub success_rate_x1000: i64,
}

impl Store {
    pub fn get_node_compute_run_by_compute_call_id(
        &self,
        compute_call_id: &str,
    ) -> Result<Option<NodeComputeRun>> {
        let compute_call_id = compute_call_id.trim();
        if compute_call_id.is_empty() {
            return Ok(None);
        }
        let conn = self.conn.lock().unwrap();
        select_run_by_compute_call_id(&conn, compute_call_id)
    }

    pub fn start_node_compute_run(&self, input: NodeComputeRunStart<'_>) -> Result<NodeComputeRun> {
        let compute_call_id = input.compute_call_id.trim();
        let ts = now();
        let conn = self.conn.lock().unwrap();

        if let Some(existing) = select_run_by_compute_call_id(&conn, compute_call_id)? {
            ensure_compute_run_replay_matches(&existing, &input)?;
            return Ok(existing);
        }

        let id = new_id("nrun");
        conn.execute(
            "INSERT INTO node_compute_runs (
               id, compute_call_id, consumer_user_id, provider_user_id,
               node_id, model_id, feature, usage_mode, status,
               started_at, route_reason, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'started', ?9, ?10, ?9, ?9)",
            params![
                id,
                compute_call_id,
                input.consumer_user_id,
                clean_optional(input.provider_user_id),
                input.node_id,
                clean_optional(input.model_id),
                input.feature,
                input.usage_mode,
                ts,
                clean_optional(input.route_reason),
            ],
        )?;
        select_run_by_compute_call_id(&conn, compute_call_id)?.ok_or_else(|| {
            anyhow::anyhow!("node compute run was inserted but could not be read back")
        })
    }

    pub fn finish_node_compute_run(
        &self,
        compute_call_id: &str,
        finish: NodeComputeRunFinish<'_>,
    ) -> Result<Option<NodeComputeRun>> {
        let compute_call_id = compute_call_id.trim();
        if compute_call_id.is_empty() {
            return Ok(None);
        }
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE node_compute_runs
                SET status = ?2,
                    finished_at = ?3,
                    duration_ms = MAX(0, CAST((julianday(?3) - julianday(started_at)) * 86400000 AS INTEGER)),
                    prompt_tokens = ?4,
                    completion_tokens = ?5,
                    billed_cost_rmb_fen = ?6,
                    provider_earned_fen = ?7,
                    settlement_status = ?8,
                    error_message = ?9,
                    provider_user_id = COALESCE(?10, provider_user_id),
                    updated_at = ?3
              WHERE compute_call_id = ?1",
            params![
                compute_call_id,
                normalize_status(finish.status),
                ts,
                finish.prompt_tokens.max(0),
                finish.completion_tokens.max(0),
                finish.billed_cost_rmb_fen.max(0),
                finish.provider_earned_fen.max(0),
                clean_optional(finish.settlement_status),
                truncate_optional(finish.error_message, 512),
                clean_optional(finish.provider_user_id),
            ],
        )?;
        select_run_by_compute_call_id(&conn, compute_call_id)
    }

    /// Refresh the execution lease while a node LLM stream is still open.
    /// Terminal or unrelated runs are never revived by a late heartbeat.
    pub fn heartbeat_started_server_node_llm_run(&self, compute_call_id: &str) -> Result<bool> {
        let compute_call_id = compute_call_id.trim();
        if compute_call_id.is_empty() {
            return Ok(false);
        }
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE node_compute_runs
                SET updated_at = ?2
              WHERE compute_call_id = ?1
                AND status = 'started'
                AND usage_mode = 'server_node_llm'",
            params![compute_call_id, ts],
        )?;
        Ok(updated > 0)
    }

    /// A server restart destroys every in-memory node LLM receiver. Return the
    /// interrupted rows so startup can also release their billing holds.
    pub fn mark_interrupted_started_server_node_llm_runs(&self) -> Result<Vec<NodeComputeRun>> {
        let ts = now();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let interrupted = {
            let mut stmt = tx.prepare(&format!(
                "{} WHERE status = 'started' AND usage_mode = 'server_node_llm'",
                run_select_sql()
            ))?;
            let rows = stmt.query_map([], read_run)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        tx.execute(
            "UPDATE node_compute_runs
                SET status = 'failed',
                    finished_at = ?1,
                    duration_ms = MAX(0, CAST((julianday(?1) - julianday(started_at)) * 86400000 AS INTEGER)),
                    settlement_status = COALESCE(settlement_status, 'released_error'),
                    error_message = COALESCE(error_message, 'server restarted before node LLM terminal event'),
                    updated_at = ?1
              WHERE status = 'started'
                AND usage_mode = 'server_node_llm'",
            params![ts],
        )?;
        tx.commit()?;
        Ok(interrupted)
    }

    pub fn mark_interrupted_started_pc_agent_runs(&self) -> Result<usize> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE node_compute_runs
                SET status = 'failed',
                    finished_at = ?1,
                    duration_ms = MAX(0, CAST((julianday(?1) - julianday(started_at)) * 86400000 AS INTEGER)),
                    error_message = COALESCE(error_message, 'server restarted before PC CLI terminal event'),
                    updated_at = ?1
              WHERE status = 'started'
                AND usage_mode = 'pc_agent_cli'",
            params![ts],
        )?;
        Ok(n)
    }

    pub fn admin_list_node_compute_runs(
        &self,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<NodeComputeRun>> {
        let status = status.and_then(normalize_status_filter);
        let limit = limit.clamp(1, 500);
        let conn = self.conn.lock().unwrap();
        if let Some(status) = status {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE status = ?1 ORDER BY started_at DESC LIMIT ?2",
                run_select_sql()
            ))?;
            let rows = stmt.query_map(params![status, limit], read_run)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        } else {
            let mut stmt = conn.prepare(&format!(
                "{} ORDER BY started_at DESC LIMIT ?1",
                run_select_sql()
            ))?;
            let rows = stmt.query_map(params![limit], read_run)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        }
    }

    pub fn list_node_compute_runs_for_consumer(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<NodeComputeRun>> {
        let limit = limit.clamp(1, 100);
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "{} WHERE consumer_user_id = ?1 ORDER BY started_at DESC LIMIT ?2",
            run_select_sql()
        ))?;
        let rows = stmt.query_map(params![user_id, limit], read_run)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn list_node_compute_runs_for_provider(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<NodeComputeRun>> {
        let limit = limit.clamp(1, 100);
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "{} WHERE provider_user_id = ?1 ORDER BY started_at DESC LIMIT ?2",
            run_select_sql()
        ))?;
        let rows = stmt.query_map(params![user_id, limit], read_run)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn node_quality_scores(&self) -> Result<HashMap<String, NodeQualityScore>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT
                node_id,
                COUNT(*) AS total_runs,
                SUM(CASE WHEN status IN ('settled', 'settled_no_provider', 'deduplicated') THEN 1 ELSE 0 END) AS successful_runs,
                SUM(CASE WHEN status IN ('failed', 'settlement_failed') THEN 1 ELSE 0 END) AS failed_runs,
                AVG(CASE WHEN duration_ms IS NOT NULL AND duration_ms >= 0 THEN duration_ms ELSE NULL END) AS avg_duration_ms,
                MAX(COALESCE(finished_at, started_at)) AS last_finished_at,
                SUM(provider_earned_fen) AS total_provider_earned_fen
             FROM (
                SELECT *
                  FROM node_compute_runs
                 WHERE status != 'started'
                 ORDER BY started_at DESC
                 LIMIT 2000
             )
             GROUP BY node_id",
        )?;
        let rows = stmt.query_map([], |row| {
            let total_runs: i64 = row.get(1)?;
            let successful_runs: i64 = row.get::<_, Option<i64>>(2)?.unwrap_or(0);
            let failed_runs: i64 = row.get::<_, Option<i64>>(3)?.unwrap_or(0);
            let avg_duration_ms = row
                .get::<_, Option<f64>>(4)?
                .map(|value| value.round().max(0.0) as i64);
            let success_rate_x1000 = if total_runs > 0 {
                (successful_runs * 1000 / total_runs).clamp(0, 1000)
            } else {
                0
            };
            Ok(NodeQualityScore {
                node_id: row.get(0)?,
                total_runs,
                successful_runs,
                failed_runs,
                avg_duration_ms,
                last_finished_at: row.get(5)?,
                total_provider_earned_fen: row.get::<_, Option<i64>>(6)?.unwrap_or(0),
                success_rate_x1000,
            })
        })?;
        let mut scores = HashMap::new();
        for row in rows {
            let score = row?;
            scores.insert(score.node_id.clone(), score);
        }
        Ok(scores)
    }
}

pub(super) fn select_run_by_compute_call_id(
    conn: &rusqlite::Connection,
    compute_call_id: &str,
) -> Result<Option<NodeComputeRun>> {
    conn.query_row(
        &format!("{} WHERE compute_call_id = ?1", run_select_sql()),
        params![compute_call_id],
        read_run,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn ensure_compute_run_replay_matches(
    existing: &NodeComputeRun,
    input: &NodeComputeRunStart<'_>,
) -> Result<()> {
    let provider_user_id = input
        .provider_user_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let model_id = input
        .model_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if existing.consumer_user_id != input.consumer_user_id.trim()
        || existing.provider_user_id.as_deref() != provider_user_id
        || existing.node_id != input.node_id.trim()
        || existing.model_id.as_deref() != model_id
        || existing.feature != input.feature.trim()
        || existing.usage_mode != input.usage_mode.trim()
    {
        bail!("同一算力调用编号不能绑定到不同的用户、节点、模型或用途");
    }
    Ok(())
}

fn run_select_sql() -> &'static str {
    "SELECT id, compute_call_id, consumer_user_id, provider_user_id,
            node_id, model_id, feature, usage_mode,
            billing_source, resource_owner_user_id, lease_id, offline_policy,
            replay_deadline, max_cost_rmb_fen, allowance_id, status,
            started_at, finished_at, duration_ms, prompt_tokens,
            completion_tokens, billed_cost_rmb_fen, provider_earned_fen,
            settlement_status, route_reason, error_message, created_at, updated_at
       FROM node_compute_runs"
}

fn read_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeComputeRun> {
    Ok(NodeComputeRun {
        id: row.get(0)?,
        compute_call_id: row.get(1)?,
        consumer_user_id: row.get(2)?,
        provider_user_id: row.get(3)?,
        node_id: row.get(4)?,
        model_id: row.get(5)?,
        feature: row.get(6)?,
        usage_mode: row.get(7)?,
        billing_source: row.get(8)?,
        resource_owner_user_id: row.get(9)?,
        lease_id: row.get(10)?,
        offline_policy: row.get(11)?,
        replay_deadline: row.get(12)?,
        max_cost_rmb_fen: row.get(13)?,
        allowance_id: row.get(14)?,
        status: row.get(15)?,
        started_at: row.get(16)?,
        finished_at: row.get(17)?,
        duration_ms: row.get(18)?,
        prompt_tokens: row.get(19)?,
        completion_tokens: row.get(20)?,
        billed_cost_rmb_fen: row.get(21)?,
        provider_earned_fen: row.get(22)?,
        settlement_status: row.get(23)?,
        route_reason: row.get(24)?,
        error_message: row.get(25)?,
        created_at: row.get(26)?,
        updated_at: row.get(27)?,
    })
}

fn normalize_status(status: &str) -> &str {
    match status.trim() {
        "started" => "started",
        "settled" => "settled",
        "settled_no_provider" => "settled_no_provider",
        "settlement_skipped" => "settlement_skipped",
        "settlement_failed" => "settlement_failed",
        "verification_pending" => "verification_pending",
        "deduplicated" => "deduplicated",
        "released_no_usage" => "released_no_usage",
        "released_error" => "released_error",
        "failed" => "failed",
        _ => "failed",
    }
}

fn normalize_status_filter(status: &str) -> Option<&str> {
    let status = status.trim();
    if status.is_empty() || status == "all" {
        return None;
    }
    Some(normalize_status(status))
}

fn truncate_optional(value: Option<&str>, max_len: usize) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    Some(value.chars().take(max_len).collect())
}

#[cfg(test)]
#[path = "node_compute_runs_tests.rs"]
mod tests;
