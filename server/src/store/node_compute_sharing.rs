//! Owner opt-in, model allowlist and bounded admission for shared node inference.

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use super::{
    clean_optional, new_id,
    node_compute_runs::{
        ensure_compute_run_replay_matches, select_run_by_compute_call_id,
        SERVER_NODE_LLM_LEASE_SECONDS,
    },
    now, NodeComputeRun, NodeComputeRunStart, Store,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NodeComputeSharingPolicy {
    pub node_id: String,
    pub owner_user_id: String,
    pub enabled: bool,
    pub allowed_model_ids: Vec<String>,
    pub max_concurrent_runs: i64,
    pub daily_token_limit: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NodeComputeSharingStatus {
    pub policy: NodeComputeSharingPolicy,
    pub active_runs: i64,
    pub tokens_used_today: i64,
    pub tokens_reserved_today: i64,
    pub available: bool,
    pub availability: String,
}

#[derive(Debug, Clone)]
pub struct UpdateNodeComputeSharingPolicy {
    pub enabled: bool,
    pub allowed_model_ids: Vec<String>,
    pub max_concurrent_runs: i64,
    pub daily_token_limit: i64,
}

impl NodeComputeSharingPolicy {
    pub fn disabled(node_id: &str, owner_user_id: &str) -> Self {
        Self {
            node_id: node_id.trim().to_string(),
            owner_user_id: owner_user_id.trim().to_string(),
            enabled: false,
            allowed_model_ids: Vec::new(),
            max_concurrent_runs: 1,
            daily_token_limit: 0,
            created_at: None,
            updated_at: None,
        }
    }
}

impl Store {
    pub fn node_compute_sharing_status(
        &self,
        node_id: &str,
        owner_user_id: &str,
        model_id: Option<&str>,
    ) -> Result<NodeComputeSharingStatus> {
        let conn = self.conn.lock().unwrap();
        sharing_status(&conn, node_id, owner_user_id, model_id)
    }

    pub fn update_node_compute_sharing_policy(
        &self,
        owner_user_id: &str,
        node_id: &str,
        update: UpdateNodeComputeSharingPolicy,
    ) -> Result<NodeComputeSharingStatus> {
        let node_id = node_id.trim();
        let owner_user_id = owner_user_id.trim();
        if node_id.is_empty() || owner_user_id.is_empty() {
            bail!("节点和所有者不能为空");
        }
        let allowed_model_ids = normalize_model_ids(&update.allowed_model_ids)?;
        if update.enabled && allowed_model_ids.is_empty() {
            bail!("开启共享前至少选择一个允许共享的模型");
        }
        if !(1..=16).contains(&update.max_concurrent_runs) {
            bail!("共享并发上限必须在 1 到 16 之间");
        }
        if !(0..=1_000_000_000_000).contains(&update.daily_token_limit) {
            bail!("每日 Token 上限必须在 0 到 1000000000000 之间");
        }

        let ts = now();
        let allowed_json = serde_json::to_string(&allowed_model_ids)?;
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
        conn.execute(
            "INSERT INTO node_compute_sharing_policies (
               node_id, owner_user_id, enabled, allowed_model_ids_json,
               max_concurrent_runs, daily_token_limit, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(node_id) DO UPDATE SET
               owner_user_id=excluded.owner_user_id,
               enabled=excluded.enabled,
               allowed_model_ids_json=excluded.allowed_model_ids_json,
               max_concurrent_runs=excluded.max_concurrent_runs,
               daily_token_limit=excluded.daily_token_limit,
               updated_at=excluded.updated_at",
            params![
                node_id,
                owner_user_id,
                if update.enabled { 1 } else { 0 },
                allowed_json,
                update.max_concurrent_runs,
                update.daily_token_limit,
                ts,
            ],
        )?;
        sharing_status(&conn, node_id, owner_user_id, None)
    }

    /// Atomically re-check the provider policy and reserve a shared inference slot.
    /// Idempotent retries return the existing run even if the owner disabled sharing later.
    pub fn claim_shared_node_compute_run_with_budget(
        &self,
        input: NodeComputeRunStart<'_>,
        reserved_token_budget: i64,
    ) -> Result<NodeComputeRun> {
        let compute_call_id = input.compute_call_id.trim();
        let provider_user_id = input
            .provider_user_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("共享节点执行缺少提供者身份"))?;
        let model_id = input
            .model_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("共享节点执行缺少模型标识"))?;
        if !(1..=1_000_000_000_000).contains(&reserved_token_budget) {
            bail!("共享节点 Token 预留必须在 1 到 1000000000000 之间");
        }
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = select_run_by_compute_call_id(&tx, compute_call_id)? {
            ensure_compute_run_replay_matches(&existing, &input)?;
            if existing.reserved_token_budget != reserved_token_budget {
                bail!("同一算力调用编号不能改变 Token 预留预算");
            }
            tx.commit()?;
            return Ok(existing);
        }

        let status = sharing_status(&tx, input.node_id, provider_user_id, Some(model_id))?;
        if !status.available {
            bail!("共享节点当前不可接单：{}", status.availability);
        }
        let requested_total = i128::from(status.tokens_used_today)
            + i128::from(status.tokens_reserved_today)
            + i128::from(reserved_token_budget);
        if status.policy.daily_token_limit > 0
            && requested_total > i128::from(status.policy.daily_token_limit)
        {
            bail!("共享节点当前不可接单：daily_token_reservation_exceeds_limit");
        }

        let id = new_id("nrun");
        let ts = now();
        tx.execute(
            "INSERT INTO node_compute_runs (
               id, compute_call_id, consumer_user_id, provider_user_id,
               node_id, model_id, feature, usage_mode, status,
               started_at, route_reason, reserved_token_budget, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'started', ?9, ?10, ?11, ?9, ?9)",
            params![
                id,
                compute_call_id,
                input.consumer_user_id,
                provider_user_id,
                input.node_id,
                model_id,
                input.feature,
                input.usage_mode,
                ts,
                clean_optional(input.route_reason),
                reserved_token_budget,
            ],
        )?;
        let run = select_run_by_compute_call_id(&tx, compute_call_id)?
            .ok_or_else(|| anyhow::anyhow!("共享节点执行已占用但无法读回"))?;
        tx.commit()?;
        Ok(run)
    }

    #[cfg(test)]
    pub fn claim_shared_node_compute_run(
        &self,
        input: NodeComputeRunStart<'_>,
    ) -> Result<NodeComputeRun> {
        self.claim_shared_node_compute_run_with_budget(input, 1)
    }
}

fn sharing_status(
    conn: &Connection,
    node_id: &str,
    owner_user_id: &str,
    model_id: Option<&str>,
) -> Result<NodeComputeSharingStatus> {
    let node_id = node_id.trim();
    let owner_user_id = owner_user_id.trim();
    let policy = conn
        .query_row(
            "SELECT node_id, owner_user_id, enabled, allowed_model_ids_json,
                    max_concurrent_runs, daily_token_limit, created_at, updated_at
               FROM node_compute_sharing_policies WHERE node_id=?1",
            params![node_id],
            read_policy,
        )
        .optional()?
        .unwrap_or_else(|| NodeComputeSharingPolicy::disabled(node_id, owner_user_id));
    let active_runs = conn.query_row(
        "SELECT COUNT(*) FROM node_compute_runs
          WHERE node_id=?1 AND usage_mode='server_node_llm' AND status='started'
            AND provider_user_id IS NOT NULL
            AND consumer_user_id <> provider_user_id
            AND julianday(updated_at) >= julianday('now', ?2)",
        params![node_id, format!("-{SERVER_NODE_LLM_LEASE_SECONDS} seconds")],
        |row| row.get::<_, i64>(0),
    )?;
    let tokens_used_today = conn.query_row(
        "SELECT COALESCE(SUM(prompt_tokens + completion_tokens), 0)
           FROM node_compute_runs
          WHERE node_id=?1 AND usage_mode='server_node_llm'
            AND provider_user_id IS NOT NULL
            AND consumer_user_id <> provider_user_id
            AND date(started_at)=date('now')",
        params![node_id],
        |row| row.get::<_, i64>(0),
    )?;
    let tokens_reserved_today = conn.query_row(
        "SELECT COALESCE(SUM(reserved_token_budget), 0)
           FROM node_compute_runs
          WHERE node_id=?1 AND usage_mode='server_node_llm' AND status='started'
            AND provider_user_id IS NOT NULL
            AND consumer_user_id <> provider_user_id
            AND date(started_at)=date('now')
            AND julianday(updated_at) >= julianday('now', ?2)",
        params![node_id, format!("-{SERVER_NODE_LLM_LEASE_SECONDS} seconds")],
        |row| row.get::<_, i64>(0),
    )?;

    let model_id = model_id.map(str::trim).filter(|value| !value.is_empty());
    let availability = if policy.owner_user_id != owner_user_id {
        "owner_mismatch"
    } else if !policy.enabled {
        "sharing_disabled"
    } else if policy.allowed_model_ids.is_empty() {
        "no_allowed_models"
    } else if model_id.is_some_and(|model| {
        !policy
            .allowed_model_ids
            .iter()
            .any(|allowed| allowed == model)
    }) {
        "model_not_allowed"
    } else if active_runs >= policy.max_concurrent_runs {
        "concurrency_limit_reached"
    } else if policy.daily_token_limit > 0
        && i128::from(tokens_used_today) + i128::from(tokens_reserved_today)
            >= i128::from(policy.daily_token_limit)
    {
        "daily_token_limit_reached"
    } else {
        "available"
    };
    Ok(NodeComputeSharingStatus {
        policy,
        active_runs,
        tokens_used_today,
        tokens_reserved_today,
        available: availability == "available",
        availability: availability.to_string(),
    })
}

fn read_policy(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeComputeSharingPolicy> {
    let json: String = row.get(3)?;
    let allowed_model_ids = serde_json::from_str::<Vec<String>>(&json).unwrap_or_default();
    Ok(NodeComputeSharingPolicy {
        node_id: row.get(0)?,
        owner_user_id: row.get(1)?,
        enabled: row.get::<_, i64>(2)? != 0,
        allowed_model_ids,
        max_concurrent_runs: row.get(4)?,
        daily_token_limit: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn normalize_model_ids(values: &[String]) -> Result<Vec<String>> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > 160 {
            bail!("模型标识不能超过 160 个字符");
        }
        if !normalized.iter().any(|existing| existing == value) {
            normalized.push(value.to_string());
        }
    }
    if normalized.len() > 64 {
        bail!("单个节点最多共享 64 个模型");
    }
    Ok(normalized)
}

#[cfg(test)]
#[path = "node_compute_sharing_tests.rs"]
mod tests;
