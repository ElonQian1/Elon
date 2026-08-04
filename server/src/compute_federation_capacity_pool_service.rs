use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        capacity::{
            ComputeCapacityMeterMode, ComputeCapacityMeterPolicy, ComputeCapacityPool,
            ComputeCapacityPoolBinding, ComputeCapacityPoolStatus, COMPUTE_CAPACITY_POOL_SCHEMA,
        },
        provider::{
            PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING,
        },
    },
    store::{ComputeCapacityLedgerHistoryPage, ComputeCapacityPoolAuditReport, Store},
};

const MAX_RESOURCE_PROFILE_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateMyComputeCapacityPoolRequest {
    pub pool_id: String,
    pub resource_scope_key: String,
    pub region_or_data_zone: String,
    pub resource_profile: Value,
    pub meter_policies: Vec<CreateMyComputeCapacityMeterPolicyRequest>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateMyComputeCapacityMeterPolicyRequest {
    pub meter: String,
    pub meter_mode: String,
    pub quantum_units: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MyComputeCapacityPoolView {
    pub pool_id: String,
    pub provider_id: String,
    pub status: String,
    pub capacity_epoch: i64,
    pub pool_revision: i64,
    pub pool_digest: String,
    pub resource_scope_digest: String,
    pub resource_profile_digest: String,
    pub region_or_data_zone: String,
    pub meter_policies: Vec<ComputeCapacityMeterPolicy>,
    pub created_at: String,
    pub replayed: bool,
}

pub(crate) fn create_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    request: CreateMyComputeCapacityPoolRequest,
) -> Result<MyComputeCapacityPoolView> {
    let provider = owned_provider(store, user_id, provider_id)?;
    if provider.provider.provider_kind == PROVIDER_KIND_EXTERNAL_POOL {
        bail!("external_pool 容量池必须由服务端适配器管理");
    }
    if !matches!(
        provider.provider.status.as_str(),
        PROVIDER_STATUS_REGISTERING | PROVIDER_STATUS_ACTIVE
    ) {
        bail!("算力 Provider 当前状态不允许登记容量池");
    }
    let (pool, resource_profile) = build_pool(provider_id, request)?;
    if let Some(existing) = store.compute_capacity_pool_if_exists(&pool.binding.pool_id)? {
        ensure_replay_matches(&existing, &pool)?;
        return Ok(pool_view(existing, true));
    }
    store.register_compute_capacity_pool(&pool, &resource_profile)?;
    Ok(pool_view(
        store.compute_capacity_pool(&pool.binding.pool_id)?,
        false,
    ))
}

pub(crate) fn get_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
) -> Result<MyComputeCapacityPoolView> {
    let pool = owned_pool_for_user(store, user_id, provider_id, pool_id)?;
    Ok(pool_view(pool, false))
}

pub(crate) fn list_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    limit: usize,
) -> Result<Vec<MyComputeCapacityPoolView>> {
    owned_provider(store, user_id, provider_id)?;
    store
        .list_compute_capacity_pools_for_provider(provider_id, limit)?
        .into_iter()
        .map(|pool| {
            ensure_pool_provider(&pool, provider_id)?;
            Ok(pool_view(pool, false))
        })
        .collect()
}

pub(crate) fn audit_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
) -> Result<ComputeCapacityPoolAuditReport> {
    let pool = owned_pool_for_user(store, user_id, provider_id, pool_id)?;
    store.audit_compute_capacity_pool_epoch(&pool.binding.pool_id, pool.binding.capacity_epoch)
}

pub(crate) fn list_ledger_history_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    before_sequence: Option<i64>,
    limit: usize,
) -> Result<ComputeCapacityLedgerHistoryPage> {
    let pool = owned_pool_for_user(store, user_id, provider_id, pool_id)?;
    store.list_compute_capacity_ledger_history(
        &pool.binding.pool_id,
        pool.binding.capacity_epoch,
        before_sequence,
        limit,
    )
}

pub(crate) fn owned_pool_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
) -> Result<ComputeCapacityPool> {
    owned_provider(store, user_id, provider_id)?;
    validate_bounded("容量池 ID", pool_id, 160)?;
    let pool = store.compute_capacity_pool(pool_id)?;
    ensure_pool_provider(&pool, provider_id)?;
    Ok(pool)
}

fn owned_provider(
    store: &Store,
    user_id: &str,
    provider_id: &str,
) -> Result<crate::store::ComputeProviderRegistrationReceipt> {
    validate_bounded("算力 Provider ID", provider_id, 160)?;
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    Ok(provider)
}

fn build_pool(
    provider_id: &str,
    request: CreateMyComputeCapacityPoolRequest,
) -> Result<(ComputeCapacityPool, Value)> {
    validate_bounded("容量池 ID", &request.pool_id, 160)?;
    validate_bounded("资源范围密钥", &request.resource_scope_key, 256)?;
    validate_bounded("容量区域", &request.region_or_data_zone, 80)?;
    let profile_bytes = serde_json::to_vec(&request.resource_profile)?;
    if !request.resource_profile.is_object() || profile_bytes.len() > MAX_RESOURCE_PROFILE_BYTES {
        bail!("资源档案必须是且只能是 32 KiB 以内的 JSON 对象");
    }
    let resource_profile_digest = digest_bytes(&profile_bytes);
    let resource_scope_digest = digest_json(&json!({
        "provider_id": provider_id,
        "resource_scope_key": request.resource_scope_key,
    }))?;
    let meter_policies = canonical_meter_policies(request.meter_policies)?;
    let pool_digest = digest_json(&json!({
        "schema": COMPUTE_CAPACITY_POOL_SCHEMA,
        "pool_id": request.pool_id,
        "capacity_epoch": 1,
        "pool_revision": 1,
        "provider_id": provider_id,
        "resource_scope_digest": resource_scope_digest,
        "resource_profile_digest": resource_profile_digest,
        "region_or_data_zone": request.region_or_data_zone,
        "meter_policies": meter_policies,
    }))?;
    Ok((
        ComputeCapacityPool {
            schema: COMPUTE_CAPACITY_POOL_SCHEMA.to_string(),
            binding: ComputeCapacityPoolBinding {
                pool_id: request.pool_id,
                capacity_epoch: 1,
                pool_revision: 1,
                pool_digest,
            },
            provider_id: provider_id.to_string(),
            resource_scope_digest,
            status: ComputeCapacityPoolStatus::Registering,
            resource_profile_digest,
            region_or_data_zone: request.region_or_data_zone,
            meter_policies,
            created_at: Utc::now().to_rfc3339(),
        },
        request.resource_profile,
    ))
}

fn canonical_meter_policies(
    requests: Vec<CreateMyComputeCapacityMeterPolicyRequest>,
) -> Result<Vec<ComputeCapacityMeterPolicy>> {
    if requests.is_empty() || requests.len() > 64 {
        bail!("容量池计量策略数量必须在 1 到 64 之间");
    }
    let mut meters = BTreeSet::new();
    let mut policies = Vec::with_capacity(requests.len());
    for request in requests {
        validate_bounded("容量 meter", &request.meter, 80)?;
        if !meters.insert(request.meter.clone()) {
            bail!("容量 meter 不能重复");
        }
        if request.quantum_units <= 0 {
            bail!("容量 meter quantum_units 必须为正整数");
        }
        let meter_mode = match request.meter_mode.as_str() {
            "consumable" => ComputeCapacityMeterMode::Consumable,
            "reusable" => ComputeCapacityMeterMode::Reusable,
            _ => bail!("容量 meter_mode 只支持 consumable 或 reusable"),
        };
        let policy_digest = digest_json(&json!({
            "meter": request.meter,
            "meter_mode": request.meter_mode,
            "quantum_units": request.quantum_units,
        }))?;
        policies.push(ComputeCapacityMeterPolicy {
            meter: request.meter,
            meter_mode,
            quantum_units: request.quantum_units,
            policy_digest,
        });
    }
    policies.sort_by(|left, right| left.meter.cmp(&right.meter));
    Ok(policies)
}

fn ensure_replay_matches(
    existing: &ComputeCapacityPool,
    requested: &ComputeCapacityPool,
) -> Result<()> {
    if existing.provider_id != requested.provider_id
        || existing.resource_scope_digest != requested.resource_scope_digest
        || existing.resource_profile_digest != requested.resource_profile_digest
        || existing.region_or_data_zone != requested.region_or_data_zone
        || existing.meter_policies != requested.meter_policies
        || existing.binding.capacity_epoch != requested.binding.capacity_epoch
        || existing.binding.pool_revision != requested.binding.pool_revision
        || existing.binding.pool_digest != requested.binding.pool_digest
    {
        bail!("容量池 ID 已绑定不同的资源合同");
    }
    Ok(())
}

fn ensure_pool_provider(pool: &ComputeCapacityPool, provider_id: &str) -> Result<()> {
    if pool.provider_id != provider_id {
        bail!("容量池不属于指定算力 Provider");
    }
    Ok(())
}

fn pool_view(pool: ComputeCapacityPool, replayed: bool) -> MyComputeCapacityPoolView {
    MyComputeCapacityPoolView {
        pool_id: pool.binding.pool_id,
        provider_id: pool.provider_id,
        status: pool_status_value(pool.status).to_string(),
        capacity_epoch: pool.binding.capacity_epoch,
        pool_revision: pool.binding.pool_revision,
        pool_digest: pool.binding.pool_digest,
        resource_scope_digest: pool.resource_scope_digest,
        resource_profile_digest: pool.resource_profile_digest,
        region_or_data_zone: pool.region_or_data_zone,
        meter_policies: pool.meter_policies,
        created_at: pool.created_at,
        replayed,
    }
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
    let bytes = serde_json::to_vec(value).context("容量合同无法规范序列化")?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
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
