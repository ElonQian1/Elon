//! Owner opt-in, model allowlist and bounded admission for shared node inference.

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use super::{
    clean_optional, new_id,
    node_compute_plugin_sharing::{
        NodeComputePluginSharingAuthorization, NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
    },
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
    pub plugin_runtime_requested: bool,
    pub plugin_policy_revision: i64,
    pub plugin_policy_digest: Option<String>,
    pub plugin_consent_schema: Option<String>,
    pub plugin_consent_receipt_id: Option<String>,
    pub plugin_installation_identity_digest: Option<String>,
    pub plugin_authorization: Option<NodeComputePluginSharingAuthorization>,
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
            plugin_runtime_requested: false,
            plugin_policy_revision: 0,
            plugin_policy_digest: None,
            plugin_consent_schema: None,
            plugin_consent_receipt_id: None,
            plugin_installation_identity_digest: None,
            plugin_authorization: None,
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
        Ok(self
            .update_node_compute_sharing_policy_with_plugin_runtime(
                owner_user_id,
                node_id,
                update,
                None,
            )?
            .status)
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

pub(super) fn sharing_status(
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
                    max_concurrent_runs, daily_token_limit, plugin_runtime_requested,
                    plugin_policy_revision, plugin_policy_digest, plugin_consent_schema,
                    plugin_consent_receipt_id, plugin_installation_identity_digest,
                    plugin_authorization_ref, plugin_authorization_revision,
                    plugin_authorization_digest, created_at, updated_at,
                    CASE WHEN plugin_policy_revision=0 THEN 1 ELSE EXISTS (
                      SELECT 1 FROM node_compute_plugin_sharing_consents c
                       WHERE c.receipt_id=node_compute_sharing_policies.plugin_consent_receipt_id
                         AND c.node_id=node_compute_sharing_policies.node_id
                         AND c.owner_user_id=node_compute_sharing_policies.owner_user_id
                         AND c.consent_schema=node_compute_sharing_policies.plugin_consent_schema
                         AND c.installation_identity_digest=node_compute_sharing_policies.plugin_installation_identity_digest
                         AND c.policy_revision=node_compute_sharing_policies.plugin_policy_revision
                         AND c.policy_digest=node_compute_sharing_policies.plugin_policy_digest
                         AND c.plugin_runtime_requested=node_compute_sharing_policies.plugin_runtime_requested
                         AND c.plugin_runtime_requested=node_compute_sharing_policies.enabled
                         AND c.allowed_model_ids_json=node_compute_sharing_policies.allowed_model_ids_json
                         AND c.max_concurrent_runs=node_compute_sharing_policies.max_concurrent_runs
                         AND c.daily_token_limit=node_compute_sharing_policies.daily_token_limit
                         AND c.authorization_ref IS node_compute_sharing_policies.plugin_authorization_ref
                         AND c.authorization_revision IS node_compute_sharing_policies.plugin_authorization_revision
                         AND c.authorization_digest IS node_compute_sharing_policies.plugin_authorization_digest
                    ) END
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
    let allowed_model_ids =
        serde_json::from_str::<Vec<String>>(&json).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let enabled = row.get::<_, i64>(2)? != 0;
    let plugin_runtime_requested = row.get::<_, i64>(6)? != 0;
    let plugin_policy_revision: i64 = row.get(7)?;
    let plugin_policy_digest: Option<String> = row.get(8)?;
    let plugin_consent_schema: Option<String> = row.get(9)?;
    let plugin_consent_receipt_id: Option<String> = row.get(10)?;
    let plugin_installation_identity_digest: Option<String> = row.get(11)?;
    let plugin_authorization = match (row.get(12)?, row.get(13)?, row.get(14)?) {
        (Some(authorization_ref), Some(revision), Some(digest)) => {
            Some(NodeComputePluginSharingAuthorization {
                authorization_ref,
                revision,
                digest,
            })
        }
        (None, None, None) => None,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    if plugin_runtime_requested != plugin_authorization.is_some() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let head_is_valid = if plugin_policy_revision == 0 {
        !plugin_runtime_requested
            && plugin_policy_digest.is_none()
            && plugin_consent_schema.is_none()
            && plugin_consent_receipt_id.is_none()
            && plugin_installation_identity_digest.is_none()
            && plugin_authorization.is_none()
    } else {
        plugin_policy_revision > 0
            && enabled == plugin_runtime_requested
            && plugin_policy_digest
                .as_ref()
                .is_some_and(|value| value.len() == 64)
            && plugin_consent_schema.as_deref()
                == Some(NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1)
            && plugin_consent_receipt_id.is_some()
            && plugin_installation_identity_digest
                .as_ref()
                .is_some_and(|value| value.len() == 64)
            && plugin_authorization.as_ref().is_none_or(|authorization| {
                authorization.revision == plugin_policy_revision
                    && Some(&authorization.digest) == plugin_policy_digest.as_ref()
            })
    };
    if !head_is_valid {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if row.get::<_, i64>(17)? != 1 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(NodeComputeSharingPolicy {
        node_id: row.get(0)?,
        owner_user_id: row.get(1)?,
        enabled,
        allowed_model_ids,
        max_concurrent_runs: row.get(4)?,
        daily_token_limit: row.get(5)?,
        plugin_runtime_requested,
        plugin_policy_revision,
        plugin_policy_digest,
        plugin_consent_schema,
        plugin_consent_receipt_id,
        plugin_installation_identity_digest,
        plugin_authorization,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

pub(super) fn normalize_model_ids(values: &[String]) -> Result<Vec<String>> {
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
