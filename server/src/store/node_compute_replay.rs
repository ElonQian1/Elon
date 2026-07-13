//! Offline replay policy bound to a dispatched node compute run.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::{new_id, node_compute_runs::select_run_by_compute_call_id, now, NodeComputeRun, Store};

#[derive(Debug, Clone, Copy)]
pub struct NodeComputeReplayBinding<'a> {
    pub billing_source: &'a str,
    pub resource_owner_user_id: Option<&'a str>,
    pub lease_id: Option<&'a str>,
    pub offline_policy: &'a str,
    pub replay_deadline: Option<&'a str>,
    pub max_cost_rmb_fen: i64,
    pub allowance_id: Option<&'a str>,
}

/// Exact pre-update identity for a replay-policy compare-and-set.  The
/// `updated_at` check is the version fence; the remaining fields make a stale
/// caller fail closed even when two writes happen within the same clock tick.
#[derive(Debug, Clone, Copy)]
pub struct NodeComputeReplayExpectation<'a> {
    pub consumer_user_id: &'a str,
    pub node_id: &'a str,
    pub billing_source: &'a str,
    pub resource_owner_user_id: Option<&'a str>,
    pub lease_id: Option<&'a str>,
    pub offline_policy: &'a str,
    pub updated_at: &'a str,
}

pub struct LocalOfflineNodeComputeRunClaim<'a> {
    pub compute_call_id: &'a str,
    pub request_id: &'a str,
    pub owner_user_id: &'a str,
    pub node_id: &'a str,
    pub project_id: &'a str,
    pub conversation_id: &'a str,
    pub model_id: Option<&'a str>,
}

#[derive(Debug)]
pub enum LocalOfflineNodeComputeRunClaimOutcome {
    Claimed { run: NodeComputeRun, created: bool },
    Conflict { reason: String },
}

impl Store {
    /// Atomically creates the complete owner-local replay binding, or proves
    /// that an existing run/session is exactly the same claim. No existing
    /// cloud/shared/platform row is ever modified by this path.
    pub fn claim_local_offline_node_compute_run(
        &self,
        claim: LocalOfflineNodeComputeRunClaim<'_>,
    ) -> Result<LocalOfflineNodeComputeRunClaimOutcome> {
        let compute_call_id = required_identifier("compute_call_id", claim.compute_call_id)?;
        let request_id = required_identifier("request_id", claim.request_id)?;
        let owner_user_id = required_identifier("owner_user_id", claim.owner_user_id)?;
        let node_id = required_identifier("node_id", claim.node_id)?;
        let project_id = required_identifier("project_id", claim.project_id)?;
        let conversation_id = required_identifier("conversation_id", claim.conversation_id)?;
        let model_id = optional_identifier("model_id", claim.model_id)?;
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        let session_identity = tx
            .query_row(
                "SELECT project_id, conversation_id, user_id, node_id
                   FROM project_execution_sessions WHERE request_id = ?1 LIMIT 1",
                params![request_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let existing = select_run_by_compute_call_id(&tx, &compute_call_id)?;
        if let Some(run) = existing {
            if !local_claim_matches(&run, &owner_user_id, &node_id, model_id.as_deref()) {
                tx.commit()?;
                return Ok(LocalOfflineNodeComputeRunClaimOutcome::Conflict {
                    reason: "req_id 已绑定其他云端、共享或不同身份的计算运行".to_string(),
                });
            }
            let Some(identity) = session_identity.as_ref() else {
                tx.commit()?;
                return Ok(LocalOfflineNodeComputeRunClaimOutcome::Conflict {
                    reason: "req_id 已存在没有原子项目身份绑定的本机计算运行".to_string(),
                });
            };
            if identity.0 != project_id
                || identity.1 != conversation_id
                || identity.2 != owner_user_id
                || identity.3 != node_id
            {
                tx.commit()?;
                return Ok(LocalOfflineNodeComputeRunClaimOutcome::Conflict {
                    reason: "req_id 已绑定不同身份的项目执行会话".to_string(),
                });
            }
            tx.commit()?;
            return Ok(LocalOfflineNodeComputeRunClaimOutcome::Claimed {
                run,
                created: false,
            });
        }

        if session_identity.is_some() {
            tx.commit()?;
            return Ok(LocalOfflineNodeComputeRunClaimOutcome::Conflict {
                reason: "req_id 已存在没有对应本机计算运行的项目执行会话".to_string(),
            });
        }

        let id = new_id("nrun");
        tx.execute(
            "INSERT INTO node_compute_runs (
                id, compute_call_id, consumer_user_id, provider_user_id,
                node_id, model_id, feature, usage_mode,
                billing_source, resource_owner_user_id, lease_id, offline_policy,
                replay_deadline, max_cost_rmb_fen, allowance_id, status,
                started_at, route_reason, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?3, ?4, ?5, 'pc_agent_cli_offline_dev', 'pc_agent_cli',
                'own_codex', ?3, NULL, 'allow_offline', NULL, 0, NULL, 'started',
                ?6, 'owner_local_offline', ?6, ?6
             )",
            params![
                id,
                compute_call_id,
                owner_user_id,
                node_id,
                model_id.as_deref(),
                ts
            ],
        )?;
        tx.execute(
            "INSERT INTO project_execution_sessions (
                id, project_id, conversation_id, user_id, node_id, request_id,
                isolated, status, model, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 'running', ?7, ?8, ?8)",
            params![
                new_id("pes"),
                project_id,
                conversation_id,
                owner_user_id,
                node_id,
                request_id,
                model_id.as_deref(),
                ts,
            ],
        )?;
        let run = select_run_by_compute_call_id(&tx, &compute_call_id)?
            .ok_or_else(|| anyhow!("local offline compute run insert could not be read back"))?;
        tx.commit()?;
        Ok(LocalOfflineNodeComputeRunClaimOutcome::Claimed { run, created: true })
    }

    /// Freezes the billing facts needed to decide whether a disconnected node may
    /// finish a task and replay its receipt later. A completed run cannot be rebound.
    pub fn bind_node_compute_run_replay_policy(
        &self,
        compute_call_id: &str,
        binding: NodeComputeReplayBinding<'_>,
    ) -> Result<Option<NodeComputeRun>> {
        let compute_call_id = required_identifier("compute_call_id", compute_call_id)?;
        let billing_source = normalize_billing_source(binding.billing_source)?;
        let offline_policy = normalize_offline_policy(binding.offline_policy)?;
        validate_source_policy(billing_source, offline_policy)?;
        let resource_owner_user_id =
            optional_identifier("resource_owner_user_id", binding.resource_owner_user_id)?;
        let lease_id = optional_identifier("lease_id", binding.lease_id)?;
        let allowance_id = optional_identifier("allowance_id", binding.allowance_id)?;
        let replay_deadline = normalize_replay_deadline(binding.replay_deadline)?;
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE node_compute_runs
                SET billing_source = ?2,
                    resource_owner_user_id = ?3,
                    lease_id = ?4,
                    offline_policy = ?5,
                    replay_deadline = ?6,
                    max_cost_rmb_fen = ?7,
                    allowance_id = ?8,
                    updated_at = ?9
              WHERE compute_call_id = ?1
                AND status = 'started'",
            params![
                compute_call_id,
                billing_source,
                resource_owner_user_id,
                lease_id,
                offline_policy,
                replay_deadline,
                binding.max_cost_rmb_fen.max(0),
                allowance_id,
                ts,
            ],
        )?;
        drop(conn);
        self.get_node_compute_run_by_compute_call_id(&compute_call_id)
    }

    /// Rebind only when the run still has the exact identity/version observed by
    /// the caller. Emergency credential issuance uses the transaction-level
    /// helper below so the lease row and this CAS commit together.
    pub fn compare_and_bind_node_compute_run_replay_policy(
        &self,
        compute_call_id: &str,
        expected: NodeComputeReplayExpectation<'_>,
        binding: NodeComputeReplayBinding<'_>,
    ) -> Result<Option<NodeComputeRun>> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let rebound = compare_and_bind_node_compute_run_replay_policy_in_tx(
            &tx,
            compute_call_id,
            expected,
            binding,
        )?;
        tx.commit()?;
        Ok(rebound)
    }

    /// Applies the current offline replay policy using the immutable dispatch
    /// binding. `replay_deadline` limits own/local authorization; a shared or
    /// platform terminal receipt may arrive after its execution deadline, but
    /// only while its durable dispatch/verification hold still exists.
    pub fn can_replay_node_compute_run_offline(&self, compute_call_id: &str) -> Result<bool> {
        let Some(run) = self.get_node_compute_run_by_compute_call_id(compute_call_id)? else {
            return Ok(false);
        };
        match (run.billing_source.as_str(), run.offline_policy.as_str()) {
            ("own_codex", "allow_offline") => {
                Ok(!replay_deadline_has_passed(run.replay_deadline.as_deref()))
            }
            ("shared_codex" | "platform", "require_active_reservation") => self
                .billing_reservation_is_still_reserved(&run.consumer_user_id, &run.compute_call_id),
            _ => Ok(false),
        }
    }
}

pub(super) fn compare_and_bind_node_compute_run_replay_policy_in_tx(
    conn: &Connection,
    compute_call_id: &str,
    expected: NodeComputeReplayExpectation<'_>,
    binding: NodeComputeReplayBinding<'_>,
) -> Result<Option<NodeComputeRun>> {
    let compute_call_id = required_identifier("compute_call_id", compute_call_id)?;
    let expected_consumer_user_id =
        required_identifier("expected.consumer_user_id", expected.consumer_user_id)?;
    let expected_node_id = required_identifier("expected.node_id", expected.node_id)?;
    let expected_billing_source = normalize_billing_source(expected.billing_source)?;
    let expected_resource_owner_user_id = optional_identifier(
        "expected.resource_owner_user_id",
        expected.resource_owner_user_id,
    )?;
    let expected_lease_id = optional_identifier("expected.lease_id", expected.lease_id)?;
    let expected_offline_policy = normalize_offline_policy(expected.offline_policy)?;
    let expected_updated_at = required_identifier("expected.updated_at", expected.updated_at)?;

    let billing_source = normalize_billing_source(binding.billing_source)?;
    let offline_policy = normalize_offline_policy(binding.offline_policy)?;
    validate_source_policy(billing_source, offline_policy)?;
    let resource_owner_user_id =
        optional_identifier("resource_owner_user_id", binding.resource_owner_user_id)?;
    let lease_id = optional_identifier("lease_id", binding.lease_id)?;
    let allowance_id = optional_identifier("allowance_id", binding.allowance_id)?;
    let replay_deadline = normalize_replay_deadline(binding.replay_deadline)?;
    let ts = now();
    let changed = conn.execute(
        "UPDATE node_compute_runs
            SET billing_source = ?2,
                resource_owner_user_id = ?3,
                lease_id = ?4,
                offline_policy = ?5,
                replay_deadline = ?6,
                max_cost_rmb_fen = ?7,
                allowance_id = ?8,
                updated_at = ?9
          WHERE compute_call_id = ?1
            AND status = 'started'
            AND consumer_user_id = ?10
            AND node_id = ?11
            AND billing_source = ?12
            AND resource_owner_user_id IS ?13
            AND lease_id IS ?14
            AND offline_policy = ?15
            AND updated_at = ?16",
        params![
            compute_call_id,
            billing_source,
            resource_owner_user_id,
            lease_id,
            offline_policy,
            replay_deadline,
            binding.max_cost_rmb_fen.max(0),
            allowance_id,
            ts,
            expected_consumer_user_id,
            expected_node_id,
            expected_billing_source,
            expected_resource_owner_user_id,
            expected_lease_id,
            expected_offline_policy,
            expected_updated_at,
        ],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    select_run_by_compute_call_id(conn, &compute_call_id)
}

fn local_claim_matches(
    run: &NodeComputeRun,
    owner_user_id: &str,
    node_id: &str,
    model_id: Option<&str>,
) -> bool {
    run.consumer_user_id == owner_user_id
        && run.provider_user_id.as_deref() == Some(owner_user_id)
        && run.node_id == node_id
        && run.model_id.as_deref() == model_id
        && run.feature == "pc_agent_cli_offline_dev"
        && run.usage_mode == "pc_agent_cli"
        && run.route_reason.as_deref() == Some("owner_local_offline")
        && run.billing_source == "own_codex"
        && run.resource_owner_user_id.as_deref() == Some(owner_user_id)
        && run.lease_id.is_none()
        && run.offline_policy == "allow_offline"
        && run.replay_deadline.is_none()
        && run.max_cost_rmb_fen == 0
        && run.allowance_id.is_none()
}

fn normalize_billing_source(value: &str) -> Result<&str> {
    match value.trim() {
        "platform" => Ok("platform"),
        "own_codex" => Ok("own_codex"),
        "shared_codex" => Ok("shared_codex"),
        "user_api_key" => Ok("user_api_key"),
        _ => Err(anyhow!("不支持的 billing_source")),
    }
}

fn normalize_offline_policy(value: &str) -> Result<&str> {
    match value.trim() {
        "allow_offline" => Ok("allow_offline"),
        "require_active_reservation" => Ok("require_active_reservation"),
        "online_only" => Ok("online_only"),
        _ => Err(anyhow!("不支持的 offline_policy")),
    }
}

fn validate_source_policy(billing_source: &str, offline_policy: &str) -> Result<()> {
    let allowed = match billing_source {
        "own_codex" => matches!(offline_policy, "allow_offline" | "online_only"),
        "shared_codex" | "platform" => {
            matches!(offline_policy, "require_active_reservation" | "online_only")
        }
        _ => offline_policy == "online_only",
    };
    if allowed {
        Ok(())
    } else {
        Err(anyhow!(
            "billing_source 与 offline_policy 组合不允许离线执行"
        ))
    }
}

fn required_identifier(field: &str, value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 200 || value.chars().any(char::is_control) {
        return Err(anyhow!("{field} 无效"));
    }
    Ok(value.to_string())
}

fn optional_identifier(field: &str, value: Option<&str>) -> Result<Option<String>> {
    value
        .map(|value| required_identifier(field, value))
        .transpose()
}

fn normalize_replay_deadline(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow!("replay_deadline 必须是 RFC3339 时间"))?;
    Ok(Some(parsed.with_timezone(&Utc).to_rfc3339()))
}

fn replay_deadline_has_passed(value: Option<&str>) -> bool {
    value
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|value| value.with_timezone(&Utc) < Utc::now())
}
