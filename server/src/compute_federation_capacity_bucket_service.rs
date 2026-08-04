use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        capacity::{
            ComputeCapacityBucket, ComputeCapacityBucketBalance, ComputeCapacityBucketBinding,
            ComputeCapacityBucketStatus, ComputeCapacityPool, ComputeCapacityPoolBinding,
            COMPUTE_CAPACITY_BUCKET_SCHEMA,
        },
        market::{ComputeDeliveryWindow, ComputeDeliveryWindowBinding},
    },
    compute_federation_capacity_pool_service,
    store::{ComputeCapacityBucketRead, Store},
};

const DELIVERY_WINDOW_SCHEMA: &str = "compute_federation.delivery_window.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateMyComputeCapacityBucketRequest {
    pub bucket_id: String,
    pub window_id: String,
    pub starts_at_utc: String,
    pub ends_at_utc: String,
    pub meter: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MyComputeCapacityBucketView {
    pub balance: ComputeCapacityBucketBalance,
    pub starts_at_utc: String,
    pub ends_at_utc: String,
    pub replayed: bool,
}

pub(crate) fn create_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    request: CreateMyComputeCapacityBucketRequest,
) -> Result<MyComputeCapacityBucketView> {
    let pool = compute_federation_capacity_pool_service::owned_pool_for_user(
        store,
        user_id,
        provider_id,
        pool_id,
    )?;
    let (bucket, window) = build_bucket(&pool, request)?;
    if let Some(existing) = store.compute_capacity_bucket_if_exists(&bucket.binding.bucket_id)? {
        ensure_replay_matches(&existing, &bucket, &window)?;
        return Ok(bucket_view(existing, true));
    }
    ensure_window_not_ended(&window)?;
    store.create_compute_capacity_bucket(&bucket, &window)?;
    Ok(bucket_view(
        store.compute_capacity_bucket(&bucket.binding.bucket_id)?,
        false,
    ))
}

pub(crate) fn get_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    bucket_id: &str,
) -> Result<MyComputeCapacityBucketView> {
    let pool = compute_federation_capacity_pool_service::owned_pool_for_user(
        store,
        user_id,
        provider_id,
        pool_id,
    )?;
    validate_bounded("容量 bucket ID", bucket_id, 160)?;
    let bucket = store.compute_capacity_bucket(bucket_id)?;
    ensure_bucket_pool(&bucket, &pool.binding)?;
    Ok(bucket_view(bucket, false))
}

pub(crate) fn list_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    limit: usize,
) -> Result<Vec<MyComputeCapacityBucketView>> {
    let pool = compute_federation_capacity_pool_service::owned_pool_for_user(
        store,
        user_id,
        provider_id,
        pool_id,
    )?;
    store
        .list_compute_capacity_buckets_for_pool(
            &pool.binding.pool_id,
            pool.binding.capacity_epoch,
            pool.binding.pool_revision,
            limit,
        )?
        .into_iter()
        .map(|bucket| {
            ensure_bucket_pool(&bucket, &pool.binding)?;
            Ok(bucket_view(bucket, false))
        })
        .collect()
}

fn build_bucket(
    pool: &ComputeCapacityPool,
    request: CreateMyComputeCapacityBucketRequest,
) -> Result<(ComputeCapacityBucket, ComputeDeliveryWindow)> {
    validate_bounded("容量 bucket ID", &request.bucket_id, 160)?;
    validate_bounded("交付窗口 ID", &request.window_id, 160)?;
    validate_bounded("容量 meter", &request.meter, 80)?;
    let starts_at_utc = canonical_utc("交付窗口开始时间", &request.starts_at_utc)?;
    let ends_at_utc = canonical_utc("交付窗口结束时间", &request.ends_at_utc)?;
    let starts = DateTime::parse_from_rfc3339(&starts_at_utc)?;
    let ends = DateTime::parse_from_rfc3339(&ends_at_utc)?;
    if starts >= ends {
        bail!("交付窗口结束时间必须晚于开始时间");
    }
    let meter_policy = pool
        .meter_policies
        .iter()
        .find(|policy| policy.meter == request.meter)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("容量 meter 不属于当前 Pool 版本"))?;
    let window_digest = digest_json(&json!({
        "schema": DELIVERY_WINDOW_SCHEMA,
        "window_id": request.window_id,
        "starts_at_utc": starts_at_utc,
        "ends_at_utc": ends_at_utc,
    }))?;
    let window = ComputeDeliveryWindow {
        binding: ComputeDeliveryWindowBinding {
            window_id: request.window_id,
            window_digest,
        },
        starts_at_utc,
        ends_at_utc,
    };
    let bucket_digest = digest_json(&json!({
        "schema": COMPUTE_CAPACITY_BUCKET_SCHEMA,
        "bucket_id": request.bucket_id,
        "pool": pool.binding,
        "delivery_window": window,
        "meter": meter_policy.meter,
        "meter_mode": meter_policy.meter_mode,
        "quantum_units": meter_policy.quantum_units,
        "meter_policy_digest": meter_policy.policy_digest,
    }))?;
    Ok((
        ComputeCapacityBucket {
            schema: COMPUTE_CAPACITY_BUCKET_SCHEMA.to_string(),
            binding: ComputeCapacityBucketBinding {
                bucket_id: request.bucket_id,
                bucket_digest,
                pool: pool.binding.clone(),
                delivery_window: window.binding.clone(),
                meter: meter_policy.meter,
                meter_mode: meter_policy.meter_mode,
                quantum_units: meter_policy.quantum_units,
                meter_policy_digest: meter_policy.policy_digest,
            },
            status: ComputeCapacityBucketStatus::Open,
            issued_units: 0,
            created_at: Utc::now().to_rfc3339(),
        },
        window,
    ))
}

fn ensure_window_not_ended(window: &ComputeDeliveryWindow) -> Result<()> {
    let ends = DateTime::parse_from_rfc3339(&window.ends_at_utc)?;
    if ends.with_timezone(&Utc) <= Utc::now() {
        bail!("不能为已经结束的交付窗口首次创建容量 bucket");
    }
    Ok(())
}

fn ensure_replay_matches(
    existing: &ComputeCapacityBucketRead,
    requested: &ComputeCapacityBucket,
    window: &ComputeDeliveryWindow,
) -> Result<()> {
    if existing.balance.binding != requested.binding
        || existing.starts_at_utc != window.starts_at_utc
        || existing.ends_at_utc != window.ends_at_utc
    {
        bail!("容量 bucket ID 已绑定不同窗口或资源合同");
    }
    Ok(())
}

fn ensure_bucket_pool(
    bucket: &ComputeCapacityBucketRead,
    pool: &ComputeCapacityPoolBinding,
) -> Result<()> {
    if &bucket.balance.binding.pool != pool {
        bail!("容量 bucket 不属于当前 Pool 版本");
    }
    Ok(())
}

fn bucket_view(bucket: ComputeCapacityBucketRead, replayed: bool) -> MyComputeCapacityBucketView {
    MyComputeCapacityBucketView {
        balance: bucket.balance,
        starts_at_utc: bucket.starts_at_utc,
        ends_at_utc: bucket.ends_at_utc,
        replayed,
    }
}

fn canonical_utc(label: &str, value: &str) -> Result<String> {
    let parsed = DateTime::parse_from_rfc3339(value.trim())
        .with_context(|| format!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed.with_timezone(&Utc).to_rfc3339())
}

fn validate_bounded(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty() || value != value.trim() {
        bail!("{label}不能为空或包含首尾空白");
    }
    if value.chars().count() > max_len {
        bail!("{label}长度不能超过 {max_len}");
    }
    Ok(())
}

fn digest_json(value: &Value) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}
