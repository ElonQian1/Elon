use anyhow::{anyhow, bail, Context, Result};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::Value;

use crate::compute_federation::{
    capacity::{
        ComputeCapacityBucket, ComputeCapacityBucketBalance, ComputeCapacityBucketStatus,
        ComputeCapacityMeterPolicy, ComputeCapacityPool, ComputeCapacityPoolStatus,
        COMPUTE_CAPACITY_BUCKET_SCHEMA, COMPUTE_CAPACITY_POOL_SCHEMA,
    },
    market::ComputeDeliveryWindow,
};

use super::{
    compute_capacity_pool_guards::{
        ensure_pool_operation_allowed_on, ComputeCapacityPoolOperation,
    },
    compute_capacity_rows::{meter_mode_value, stored_bucket_on},
    now, Store,
};

impl Store {
    pub(crate) fn register_compute_capacity_pool(
        &self,
        pool: &ComputeCapacityPool,
        resource_profile: &Value,
    ) -> Result<ComputeCapacityPool> {
        validate_pool(pool)?;
        let profile_json = serde_json::to_string(&serde_json::json!({
            "digest": pool.resource_profile_digest,
            "profile": resource_profile,
        }))?;
        let meters_json = serde_json::to_string(&pool.meter_policies)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let existing_pool = tx
            .query_row(
                "SELECT provider_id, resource_scope_digest, current_capacity_epoch, status
                   FROM compute_capacity_pools WHERE pool_id=?1",
                params![pool.binding.pool_id.trim()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        if let Some((provider_id, resource_scope_digest, capacity_epoch, status)) = existing_pool {
            if provider_id != pool.provider_id
                || resource_scope_digest != pool.resource_scope_digest
                || capacity_epoch != pool.binding.capacity_epoch
            {
                bail!("容量池稳定身份、资源范围或 epoch 不能原地改变");
            }
            if status != pool_status_value(pool.status) {
                bail!("容量池状态必须通过独立生命周期操作变更");
            }
        } else {
            if pool.status != ComputeCapacityPoolStatus::Registering {
                bail!("新容量池必须先以 registering 状态创建");
            }
            let conflicting_pool_id = tx
                .query_row(
                    "SELECT pool_id FROM compute_capacity_pools
                      WHERE provider_id=?1 AND resource_scope_digest=?2",
                    params![pool.provider_id.trim(), pool.resource_scope_digest.trim()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            if let Some(conflicting_pool_id) = conflicting_pool_id {
                bail!("相同提供者资源范围已绑定容量池 {conflicting_pool_id}");
            }
            tx.execute(
                "INSERT INTO compute_capacity_pools (
                    pool_id, provider_id, resource_scope_digest, status,
                    current_capacity_epoch, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![
                    pool.binding.pool_id.trim(),
                    pool.provider_id.trim(),
                    pool.resource_scope_digest.trim(),
                    pool_status_value(pool.status),
                    pool.binding.capacity_epoch,
                    pool.created_at.trim(),
                ],
            )?;
        }

        let existing_version = tx
            .query_row(
                "SELECT pool_digest, resource_profile_json, COALESCE(region, ''),
                        supported_meters_json
                   FROM compute_capacity_pool_versions
                  WHERE pool_id=?1 AND capacity_epoch=?2 AND pool_revision=?3",
                params![
                    pool.binding.pool_id.trim(),
                    pool.binding.capacity_epoch,
                    pool.binding.pool_revision,
                ],
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
        if let Some((digest, profile, region, meters)) = existing_version {
            if digest != pool.binding.pool_digest
                || profile != profile_json
                || region != pool.region_or_data_zone
                || meters != meters_json
            {
                bail!("相同容量池版本不能绑定不同资源合同");
            }
            tx.commit()?;
            return Ok(pool.clone());
        }
        if !matches!(
            pool.status,
            ComputeCapacityPoolStatus::Registering | ComputeCapacityPoolStatus::Active
        ) {
            bail!("容量池当前状态不允许登记新版本");
        }

        let latest_revision: i64 = tx.query_row(
            "SELECT COALESCE(MAX(pool_revision), 0)
               FROM compute_capacity_pool_versions
              WHERE pool_id=?1 AND capacity_epoch=?2",
            params![pool.binding.pool_id.trim(), pool.binding.capacity_epoch],
            |row| row.get(0),
        )?;
        if pool.binding.pool_revision != latest_revision + 1 {
            bail!("容量池新版本必须连续递增，当前最新版本为 {latest_revision}");
        }
        tx.execute(
            "INSERT INTO compute_capacity_pool_versions (
                pool_id, capacity_epoch, pool_revision, pool_digest,
                resource_profile_json, region, supported_meters_json,
                created_at, retired_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
            params![
                pool.binding.pool_id.trim(),
                pool.binding.capacity_epoch,
                pool.binding.pool_revision,
                pool.binding.pool_digest.trim(),
                profile_json,
                pool.region_or_data_zone.trim(),
                meters_json,
                pool.created_at.trim(),
            ],
        )?;
        tx.execute(
            "UPDATE compute_capacity_pools SET status=?1, updated_at=?2 WHERE pool_id=?3",
            params![
                pool_status_value(pool.status),
                now(),
                pool.binding.pool_id.trim()
            ],
        )?;
        tx.commit()?;
        Ok(pool.clone())
    }

    pub(crate) fn create_compute_capacity_bucket(
        &self,
        bucket: &ComputeCapacityBucket,
        window: &ComputeDeliveryWindow,
    ) -> Result<ComputeCapacityBucketBalance> {
        validate_bucket(bucket, window)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_pool_binding(&tx, &bucket.binding.pool)?;
        ensure_meter_policy(&tx, bucket)?;

        if let Some(existing) = stored_bucket_on(&tx, &bucket.binding.bucket_id)? {
            if existing.balance.binding != bucket.binding
                || existing.starts_at != window.starts_at_utc
                || existing.ends_at != window.ends_at_utc
            {
                bail!("相同容量 bucket ID 不能绑定不同窗口或资源合同");
            }
            tx.commit()?;
            return Ok(existing.balance);
        }
        ensure_pool_operation_allowed_on(
            &tx,
            &bucket.binding.pool,
            ComputeCapacityPoolOperation::ConfigureBucket,
        )?;

        let existing_window = tx
            .query_row(
                "SELECT delivery_window_digest, delivery_window_starts_at,
                        delivery_window_ends_at
                   FROM compute_capacity_buckets
                  WHERE pool_id=?1 AND capacity_epoch=?2 AND delivery_window_id=?3
                  LIMIT 1",
                params![
                    bucket.binding.pool.pool_id.trim(),
                    bucket.binding.pool.capacity_epoch,
                    bucket.binding.delivery_window.window_id.trim(),
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        if let Some((window_digest, starts_at, ends_at)) = existing_window {
            if window_digest != bucket.binding.delivery_window.window_digest
                || starts_at != window.starts_at_utc
                || ends_at != window.ends_at_utc
            {
                bail!("同一容量池 epoch 的交付窗口 ID 不能绑定不同时间或摘要");
            }
        }

        let overlap = tx
            .query_row(
                "SELECT bucket_id FROM compute_capacity_buckets
                  WHERE pool_id=?1 AND capacity_epoch=?2 AND meter=?3
                    AND status <> 'retired'
                    AND delivery_window_starts_at < ?4
                    AND delivery_window_ends_at > ?5
                  LIMIT 1",
                params![
                    bucket.binding.pool.pool_id.trim(),
                    bucket.binding.pool.capacity_epoch,
                    bucket.binding.meter.trim(),
                    window.ends_at_utc.trim(),
                    window.starts_at_utc.trim(),
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(overlap) = overlap {
            bail!("容量 bucket 窗口与现有 bucket {overlap} 重叠");
        }
        tx.execute(
            "INSERT INTO compute_capacity_buckets (
                bucket_id, bucket_digest, pool_id, capacity_epoch, pool_revision,
                delivery_window_id, delivery_window_digest,
                delivery_window_starts_at, delivery_window_ends_at,
                meter, meter_mode, quantum_units, meter_policy_digest, status,
                issued_units, available_units, held_units, active_units,
                consumed_units, retired_units, balance_revision,
                through_ledger_sequence, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                'open', 0, 0, 0, 0, 0, 0, 0, NULL, ?14, ?14
             )",
            params![
                bucket.binding.bucket_id.trim(),
                bucket.binding.bucket_digest.trim(),
                bucket.binding.pool.pool_id.trim(),
                bucket.binding.pool.capacity_epoch,
                bucket.binding.pool.pool_revision,
                bucket.binding.delivery_window.window_id.trim(),
                bucket.binding.delivery_window.window_digest.trim(),
                window.starts_at_utc.trim(),
                window.ends_at_utc.trim(),
                bucket.binding.meter.trim(),
                meter_mode_value(bucket.binding.meter_mode),
                bucket.binding.quantum_units,
                bucket.binding.meter_policy_digest.trim(),
                bucket.created_at.trim(),
            ],
        )?;
        let stored = stored_bucket_on(&tx, &bucket.binding.bucket_id)?
            .ok_or_else(|| anyhow!("容量 bucket 已创建但无法读回"))?;
        tx.commit()?;
        Ok(stored.balance)
    }

    pub(crate) fn compute_capacity_bucket_balance(
        &self,
        bucket_id: &str,
    ) -> Result<ComputeCapacityBucketBalance> {
        stored_bucket_on(&self.conn()?, bucket_id)?
            .map(|stored| stored.balance)
            .ok_or_else(|| anyhow!("容量 bucket 不存在"))
    }
}

fn validate_pool(pool: &ComputeCapacityPool) -> Result<()> {
    if pool.schema != COMPUTE_CAPACITY_POOL_SCHEMA {
        bail!("容量池 schema 不受支持");
    }
    for (label, value) in [
        ("容量池 ID", pool.binding.pool_id.as_str()),
        ("容量池摘要", pool.binding.pool_digest.as_str()),
        ("提供者 ID", pool.provider_id.as_str()),
        ("资源范围摘要", pool.resource_scope_digest.as_str()),
        ("资源档案摘要", pool.resource_profile_digest.as_str()),
        ("区域", pool.region_or_data_zone.as_str()),
        ("创建时间", pool.created_at.as_str()),
    ] {
        if value.trim().is_empty() {
            bail!("{label}不能为空");
        }
    }
    if pool.binding.capacity_epoch <= 0 || pool.binding.pool_revision <= 0 {
        bail!("容量池 epoch 和版本必须为正整数");
    }
    if pool.meter_policies.is_empty() {
        bail!("容量池至少需要一种计量策略");
    }
    let mut meters = std::collections::BTreeSet::new();
    for policy in &pool.meter_policies {
        if policy.meter.trim().is_empty()
            || policy.policy_digest.trim().is_empty()
            || policy.quantum_units <= 0
            || !meters.insert(policy.meter.trim())
        {
            bail!("容量池计量策略无效或重复");
        }
    }
    Ok(())
}

fn validate_bucket(bucket: &ComputeCapacityBucket, window: &ComputeDeliveryWindow) -> Result<()> {
    if bucket.schema != COMPUTE_CAPACITY_BUCKET_SCHEMA {
        bail!("容量 bucket schema 不受支持");
    }
    if bucket.status != ComputeCapacityBucketStatus::Open || bucket.issued_units != 0 {
        bail!("新容量 bucket 必须以 open 和零发行余额创建");
    }
    if bucket.binding.delivery_window != window.binding {
        bail!("容量 bucket 与交付窗口绑定不一致");
    }
    if bucket.binding.bucket_id.trim().is_empty()
        || bucket.binding.bucket_digest.trim().is_empty()
        || bucket.binding.meter.trim().is_empty()
        || bucket.binding.meter_policy_digest.trim().is_empty()
        || bucket.binding.quantum_units <= 0
    {
        bail!("容量 bucket 身份或计量策略无效");
    }
    let starts = DateTime::parse_from_rfc3339(window.starts_at_utc.trim())
        .context("交付窗口开始时间不是 RFC3339")?;
    let ends = DateTime::parse_from_rfc3339(window.ends_at_utc.trim())
        .context("交付窗口结束时间不是 RFC3339")?;
    if starts.offset().local_minus_utc() != 0 || ends.offset().local_minus_utc() != 0 {
        bail!("交付窗口必须使用 UTC 时区");
    }
    if starts >= ends {
        bail!("交付窗口结束时间必须晚于开始时间");
    }
    Ok(())
}

fn ensure_pool_binding(
    conn: &Connection,
    binding: &crate::compute_federation::capacity::ComputeCapacityPoolBinding,
) -> Result<()> {
    let digest = conn
        .query_row(
            "SELECT pool_digest FROM compute_capacity_pool_versions
              WHERE pool_id=?1 AND capacity_epoch=?2 AND pool_revision=?3",
            params![
                binding.pool_id.trim(),
                binding.capacity_epoch,
                binding.pool_revision
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match digest {
        Some(digest) if digest == binding.pool_digest => Ok(()),
        Some(_) => bail!("容量池版本摘要不匹配"),
        None => bail!("容量池版本不存在"),
    }
}

fn ensure_meter_policy(conn: &Connection, bucket: &ComputeCapacityBucket) -> Result<()> {
    let meters_json: String = conn.query_row(
        "SELECT supported_meters_json FROM compute_capacity_pool_versions
          WHERE pool_id=?1 AND capacity_epoch=?2 AND pool_revision=?3",
        params![
            bucket.binding.pool.pool_id.trim(),
            bucket.binding.pool.capacity_epoch,
            bucket.binding.pool.pool_revision,
        ],
        |row| row.get(0),
    )?;
    let policies: Vec<ComputeCapacityMeterPolicy> = serde_json::from_str(&meters_json)?;
    let matches = policies.iter().any(|policy| {
        policy.meter == bucket.binding.meter
            && policy.meter_mode == bucket.binding.meter_mode
            && policy.quantum_units == bucket.binding.quantum_units
            && policy.policy_digest == bucket.binding.meter_policy_digest
    });
    if !matches {
        bail!("容量 bucket 计量策略不属于绑定的容量池版本");
    }
    Ok(())
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
