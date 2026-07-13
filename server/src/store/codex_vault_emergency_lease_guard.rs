//! Transactional authorization boundary for emergency shared Codex leases.

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    codex_vault_dispatch_authorization::require_dispatch_authorization_in_tx,
    codex_vault_emergency::{CodexVaultEmergencyLeaseCreate, CodexVaultEmergencyLeaseRecord},
    common::{clean_optional, new_id, now},
    node_compute_replay::compare_and_bind_node_compute_run_replay_policy_in_tx,
    node_compute_runs::select_run_by_compute_call_id,
    NodeComputeReplayBinding, NodeComputeReplayExpectation, NodeComputeRun, Store,
};

pub(crate) struct CodexVaultEmergencyLeaseRunIssue {
    pub lease: CodexVaultEmergencyLeaseRecord,
    pub run: NodeComputeRun,
    pub superseded_cancel_targets: Vec<(String, String)>,
}

pub(crate) struct CodexVaultEmergencyLeaseIssue {
    pub lease: CodexVaultEmergencyLeaseRecord,
    pub superseded_cancel_targets: Vec<(String, String)>,
}

pub(crate) struct CodexVaultEmergencyLeaseClearIssue {
    pub lease: CodexVaultEmergencyLeaseRecord,
    pub cancel_targets: Vec<(String, String)>,
}

impl Store {
    /// Bind an already-issued emergency lease to a not-yet-dispatched run.
    ///
    /// The lease/grant check and the run CAS share one transaction. Therefore a
    /// concurrent revoke/clear/supersede either sees the bound run and cancels
    /// it, or commits first and makes this bind fail closed.
    pub(crate) fn bind_node_compute_run_to_active_emergency_lease(
        &self,
        compute_call_id: &str,
        binding: NodeComputeReplayBinding<'_>,
    ) -> Result<Option<NodeComputeRun>> {
        let compute_call_id = compute_call_id.trim();
        if compute_call_id.is_empty() {
            bail!("共享 Codex 运行缺少 compute_call_id");
        }
        if binding.billing_source != "shared_codex"
            || binding.offline_policy != "require_active_reservation"
        {
            bail!("共享 Codex 运行的计费来源或离线策略无效");
        }
        let lease_id = binding
            .lease_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("共享 Codex 运行缺少 lease_id"))?;
        let expected_provider_user_id = binding
            .resource_owner_user_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("共享 Codex 运行缺少 provider 身份"))?;
        let expected_deadline = binding
            .replay_deadline
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("共享 Codex 运行缺少授权截止时间"))?;

        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let Some(run) = select_run_by_compute_call_id(&tx, compute_call_id)? else {
            tx.commit()?;
            return Ok(None);
        };
        if run.status != "started" {
            tx.commit()?;
            return Ok(None);
        }
        if run_matches_replay_binding(&run, binding) {
            require_dispatch_authorization_in_tx(
                &tx,
                &run,
                true,
                Some(expected_deadline),
                Some(lease_id),
                Some(expected_provider_user_id),
            )?;
            tx.commit()?;
            return Ok(Some(run));
        }
        require_initial_shared_bind_state(&run)?;
        let expected = NodeComputeReplayExpectation {
            consumer_user_id: &run.consumer_user_id,
            node_id: &run.node_id,
            billing_source: &run.billing_source,
            resource_owner_user_id: run.resource_owner_user_id.as_deref(),
            lease_id: run.lease_id.as_deref(),
            offline_policy: &run.offline_policy,
            updated_at: &run.updated_at,
        };
        let Some(bound) = compare_and_bind_node_compute_run_replay_policy_in_tx(
            &tx,
            compute_call_id,
            expected,
            binding,
        )?
        else {
            tx.commit()?;
            return Ok(None);
        };
        require_dispatch_authorization_in_tx(
            &tx,
            &bound,
            true,
            Some(expected_deadline),
            Some(lease_id),
            Some(expected_provider_user_id),
        )?;
        tx.commit()?;
        Ok(Some(bound))
    }

    /// Revalidate the exact frozen run immediately before every send/retry.
    /// Revocation fences `replay_deadline` to now, so a lost Cancel followed by
    /// reconnect cannot reuse the stale deadline on a fresh WebSocket session.
    pub(crate) fn require_node_compute_run_dispatch_authorization(
        &self,
        compute_call_id: &str,
        node_id: &str,
        requires_cloud_control: bool,
        expected_deadline: Option<&str>,
        expected_lease_id: Option<&str>,
    ) -> Result<NodeComputeRun> {
        let compute_call_id = compute_call_id.trim();
        let node_id = node_id.trim();
        if compute_call_id.is_empty() || node_id.is_empty() {
            bail!("PC CLI 派发身份不完整");
        }
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let run = select_run_by_compute_call_id(&tx, compute_call_id)?
            .ok_or_else(|| anyhow!("PC CLI 派发运行不存在"))?;
        if run.node_id != node_id {
            bail!("PC CLI 派发节点与冻结运行不一致");
        }
        require_dispatch_authorization_in_tx(
            &tx,
            &run,
            requires_cloud_control,
            expected_deadline,
            expected_lease_id,
            run.resource_owner_user_id.as_deref(),
        )?;
        tx.commit()?;
        Ok(run)
    }

    pub fn clear_codex_vault_emergency_lease_for_node(
        &self,
        consumer_user_id: &str,
        consumer_node_id: &str,
        lease_id: Option<&str>,
    ) -> Result<Option<CodexVaultEmergencyLeaseRecord>> {
        Ok(self
            .clear_codex_vault_emergency_lease_for_node_with_cancel_targets(
                consumer_user_id,
                consumer_node_id,
                lease_id,
            )?
            .map(|issue| issue.lease))
    }

    pub(crate) fn clear_codex_vault_emergency_lease_for_node_with_cancel_targets(
        &self,
        consumer_user_id: &str,
        consumer_node_id: &str,
        lease_id: Option<&str>,
    ) -> Result<Option<CodexVaultEmergencyLeaseClearIssue>> {
        let ts = now();
        let clean_lease_id = clean_optional(lease_id);
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let target: Option<String> = if let Some(lease_id) = clean_lease_id {
            tx.query_row(
                "SELECT id
                   FROM codex_vault_emergency_leases
                  WHERE id = ?1
                    AND consumer_user_id = ?2
                    AND consumer_node_id = ?3
                    AND status = 'active'
                    AND cleared_at IS NULL
                  LIMIT 1",
                params![lease_id, consumer_user_id, consumer_node_id],
                |row| row.get(0),
            )
            .optional()?
        } else {
            tx.query_row(
                "SELECT id
                   FROM codex_vault_emergency_leases
                  WHERE consumer_user_id = ?1
                    AND consumer_node_id = ?2
                    AND status = 'active'
                    AND cleared_at IS NULL
                  ORDER BY leased_at DESC, id DESC
                  LIMIT 1",
                params![consumer_user_id, consumer_node_id],
                |row| row.get(0),
            )
            .optional()?
        };
        let Some(target) = target else {
            tx.commit()?;
            return Ok(None);
        };
        let cancel_targets = cancel_targets_for_lease(&tx, &target)?;
        tx.execute(
            "UPDATE node_compute_runs
                SET replay_deadline = ?2,
                    updated_at = ?2
              WHERE lease_id = ?1
                AND (
                    replay_deadline IS NULL
                    OR julianday(replay_deadline) IS NULL
                    OR julianday(replay_deadline) > julianday(?2)
                )",
            params![target, ts],
        )?;
        let changed = tx.execute(
            "UPDATE codex_vault_emergency_leases
                SET status = 'cleared',
                    cleared_at = ?2,
                    updated_at = ?2
              WHERE id = ?1
                AND consumer_user_id = ?3
                AND consumer_node_id = ?4
                AND status = 'active'
                AND cleared_at IS NULL",
            params![target, ts, consumer_user_id, consumer_node_id],
        )?;
        if changed == 0 {
            tx.rollback()?;
            return Ok(None);
        }
        tx.commit()?;
        drop(conn);
        let lease = self
            .get_codex_vault_emergency_lease(&target)?
            .ok_or_else(|| anyhow!("共享租约清除后无法读取"))?;
        Ok(Some(CodexVaultEmergencyLeaseClearIssue {
            lease,
            cancel_targets,
        }))
    }

    /// Revoking a grant invalidates every issued lease in the same transaction.
    /// Any bound run is deadline-fenced so replay cannot keep using the revoked
    /// authorization even if the node has not reconnected yet.
    pub fn revoke_codex_vault_emergency_grant(
        &self,
        grant_id: &str,
        provider_user_id: &str,
    ) -> Result<Option<Vec<(String, String)>>> {
        let ts = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE codex_vault_emergency_grants
                SET status = 'revoked',
                    revoked_at = ?3,
                    updated_at = ?3
              WHERE id = ?1
                AND provider_user_id = ?2
                AND status = 'active'",
            params![grant_id, provider_user_id, ts],
        )?;
        if changed == 0 {
            tx.commit()?;
            return Ok(None);
        }
        let cancel_targets = {
            let mut stmt = tx.prepare(
                "SELECT DISTINCT run.node_id, run.compute_call_id
                   FROM node_compute_runs AS run
                   JOIN codex_vault_emergency_leases AS lease
                     ON lease.id = run.lease_id
                  WHERE lease.grant_id = ?1
                    AND lease.status = 'active'
                    AND lease.cleared_at IS NULL
                    AND (
                        run.status = 'started'
                        OR (
                            run.status = 'verification_pending'
                            AND run.settlement_status = 'dispatch_outcome_unknown'
                        )
                    )",
            )?;
            let rows = stmt.query_map([grant_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .filter_map(|(node_id, compute_call_id)| {
                    compute_call_id
                        .strip_prefix("pc_agent_cli:")
                        .filter(|req_id| !req_id.is_empty())
                        .map(|req_id| (node_id, req_id.to_string()))
                })
                .collect::<Vec<_>>()
        };
        tx.execute(
            "UPDATE node_compute_runs
                SET replay_deadline = ?2,
                    updated_at = ?2
              WHERE lease_id IN (
                    SELECT id
                      FROM codex_vault_emergency_leases
                     WHERE grant_id = ?1
                       AND status = 'active'
                       AND cleared_at IS NULL
                )
                AND (
                    replay_deadline IS NULL
                    OR julianday(replay_deadline) IS NULL
                    OR julianday(replay_deadline) > julianday(?2)
                )",
            params![grant_id, ts],
        )?;
        tx.execute(
            "UPDATE codex_vault_emergency_leases
                SET status = 'cleared',
                    cleared_at = COALESCE(cleared_at, ?2),
                    updated_at = ?2
              WHERE grant_id = ?1
                AND status = 'active'
                AND cleared_at IS NULL",
            params![grant_id, ts],
        )?;
        tx.commit()?;
        Ok(Some(cancel_targets))
    }

    /// Creates the sole active shared credential lease for a consumer/node.
    /// Existing active rows are superseded in the same transaction, so clearing
    /// the new lease can never resurrect an older credential.
    pub fn create_codex_vault_emergency_lease(
        &self,
        input: CodexVaultEmergencyLeaseCreate<'_>,
    ) -> Result<CodexVaultEmergencyLeaseRecord> {
        Ok(self
            .create_codex_vault_emergency_lease_with_superseded_runs(input)?
            .lease)
    }

    pub(crate) fn create_codex_vault_emergency_lease_with_superseded_runs(
        &self,
        input: CodexVaultEmergencyLeaseCreate<'_>,
    ) -> Result<CodexVaultEmergencyLeaseIssue> {
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let leased_at = Utc::now();
        let grant_policy = require_active_grant(&tx, &input, &leased_at)?;
        let lease = PendingEmergencyLease::new(leased_at, grant_policy);
        let superseded_cancel_targets =
            supersede_active_node_leases(&tx, &input, &lease.leased_at)?;
        insert_lease(&tx, &input, &lease)?;
        tx.commit()?;
        drop(conn);
        let lease = self
            .get_codex_vault_emergency_lease(&lease.id)?
            .ok_or_else(|| anyhow!("共享租约保存后无法读取"))?;
        Ok(CodexVaultEmergencyLeaseIssue {
            lease,
            superseded_cancel_targets,
        })
    }

    /// Atomically creates a lease and upgrades the exact observed own-Codex run
    /// to that lease. A stale/concurrent request returns `None` and rolls back
    /// both the new row and any superseding update.
    pub(crate) fn create_codex_vault_emergency_lease_for_run(
        &self,
        input: CodexVaultEmergencyLeaseCreate<'_>,
        expected_run: &NodeComputeRun,
        reservation_id: &str,
    ) -> Result<Option<CodexVaultEmergencyLeaseRunIssue>> {
        validate_expected_own_run(&input, expected_run)?;
        let reservation_id = required_value("reservation_id", reservation_id)?;
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let leased_at = Utc::now();
        let grant_policy = require_active_grant(&tx, &input, &leased_at)?;
        let lease = PendingEmergencyLease::new(leased_at, grant_policy);
        let Some(reserved_fen) = active_reservation(
            &tx,
            reservation_id,
            &expected_run.consumer_user_id,
            &expected_run.compute_call_id,
            &lease.leased_at,
        )?
        else {
            tx.rollback()?;
            bail!("共享 Codex 运行没有有效的冻结计费预留");
        };

        let superseded_cancel_targets =
            supersede_active_node_leases(&tx, &input, &lease.leased_at)?;
        insert_lease(&tx, &input, &lease)?;
        let rebound = compare_and_bind_node_compute_run_replay_policy_in_tx(
            &tx,
            &expected_run.compute_call_id,
            NodeComputeReplayExpectation {
                consumer_user_id: &expected_run.consumer_user_id,
                node_id: &expected_run.node_id,
                billing_source: &expected_run.billing_source,
                resource_owner_user_id: expected_run.resource_owner_user_id.as_deref(),
                lease_id: expected_run.lease_id.as_deref(),
                offline_policy: &expected_run.offline_policy,
                updated_at: &expected_run.updated_at,
            },
            NodeComputeReplayBinding {
                billing_source: "shared_codex",
                resource_owner_user_id: Some(input.provider_user_id),
                lease_id: Some(&lease.id),
                offline_policy: "require_active_reservation",
                replay_deadline: Some(&lease.expires_at),
                max_cost_rmb_fen: reserved_fen,
                allowance_id: Some(reservation_id),
            },
        )?;
        let Some(rebound) = rebound else {
            tx.rollback()?;
            return Ok(None);
        };
        if rebound.lease_id.as_deref() != Some(lease.id.as_str())
            || rebound.resource_owner_user_id.as_deref() != Some(input.provider_user_id)
            || rebound.allowance_id.as_deref() != Some(reservation_id)
        {
            tx.rollback()?;
            bail!("共享 Codex 租约与计算运行没有原子绑定");
        }
        let held = tx.execute(
            "UPDATE billing_reservations
                SET status = 'dispatch_hold', updated_at = ?2
              WHERE id = ?1 AND status = 'reserved'",
            params![reservation_id, lease.leased_at],
        )?;
        if held != 1 {
            tx.rollback()?;
            bail!("共享 Codex 租约未能原子转为 durable dispatch hold");
        }
        tx.commit()?;
        drop(conn);
        let lease = self
            .get_codex_vault_emergency_lease(&lease.id)?
            .ok_or_else(|| anyhow!("共享租约保存后无法读取"))?;
        Ok(Some(CodexVaultEmergencyLeaseRunIssue {
            lease,
            run: rebound,
            superseded_cancel_targets,
        }))
    }
}

struct PendingEmergencyLease {
    id: String,
    leased_at: String,
    expires_at: String,
}

struct ActiveGrantPolicy {
    max_lease_seconds: i64,
    expires_at: Option<DateTime<Utc>>,
}

impl PendingEmergencyLease {
    fn new(leased_at: DateTime<Utc>, policy: ActiveGrantPolicy) -> Self {
        let requested_expires_at = leased_at + Duration::seconds(policy.max_lease_seconds);
        let expires_at = policy
            .expires_at
            .map(|grant_expires_at| grant_expires_at.min(requested_expires_at))
            .unwrap_or(requested_expires_at);
        Self {
            id: new_id("cvel"),
            leased_at: leased_at.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
        }
    }
}

fn require_active_grant(
    conn: &Connection,
    input: &CodexVaultEmergencyLeaseCreate<'_>,
    active_at: &DateTime<Utc>,
) -> Result<ActiveGrantPolicy> {
    let policy = conn
        .query_row(
            "SELECT max_lease_seconds, expires_at
               FROM codex_vault_emergency_grants
              WHERE id = ?1
                AND provider_user_id = ?2
                AND consumer_user_id = ?3
                AND status = 'active'",
            params![
                input.grant_id,
                input.provider_user_id,
                input.consumer_user_id,
            ],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    let Some((max_lease_seconds, raw_expires_at)) = policy else {
        bail!("共享授权已撤销、过期或与 provider/consumer 不匹配");
    };
    if !(60..=7200).contains(&max_lease_seconds) {
        bail!("共享授权 max_lease_seconds 无效");
    }
    let expires_at = raw_expires_at
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|_| anyhow!("共享授权 expires_at 不是有效 RFC3339 时间"))
        })
        .transpose()?;
    if expires_at
        .as_ref()
        .is_some_and(|expires_at| expires_at <= active_at)
    {
        bail!("共享授权已撤销、过期或与 provider/consumer 不匹配");
    }
    Ok(ActiveGrantPolicy {
        max_lease_seconds,
        expires_at,
    })
}

fn supersede_active_node_leases(
    conn: &Connection,
    input: &CodexVaultEmergencyLeaseCreate<'_>,
    ts: &str,
) -> Result<Vec<(String, String)>> {
    let cancel_targets = {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT run.node_id, run.compute_call_id
               FROM node_compute_runs AS run
               JOIN codex_vault_emergency_leases AS lease
                 ON lease.id = run.lease_id
              WHERE lease.consumer_user_id = ?1
                AND lease.consumer_node_id = ?2
                AND lease.status = 'active'
                AND lease.cleared_at IS NULL
                AND (
                    run.status = 'started'
                    OR (
                        run.status = 'verification_pending'
                        AND run.settlement_status = 'dispatch_outcome_unknown'
                    )
                )",
        )?;
        let rows = stmt.query_map(
            params![input.consumer_user_id, input.consumer_node_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|(node_id, compute_call_id)| {
                compute_call_id
                    .strip_prefix("pc_agent_cli:")
                    .filter(|req_id| !req_id.is_empty())
                    .map(|req_id| (node_id, req_id.to_string()))
            })
            .collect::<Vec<_>>()
    };
    conn.execute(
        "UPDATE node_compute_runs
            SET replay_deadline = ?3,
                updated_at = ?3
          WHERE lease_id IN (
                SELECT id
                  FROM codex_vault_emergency_leases
                 WHERE consumer_user_id = ?1
                   AND consumer_node_id = ?2
                   AND status = 'active'
                   AND cleared_at IS NULL
            )
            AND (
                replay_deadline IS NULL
                OR julianday(replay_deadline) IS NULL
                OR julianday(replay_deadline) > julianday(?3)
            )",
        params![input.consumer_user_id, input.consumer_node_id, ts],
    )?;
    conn.execute(
        "UPDATE codex_vault_emergency_leases
            SET status = 'cleared',
                cleared_at = COALESCE(cleared_at, ?3),
                updated_at = ?3
          WHERE consumer_user_id = ?1
            AND consumer_node_id = ?2
            AND status = 'active'
            AND cleared_at IS NULL",
        params![input.consumer_user_id, input.consumer_node_id, ts],
    )?;
    Ok(cancel_targets)
}

fn cancel_targets_for_lease(conn: &Connection, lease_id: &str) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT node_id, compute_call_id
           FROM node_compute_runs
          WHERE lease_id = ?1
            AND (
                status = 'started'
                OR (
                    status = 'verification_pending'
                    AND settlement_status = 'dispatch_outcome_unknown'
                )
            )",
    )?;
    let rows = stmt.query_map([lease_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|(node_id, compute_call_id)| {
            compute_call_id
                .strip_prefix("pc_agent_cli:")
                .filter(|req_id| !req_id.is_empty())
                .map(|req_id| (node_id, req_id.to_string()))
        })
        .collect())
}

fn insert_lease(
    conn: &Connection,
    input: &CodexVaultEmergencyLeaseCreate<'_>,
    lease: &PendingEmergencyLease,
) -> Result<()> {
    conn.execute(
        "INSERT INTO codex_vault_emergency_leases
         (id, grant_id, provider_user_id, consumer_user_id, consumer_node_id,
          provider_slot_id, account_hint_hash, purpose, failure_reason, billing_source,
          status, leased_at, expires_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'shared_codex',
                 'active', ?10, ?11, ?10, ?10)",
        params![
            lease.id,
            input.grant_id,
            input.provider_user_id,
            input.consumer_user_id,
            input.consumer_node_id,
            input.provider_slot_id,
            clean_optional(input.account_hint_hash),
            clean_optional(input.purpose),
            clean_optional(input.failure_reason),
            lease.leased_at,
            lease.expires_at,
        ],
    )?;
    Ok(())
}

fn validate_expected_own_run(
    input: &CodexVaultEmergencyLeaseCreate<'_>,
    run: &NodeComputeRun,
) -> Result<()> {
    if run.consumer_user_id != input.consumer_user_id
        || run.provider_user_id.as_deref() != Some(input.consumer_user_id)
        || run.node_id != input.consumer_node_id
        || run.usage_mode != "pc_agent_cli"
        || run.status != "started"
        || run.billing_source != "own_codex"
        || run.resource_owner_user_id.as_deref() != Some(input.consumer_user_id)
        || run.offline_policy != "allow_offline"
        || run.lease_id.is_some()
        || run.replay_deadline.is_some()
        || run.max_cost_rmb_fen != 0
        || run.allowance_id.is_some()
    {
        bail!("共享 Codex 租约只能 CAS 绑定到同用户、同节点且未部分接管的自有 Codex 运行");
    }
    Ok(())
}

fn active_reservation(
    conn: &Connection,
    reservation_id: &str,
    user_id: &str,
    compute_call_id: &str,
    active_at: &str,
) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT reserved_fen
           FROM billing_reservations
          WHERE id = ?1
            AND user_id = ?2
            AND compute_call_id = ?3
            AND (
              status = 'dispatch_hold'
              OR (status = 'reserved' AND (expires_at IS NULL OR expires_at >= ?4))
            )",
        params![reservation_id, user_id, compute_call_id, active_at],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn required_value<'a>(field: &str, value: &'a str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() || value.len() > 200 || value.chars().any(char::is_control) {
        bail!("{field} 无效");
    }
    Ok(value)
}

fn require_initial_shared_bind_state(run: &NodeComputeRun) -> Result<()> {
    if run.billing_source != "platform"
        || run.resource_owner_user_id.is_some()
        || run.lease_id.is_some()
        || run.offline_policy != "online_only"
        || run.replay_deadline.is_some()
        || run.max_cost_rmb_fen != 0
        || run.allowance_id.is_some()
    {
        bail!("共享 Codex 租约只能绑定到尚未冻结派发策略的计算运行");
    }
    Ok(())
}

fn run_matches_replay_binding(run: &NodeComputeRun, binding: NodeComputeReplayBinding<'_>) -> bool {
    run.billing_source == binding.billing_source.trim()
        && same_optional(
            run.resource_owner_user_id.as_deref(),
            binding.resource_owner_user_id,
        )
        && same_optional(run.lease_id.as_deref(), binding.lease_id)
        && run.offline_policy == binding.offline_policy.trim()
        && same_optional(run.replay_deadline.as_deref(), binding.replay_deadline)
        && run.max_cost_rmb_fen == binding.max_cost_rmb_fen.max(0)
        && same_optional(run.allowance_id.as_deref(), binding.allowance_id)
}

fn same_optional(left: Option<&str>, right: Option<&str>) -> bool {
    left.map(str::trim).filter(|value| !value.is_empty())
        == right.map(str::trim).filter(|value| !value.is_empty())
}
