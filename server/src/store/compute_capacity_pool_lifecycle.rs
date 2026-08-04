use anyhow::{anyhow, bail, Result};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::capacity::ComputeCapacityPoolStatus;

use super::{new_id, now, Store};

#[derive(Debug, Clone)]
pub(crate) struct TransitionComputeCapacityPoolStatus {
    pub pool_id: String,
    pub expected_capacity_epoch: i64,
    pub expected_status: ComputeCapacityPoolStatus,
    pub target_status: ComputeCapacityPoolStatus,
    pub reason_code: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityPoolStatusReceipt {
    pub event_id: String,
    pub pool_id: String,
    pub capacity_epoch: i64,
    pub previous_status: String,
    pub target_status: String,
    pub current_status: String,
    pub request_digest: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn transition_compute_capacity_pool_status(
        &self,
        input: TransitionComputeCapacityPoolStatus,
    ) -> Result<ComputeCapacityPoolStatusReceipt> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = transition_compute_capacity_pool_status_on(&tx, &input)?;
        tx.commit()?;
        Ok(receipt)
    }
}

pub(super) fn transition_compute_capacity_pool_status_on(
    conn: &Connection,
    input: &TransitionComputeCapacityPoolStatus,
) -> Result<ComputeCapacityPoolStatusReceipt> {
    validate_transition_input(input)?;
    if let Some(existing) = read_existing_event_on(
        conn,
        input.idempotency_scope.trim(),
        input.idempotency_key.trim(),
    )? {
        validate_replay(input, &existing)?;
        let current_status = current_pool_status_on(conn, &existing.pool_id)?;
        return Ok(ComputeCapacityPoolStatusReceipt {
            event_id: existing.event_id,
            pool_id: existing.pool_id,
            capacity_epoch: existing.capacity_epoch,
            previous_status: existing.previous_status,
            target_status: existing.target_status,
            current_status,
            request_digest: existing.request_digest,
            replayed: true,
        });
    }

    let current = conn
        .query_row(
            "SELECT status, current_capacity_epoch
               FROM compute_capacity_pools WHERE pool_id=?1",
            params![input.pool_id.trim()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("容量池不存在"))?;
    let expected_status = pool_status_value(input.expected_status);
    let target_status = pool_status_value(input.target_status);
    if current.0 != expected_status || current.1 != input.expected_capacity_epoch {
        bail!("容量池状态或 epoch 已变化，拒绝执行旧生命周期请求");
    }
    if !is_allowed_transition(input.expected_status, input.target_status) {
        bail!("容量池生命周期转换不受支持");
    }
    if input.target_status == ComputeCapacityPoolStatus::Active {
        ensure_pool_has_version_on(conn, input.pool_id.trim(), input.expected_capacity_epoch)?;
    }
    if input.target_status == ComputeCapacityPoolStatus::Retired {
        ensure_pool_drained_on(conn, input.pool_id.trim(), input.expected_capacity_epoch)?;
    }

    let event_id = new_id("capacity_pool_event");
    let recorded_at = now();
    conn.execute(
        "INSERT INTO compute_capacity_pool_lifecycle_events (
            event_id, pool_id, capacity_epoch, previous_status, target_status,
            reason_code, subject_kind, subject_id, idempotency_scope,
            idempotency_key, request_digest, occurred_at, recorded_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            event_id,
            input.pool_id.trim(),
            input.expected_capacity_epoch,
            expected_status,
            target_status,
            input.reason_code.trim(),
            input.subject_kind.trim(),
            input.subject_id.trim(),
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
            input.request_digest.trim(),
            input.occurred_at.trim(),
            recorded_at,
        ],
    )?;
    let changed = conn.execute(
        "UPDATE compute_capacity_pools SET status=?1, updated_at=?2
          WHERE pool_id=?3 AND current_capacity_epoch=?4 AND status=?5",
        params![
            target_status,
            recorded_at,
            input.pool_id.trim(),
            input.expected_capacity_epoch,
            expected_status,
        ],
    )?;
    if changed != 1 {
        bail!("容量池状态发生并发变化，生命周期事件未提交");
    }
    Ok(ComputeCapacityPoolStatusReceipt {
        event_id,
        pool_id: input.pool_id.trim().to_string(),
        capacity_epoch: input.expected_capacity_epoch,
        previous_status: expected_status.to_string(),
        target_status: target_status.to_string(),
        current_status: target_status.to_string(),
        request_digest: input.request_digest.trim().to_string(),
        replayed: false,
    })
}

struct ExistingLifecycleEvent {
    event_id: String,
    pool_id: String,
    capacity_epoch: i64,
    previous_status: String,
    target_status: String,
    request_digest: String,
}

fn read_existing_event_on(
    conn: &Connection,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<Option<ExistingLifecycleEvent>> {
    conn.query_row(
        "SELECT event_id, pool_id, capacity_epoch, previous_status,
                target_status, request_digest
           FROM compute_capacity_pool_lifecycle_events
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![idempotency_scope, idempotency_key],
        |row| {
            Ok(ExistingLifecycleEvent {
                event_id: row.get(0)?,
                pool_id: row.get(1)?,
                capacity_epoch: row.get(2)?,
                previous_status: row.get(3)?,
                target_status: row.get(4)?,
                request_digest: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn validate_replay(
    input: &TransitionComputeCapacityPoolStatus,
    existing: &ExistingLifecycleEvent,
) -> Result<()> {
    if existing.pool_id != input.pool_id.trim()
        || existing.capacity_epoch != input.expected_capacity_epoch
        || existing.previous_status != pool_status_value(input.expected_status)
        || existing.target_status != pool_status_value(input.target_status)
        || existing.request_digest != input.request_digest.trim()
    {
        bail!("相同容量池生命周期幂等键不能用于不同请求");
    }
    Ok(())
}

fn current_pool_status_on(conn: &Connection, pool_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT status FROM compute_capacity_pools WHERE pool_id=?1",
        params![pool_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn ensure_pool_has_version_on(conn: &Connection, pool_id: &str, capacity_epoch: i64) -> Result<()> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_pool_versions
              WHERE pool_id=?1 AND capacity_epoch=?2 LIMIT 1",
            params![pool_id, capacity_epoch],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();
    if !exists {
        bail!("容量池没有当前 epoch 的已登记版本，不能激活");
    }
    Ok(())
}

fn ensure_pool_drained_on(conn: &Connection, pool_id: &str, capacity_epoch: i64) -> Result<()> {
    let busy_bucket_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM compute_capacity_buckets
          WHERE pool_id=?1 AND capacity_epoch=?2
            AND (available_units<>0 OR held_units<>0 OR active_units<>0)",
        params![pool_id, capacity_epoch],
        |row| row.get(0),
    )?;
    if busy_bucket_count != 0 {
        bail!("容量池仍有可用、预留或活跃容量，不能退役");
    }
    Ok(())
}

fn validate_transition_input(input: &TransitionComputeCapacityPoolStatus) -> Result<()> {
    for (label, value) in [
        ("容量池 ID", input.pool_id.as_str()),
        ("原因代码", input.reason_code.as_str()),
        ("主体类型", input.subject_kind.as_str()),
        ("主体 ID", input.subject_id.as_str()),
        ("幂等范围", input.idempotency_scope.as_str()),
        ("幂等键", input.idempotency_key.as_str()),
        ("请求摘要", input.request_digest.as_str()),
        ("发生时间", input.occurred_at.as_str()),
    ] {
        if value.trim().is_empty() {
            bail!("{label}不能为空");
        }
    }
    if input.expected_capacity_epoch <= 0 {
        bail!("容量池 expected epoch 必须为正整数");
    }
    if input.expected_status == input.target_status {
        bail!("容量池目标状态不能与当前状态相同");
    }
    let occurred_at = DateTime::parse_from_rfc3339(input.occurred_at.trim())
        .map_err(|_| anyhow!("容量池生命周期发生时间不是 RFC3339"))?;
    if occurred_at.offset().local_minus_utc() != 0 {
        bail!("容量池生命周期发生时间必须使用 UTC 时区");
    }
    Ok(())
}

fn is_allowed_transition(
    previous: ComputeCapacityPoolStatus,
    target: ComputeCapacityPoolStatus,
) -> bool {
    use ComputeCapacityPoolStatus::{Active, Draining, Quarantined, Registering, Retired};
    matches!(
        (previous, target),
        (Registering, Active | Quarantined)
            | (Active, Draining | Quarantined)
            | (Draining, Active | Retired | Quarantined)
            | (Quarantined, Active | Draining)
    )
}

fn pool_status_value(status: ComputeCapacityPoolStatus) -> &'static str {
    match status {
        ComputeCapacityPoolStatus::Registering => "registering",
        ComputeCapacityPoolStatus::Active => "active",
        ComputeCapacityPoolStatus::Draining => "draining",
        ComputeCapacityPoolStatus::Retired => "retired",
        ComputeCapacityPoolStatus::Quarantined => "quarantined",
    }
}
