use anyhow::{anyhow, bail, Result};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use super::{new_id, now, Store};

#[derive(Debug, Clone)]
pub(crate) struct RolloverComputeCapacityPoolEpoch {
    pub pool_id: String,
    pub expected_capacity_epoch: i64,
    pub reason_code: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityPoolEpochReceipt {
    pub event_id: String,
    pub pool_id: String,
    pub previous_capacity_epoch: i64,
    pub target_capacity_epoch: i64,
    pub target_status: String,
    pub current_capacity_epoch: i64,
    pub current_status: String,
    pub request_digest: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn rollover_compute_capacity_pool_epoch(
        &self,
        input: RolloverComputeCapacityPoolEpoch,
    ) -> Result<ComputeCapacityPoolEpochReceipt> {
        validate_rollover_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = read_existing_epoch_event_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            validate_replay(&input, &existing)?;
            let current = current_pool_epoch_on(&tx, &existing.pool_id)?;
            tx.commit()?;
            return Ok(ComputeCapacityPoolEpochReceipt {
                event_id: existing.event_id,
                pool_id: existing.pool_id,
                previous_capacity_epoch: existing.previous_capacity_epoch,
                target_capacity_epoch: existing.target_capacity_epoch,
                target_status: "registering".to_string(),
                current_capacity_epoch: current.1,
                current_status: current.0,
                request_digest: existing.request_digest,
                replayed: true,
            });
        }

        let current = current_pool_epoch_on(&tx, input.pool_id.trim())?;
        if current.0 != "retired" || current.1 != input.expected_capacity_epoch {
            bail!("容量池必须处于预期 retired epoch 才能轮换");
        }
        ensure_epoch_drained_on(&tx, input.pool_id.trim(), input.expected_capacity_epoch)?;
        let target_capacity_epoch = input
            .expected_capacity_epoch
            .checked_add(1)
            .ok_or_else(|| anyhow!("容量池 epoch 溢出"))?;
        ensure_target_epoch_unused_on(&tx, input.pool_id.trim(), target_capacity_epoch)?;

        let event_id = new_id("capacity_epoch_event");
        let recorded_at = now();
        tx.execute(
            "INSERT INTO compute_capacity_pool_epoch_events (
                event_id, pool_id, previous_capacity_epoch, target_capacity_epoch,
                previous_status, target_status, reason_code, subject_kind,
                subject_id, idempotency_scope, idempotency_key, request_digest,
                occurred_at, recorded_at
             ) VALUES (
                ?1, ?2, ?3, ?4, 'retired', 'registering', ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12
             )",
            params![
                event_id,
                input.pool_id.trim(),
                input.expected_capacity_epoch,
                target_capacity_epoch,
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
        let changed = tx.execute(
            "UPDATE compute_capacity_pools SET
                current_capacity_epoch=?1, status='registering', updated_at=?2
              WHERE pool_id=?3 AND current_capacity_epoch=?4 AND status='retired'",
            params![
                target_capacity_epoch,
                recorded_at,
                input.pool_id.trim(),
                input.expected_capacity_epoch,
            ],
        )?;
        if changed != 1 {
            bail!("容量池 epoch 发生并发变化，轮换事件未提交");
        }
        tx.commit()?;
        Ok(ComputeCapacityPoolEpochReceipt {
            event_id,
            pool_id: input.pool_id.trim().to_string(),
            previous_capacity_epoch: input.expected_capacity_epoch,
            target_capacity_epoch,
            target_status: "registering".to_string(),
            current_capacity_epoch: target_capacity_epoch,
            current_status: "registering".to_string(),
            request_digest: input.request_digest.trim().to_string(),
            replayed: false,
        })
    }
}

struct ExistingEpochEvent {
    event_id: String,
    pool_id: String,
    previous_capacity_epoch: i64,
    target_capacity_epoch: i64,
    request_digest: String,
}

fn read_existing_epoch_event_on(
    conn: &Connection,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<Option<ExistingEpochEvent>> {
    conn.query_row(
        "SELECT event_id, pool_id, previous_capacity_epoch,
                target_capacity_epoch, request_digest
           FROM compute_capacity_pool_epoch_events
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![idempotency_scope, idempotency_key],
        |row| {
            Ok(ExistingEpochEvent {
                event_id: row.get(0)?,
                pool_id: row.get(1)?,
                previous_capacity_epoch: row.get(2)?,
                target_capacity_epoch: row.get(3)?,
                request_digest: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn validate_replay(
    input: &RolloverComputeCapacityPoolEpoch,
    existing: &ExistingEpochEvent,
) -> Result<()> {
    if existing.pool_id != input.pool_id.trim()
        || existing.previous_capacity_epoch != input.expected_capacity_epoch
        || existing.target_capacity_epoch != input.expected_capacity_epoch.saturating_add(1)
        || existing.request_digest != input.request_digest.trim()
    {
        bail!("相同容量池 epoch 幂等键不能用于不同轮换请求");
    }
    Ok(())
}

fn current_pool_epoch_on(conn: &Connection, pool_id: &str) -> Result<(String, i64)> {
    conn.query_row(
        "SELECT status, current_capacity_epoch
           FROM compute_capacity_pools WHERE pool_id=?1",
        params![pool_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| anyhow!("容量池不存在"))
}

fn ensure_epoch_drained_on(conn: &Connection, pool_id: &str, capacity_epoch: i64) -> Result<()> {
    let busy_bucket_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM compute_capacity_buckets
          WHERE pool_id=?1 AND capacity_epoch=?2
            AND (available_units<>0 OR held_units<>0 OR active_units<>0)",
        params![pool_id, capacity_epoch],
        |row| row.get(0),
    )?;
    if busy_bucket_count != 0 {
        bail!("容量池旧 epoch 尚未排空，不能轮换");
    }
    Ok(())
}

fn ensure_target_epoch_unused_on(
    conn: &Connection,
    pool_id: &str,
    target_capacity_epoch: i64,
) -> Result<()> {
    let fact_count: i64 = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM compute_capacity_pool_versions
              WHERE pool_id=?1 AND capacity_epoch=?2)
          + (SELECT COUNT(*) FROM compute_capacity_buckets
              WHERE pool_id=?1 AND capacity_epoch=?2)
          + (SELECT COUNT(*) FROM compute_capacity_claims
              WHERE pool_id=?1 AND capacity_epoch=?2)
          + (SELECT COUNT(*) FROM compute_capacity_ledger_transactions
              WHERE pool_id=?1 AND capacity_epoch=?2)",
        params![pool_id, target_capacity_epoch],
        |row| row.get(0),
    )?;
    if fact_count != 0 {
        bail!("容量池目标 epoch 已有版本、bucket、Claim 或账本事实，拒绝复用");
    }
    Ok(())
}

fn validate_rollover_input(input: &RolloverComputeCapacityPoolEpoch) -> Result<()> {
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
    if input.expected_capacity_epoch <= 0 || input.expected_capacity_epoch == i64::MAX {
        bail!("容量池 expected epoch 无效或无法递增");
    }
    let occurred_at = DateTime::parse_from_rfc3339(input.occurred_at.trim())
        .map_err(|_| anyhow!("容量池 epoch 轮换发生时间不是 RFC3339"))?;
    if occurred_at.offset().local_minus_utc() != 0 {
        bail!("容量池 epoch 轮换发生时间必须使用 UTC 时区");
    }
    Ok(())
}
