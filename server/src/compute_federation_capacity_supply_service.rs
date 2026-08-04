use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        capacity::{ComputeCapacityBucketStatus, ComputeCapacityPoolBinding},
        market::ComputeDeliveryWindowBinding,
        provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING},
    },
    compute_federation_capacity_pool_service,
    store::{
        AddComputeCapacitySupply, AddComputeCapacitySupplyLine, ComputeCapacityLedgerWriteReceipt,
        Store,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AddMyComputeCapacitySupplyRequest {
    pub idempotency_key: String,
    pub lines: Vec<AddMyComputeCapacitySupplyLineRequest>,
    pub confirm_supply: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AddMyComputeCapacitySupplyLineRequest {
    pub bucket_id: String,
    pub quantity_units: i64,
}

pub(crate) fn add_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    request: AddMyComputeCapacitySupplyRequest,
) -> Result<ComputeCapacityLedgerWriteReceipt> {
    if !request.confirm_supply {
        bail!("追加容量供给前必须显式确认 confirm_supply=true");
    }
    validate_bounded("容量发行幂等键", &request.idempotency_key, 160)?;
    if request.lines.is_empty() || request.lines.len() > 64 {
        bail!("容量发行明细数量必须在 1 到 64 之间");
    }
    let pool = compute_federation_capacity_pool_service::owned_pool_for_user(
        store,
        user_id,
        provider_id,
        pool_id,
    )?;
    let provider = store.compute_provider(provider_id)?;
    if !matches!(
        provider.provider.status.as_str(),
        PROVIDER_STATUS_REGISTERING | PROVIDER_STATUS_ACTIVE
    ) {
        bail!("算力 Provider 当前状态不允许追加容量供给");
    }
    let (delivery_window, window_ends_at, lines) =
        validated_supply_lines(store, &pool.binding, request.lines)?;
    let idempotency_scope = supply_scope(user_id, &pool.binding.pool_id)?;
    let existing_occurred_at =
        store.compute_capacity_supply_occurred_at(&idempotency_scope, &request.idempotency_key)?;
    let occurred_at = existing_occurred_at
        .clone()
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    if existing_occurred_at.is_none() {
        ensure_window_open(&window_ends_at)?;
    }
    let input = AddComputeCapacitySupply {
        pool: pool.binding,
        delivery_window,
        subject_kind: "compute_provider_owner".to_string(),
        subject_id: user_id.to_string(),
        idempotency_scope: idempotency_scope.clone(),
        idempotency_key: request.idempotency_key.clone(),
        lines,
        occurred_at,
    };
    let mut concurrent_retry = input.clone();
    match store.add_compute_capacity_supply(input) {
        Ok(receipt) => Ok(receipt),
        Err(first_error) => {
            let Some(stored_occurred_at) = store.compute_capacity_supply_occurred_at(
                &idempotency_scope,
                &request.idempotency_key,
            )?
            else {
                return Err(first_error);
            };
            if concurrent_retry.occurred_at == stored_occurred_at {
                return Err(first_error);
            }
            concurrent_retry.occurred_at = stored_occurred_at;
            store
                .add_compute_capacity_supply(concurrent_retry)
                .context("容量发行并发首写后幂等重放失败")
        }
    }
}

fn validated_supply_lines(
    store: &Store,
    pool: &ComputeCapacityPoolBinding,
    requests: Vec<AddMyComputeCapacitySupplyLineRequest>,
) -> Result<(
    ComputeDeliveryWindowBinding,
    String,
    Vec<AddComputeCapacitySupplyLine>,
)> {
    let mut bucket_ids = BTreeSet::new();
    let mut delivery_window = None;
    let mut window_ends_at = None;
    let mut lines = Vec::with_capacity(requests.len());
    for request in requests {
        validate_bounded("容量 bucket ID", &request.bucket_id, 160)?;
        if !bucket_ids.insert(request.bucket_id.clone()) {
            bail!("同一容量发行请求不能重复 bucket");
        }
        if request.quantity_units <= 0 {
            bail!("容量发行数量必须为正整数");
        }
        let bucket = store.compute_capacity_bucket(&request.bucket_id)?;
        if bucket.balance.binding.pool != *pool {
            bail!("容量发行 bucket 不属于当前 Pool 版本");
        }
        if bucket.balance.status != ComputeCapacityBucketStatus::Open {
            bail!("容量发行只能写入 open bucket");
        }
        if request.quantity_units % bucket.balance.binding.quantum_units != 0 {
            bail!("容量发行数量必须是 bucket 最小量子的整数倍");
        }
        match &delivery_window {
            Some(current) if current != &bucket.balance.binding.delivery_window => {
                bail!("一次容量发行的全部 bucket 必须属于同一交付窗口");
            }
            None => delivery_window = Some(bucket.balance.binding.delivery_window.clone()),
            _ => {}
        }
        match &window_ends_at {
            Some(current) if current != &bucket.ends_at_utc => {
                bail!("同一交付窗口的 bucket 结束时间不一致");
            }
            None => window_ends_at = Some(bucket.ends_at_utc),
            _ => {}
        }
        lines.push(AddComputeCapacitySupplyLine {
            bucket_id: request.bucket_id,
            quantity_units: request.quantity_units,
        });
    }
    lines.sort_by(|left, right| left.bucket_id.cmp(&right.bucket_id));
    Ok((
        delivery_window.ok_or_else(|| anyhow::anyhow!("容量发行缺少交付窗口"))?,
        window_ends_at.ok_or_else(|| anyhow::anyhow!("容量发行缺少窗口结束时间"))?,
        lines,
    ))
}

fn ensure_window_open(ends_at_utc: &str) -> Result<()> {
    let ends = DateTime::parse_from_rfc3339(ends_at_utc).context("交付窗口结束时间无效")?;
    if ends.with_timezone(&Utc) <= Utc::now() {
        bail!("不能向已经结束的交付窗口首次追加供给");
    }
    Ok(())
}

fn supply_scope(user_id: &str, pool_id: &str) -> Result<String> {
    let payload = serde_json::to_vec(&json!({
        "schema":"compute_federation.owner_supply_scope.v1",
        "user_id":user_id,
        "pool_id":pool_id,
    }))?;
    Ok(format!(
        "compute_owner_supply:{}",
        hex::encode(Sha256::digest(payload))
    ))
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
