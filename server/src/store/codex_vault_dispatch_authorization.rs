//! Exact, fail-closed authorization checks immediately before PC-node dispatch.

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::NodeComputeRun;

pub(super) fn require_dispatch_authorization_in_tx(
    conn: &Connection,
    run: &NodeComputeRun,
    requires_cloud_control: bool,
    expected_deadline: Option<&str>,
    expected_lease_id: Option<&str>,
    expected_provider_user_id: Option<&str>,
) -> Result<()> {
    if run.status != "started" {
        bail!("PC CLI 派发运行已结束");
    }
    if normalized(run.lease_id.as_deref()) != normalized(expected_lease_id) {
        bail!("PC CLI 派发租约与冻结运行不一致");
    }
    if !requires_cloud_control {
        return require_offline_owner_run(run, expected_deadline, expected_provider_user_id);
    }
    require_cloud_controlled_run(
        conn,
        run,
        expected_deadline,
        expected_lease_id,
        expected_provider_user_id,
    )
}

fn require_offline_owner_run(
    run: &NodeComputeRun,
    expected_deadline: Option<&str>,
    expected_provider_user_id: Option<&str>,
) -> Result<()> {
    if run.billing_source != "own_codex"
        || run.offline_policy != "allow_offline"
        || run.lease_id.is_some()
        || run.replay_deadline.is_some()
        || normalized(expected_deadline).is_some()
        || run.max_cost_rmb_fen != 0
        || run.allowance_id.is_some()
        || normalized(run.resource_owner_user_id.as_deref())
            != normalized(expected_provider_user_id)
        || normalized(run.resource_owner_user_id.as_deref()) != Some(run.consumer_user_id.as_str())
    {
        bail!("本机自有 Codex 运行的离线授权状态无效");
    }
    Ok(())
}

fn require_cloud_controlled_run(
    conn: &Connection,
    run: &NodeComputeRun,
    expected_deadline: Option<&str>,
    expected_lease_id: Option<&str>,
    expected_provider_user_id: Option<&str>,
) -> Result<()> {
    if !matches!(run.billing_source.as_str(), "platform" | "shared_codex")
        || run.offline_policy != "require_active_reservation"
    {
        bail!("云端受控 PC CLI 运行的计费来源或离线策略无效");
    }
    let deadline =
        parse_required_future("PC CLI 派发授权截止时间", run.replay_deadline.as_deref())?;
    let expected_deadline = parse_required("PC CLI 请求授权截止时间", expected_deadline)?;
    if deadline != expected_deadline {
        bail!("PC CLI 请求授权截止时间与冻结运行不一致");
    }
    let allowance_id = required("PC CLI 派发 allowance_id", run.allowance_id.as_deref())?;
    let (reserved_fen, reservation_expiry): (i64, Option<String>) = conn
        .query_row(
            "SELECT reserved_fen, expires_at
               FROM billing_reservations
              WHERE id = ?1
                AND user_id = ?2
                AND compute_call_id = ?3
                AND status = 'dispatch_hold'",
            params![allowance_id, run.consumer_user_id, run.compute_call_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("PC CLI 派发缺少精确的 durable dispatch hold"))?;
    if reserved_fen <= 0 || reserved_fen != run.max_cost_rmb_fen {
        bail!("PC CLI 派发预留金额与冻结运行不一致");
    }
    let reservation_expiry =
        parse_required_future("PC CLI 派发预留截止时间", reservation_expiry.as_deref())?;
    if deadline > reservation_expiry {
        bail!("PC CLI 派发授权超过计费预留截止时间");
    }

    match normalized(expected_lease_id) {
        Some(lease_id) => {
            require_active_shared_lease(conn, run, lease_id, expected_provider_user_id, deadline)
        }
        None => {
            if run.billing_source != "platform"
                || run.lease_id.is_some()
                || normalized(run.resource_owner_user_id.as_deref())
                    != normalized(expected_provider_user_id)
            {
                bail!("平台 PC CLI 派发携带了无效的共享租约身份");
            }
            Ok(())
        }
    }
}

fn require_active_shared_lease(
    conn: &Connection,
    run: &NodeComputeRun,
    lease_id: &str,
    expected_provider_user_id: Option<&str>,
    deadline: DateTime<Utc>,
) -> Result<()> {
    if run.billing_source != "shared_codex" {
        bail!("共享租约只能绑定 shared_codex 运行");
    }
    let provider_user_id = required(
        "共享 Codex provider 身份",
        run.resource_owner_user_id.as_deref(),
    )?;
    if Some(provider_user_id) != normalized(expected_provider_user_id) {
        bail!("共享 Codex provider 身份与冻结运行不一致");
    }
    let active: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT lease.expires_at, grant.expires_at
               FROM codex_vault_emergency_leases AS lease
               JOIN codex_vault_emergency_grants AS grant
                 ON grant.id = lease.grant_id
              WHERE lease.id = ?1
                AND lease.provider_user_id = ?2
                AND lease.consumer_user_id = ?3
                AND lease.consumer_node_id = ?4
                AND lease.billing_source = 'shared_codex'
                AND lease.status = 'active'
                AND lease.cleared_at IS NULL
                AND grant.provider_user_id = lease.provider_user_id
                AND grant.consumer_user_id = lease.consumer_user_id
                AND grant.status = 'active'
                AND grant.revoked_at IS NULL",
            params![
                lease_id,
                provider_user_id,
                run.consumer_user_id,
                run.node_id
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (lease_expiry, grant_expiry) =
        active.ok_or_else(|| anyhow!("共享 Codex 租约或授权已撤销、清除或身份不匹配"))?;
    let lease_expiry = parse_future("共享 Codex 租约截止时间", &lease_expiry)?;
    if deadline > lease_expiry {
        bail!("PC CLI 派发授权超过共享租约截止时间");
    }
    if let Some(grant_expiry) = grant_expiry {
        let grant_expiry = parse_future("共享 Codex 授权截止时间", &grant_expiry)?;
        if deadline > grant_expiry {
            bail!("PC CLI 派发授权超过共享授权截止时间");
        }
    }
    Ok(())
}

fn normalized(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn required<'a>(field: &str, value: Option<&'a str>) -> Result<&'a str> {
    normalized(value).ok_or_else(|| anyhow!("{field}缺失"))
}

fn parse_required(field: &str, value: Option<&str>) -> Result<DateTime<Utc>> {
    parse(field, required(field, value)?)
}

fn parse_required_future(field: &str, value: Option<&str>) -> Result<DateTime<Utc>> {
    parse_future(field, required(field, value)?)
}

fn parse_future(field: &str, value: &str) -> Result<DateTime<Utc>> {
    let value = parse(field, value)?;
    if value <= Utc::now() {
        bail!("{field}已过期");
    }
    Ok(value)
}

fn parse(field: &str, value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("{field}不是有效 RFC3339 时间"))
        .map(|value| value.with_timezone(&Utc))
}
