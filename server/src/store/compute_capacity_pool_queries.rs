use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::capacity::{
    ComputeCapacityMeterMode, ComputeCapacityMeterPolicy, ComputeCapacityPool,
    ComputeCapacityPoolBinding, ComputeCapacityPoolStatus, COMPUTE_CAPACITY_POOL_SCHEMA,
};

use super::Store;

const LEGACY_MAX_RESOURCE_PROFILE_BYTES: usize = 32 * 1024;

#[derive(Debug)]
pub(in crate::store) struct AuditedComputeCapacityPoolVersion {
    pub(in crate::store) binding: ComputeCapacityPoolBinding,
    pub(in crate::store) provider_id: String,
    pub(in crate::store) resource_scope_digest: String,
    pub(in crate::store) resource_profile_digest: String,
    pub(in crate::store) region_or_data_zone: String,
    pub(in crate::store) meter_policies: Vec<ComputeCapacityMeterPolicy>,
    pub(in crate::store) created_at: String,
}

impl Store {
    pub(crate) fn compute_capacity_pool(&self, pool_id: &str) -> Result<ComputeCapacityPool> {
        if pool_id.trim().is_empty() {
            bail!("容量池 ID 不能为空");
        }
        current_capacity_pool_on(&*self.conn()?, pool_id.trim())?
            .ok_or_else(|| anyhow!("容量池不存在"))
    }

    pub(crate) fn compute_capacity_pool_if_exists(
        &self,
        pool_id: &str,
    ) -> Result<Option<ComputeCapacityPool>> {
        if pool_id.trim().is_empty() {
            bail!("容量池 ID 不能为空");
        }
        current_capacity_pool_on(&*self.conn()?, pool_id.trim())
    }

    pub(crate) fn list_compute_capacity_pools_for_provider(
        &self,
        provider_id: &str,
        limit: usize,
    ) -> Result<Vec<ComputeCapacityPool>> {
        if provider_id.trim().is_empty() {
            bail!("算力提供者 ID 不能为空");
        }
        let conn = self.conn()?;
        let pool_ids = {
            let mut stmt = conn.prepare(
                "SELECT pool_id FROM compute_capacity_pools
                  WHERE provider_id=?1
                  ORDER BY updated_at DESC, pool_id ASC
                  LIMIT ?2",
            )?;
            let rows = stmt.query_map(
                params![provider_id.trim(), limit.clamp(1, 100) as i64],
                |row| row.get::<_, String>(0),
            )?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        pool_ids
            .into_iter()
            .map(|pool_id| {
                current_capacity_pool_on(&conn, &pool_id)?
                    .ok_or_else(|| anyhow!("容量池当前版本在列表读取期间消失"))
            })
            .collect()
    }
}

pub(super) fn current_capacity_pool_on(
    conn: &Connection,
    pool_id: &str,
) -> Result<Option<ComputeCapacityPool>> {
    let stored = conn
        .query_row(
            "SELECT p.pool_id, p.provider_id, p.resource_scope_digest, p.status,
                    p.current_capacity_epoch, v.pool_revision, v.pool_digest,
                    v.resource_profile_json, COALESCE(v.region, ''),
                    v.supported_meters_json, v.created_at
               FROM compute_capacity_pools p
               JOIN compute_capacity_pool_versions v ON v.pool_id=p.pool_id
              WHERE p.pool_id=?1 AND v.capacity_epoch=p.current_capacity_epoch
              ORDER BY v.pool_revision DESC
              LIMIT 1",
            params![pool_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                ))
            },
        )
        .optional()?;
    let Some((
        pool_id,
        provider_id,
        resource_scope_digest,
        status,
        capacity_epoch,
        pool_revision,
        pool_digest,
        resource_profile_json,
        region_or_data_zone,
        meter_policies_json,
        created_at,
    )) = stored
    else {
        return Ok(None);
    };
    let (resource_profile_digest, meter_policies) =
        audit_pool_version_payload(&resource_profile_json, &meter_policies_json)?;
    audit_legacy_compute_capacity_pool_digests(
        &ComputeCapacityPoolBinding {
            pool_id: pool_id.clone(),
            capacity_epoch,
            pool_revision,
            pool_digest: pool_digest.clone(),
        },
        &provider_id,
        &resource_scope_digest,
        &resource_profile_digest,
        &region_or_data_zone,
        &meter_policies,
    )?;
    Ok(Some(ComputeCapacityPool {
        schema: COMPUTE_CAPACITY_POOL_SCHEMA.to_string(),
        binding: ComputeCapacityPoolBinding {
            pool_id,
            capacity_epoch,
            pool_revision,
            pool_digest,
        },
        provider_id,
        resource_scope_digest,
        status: parse_pool_status(&status)?,
        resource_profile_digest,
        region_or_data_zone,
        meter_policies,
        created_at,
    }))
}

pub(in crate::store) fn audited_compute_capacity_pool_version_on(
    conn: &Connection,
    pool_id: &str,
    capacity_epoch: i64,
    pool_revision: i64,
) -> Result<Option<AuditedComputeCapacityPoolVersion>> {
    let stored = conn
        .query_row(
            "SELECT v.pool_id, v.capacity_epoch, v.pool_revision, v.pool_digest,
                    p.provider_id, p.resource_scope_digest, v.resource_profile_json,
                    COALESCE(v.region, ''), v.supported_meters_json, v.created_at
               FROM compute_capacity_pool_versions v
               JOIN compute_capacity_pools p ON p.pool_id=v.pool_id
              WHERE v.pool_id=?1 AND v.capacity_epoch=?2 AND v.pool_revision=?3",
            params![pool_id, capacity_epoch, pool_revision],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .optional()?;
    let Some((
        stored_pool_id,
        stored_capacity_epoch,
        stored_pool_revision,
        pool_digest,
        provider_id,
        resource_scope_digest,
        resource_profile_json,
        region_or_data_zone,
        meter_policies_json,
        created_at,
    )) = stored
    else {
        return Ok(None);
    };
    let (resource_profile_digest, meter_policies) =
        audit_pool_version_payload(&resource_profile_json, &meter_policies_json)?;
    if stored_pool_id != pool_id
        || stored_capacity_epoch != capacity_epoch
        || stored_pool_revision != pool_revision
        || stored_capacity_epoch <= 0
        || stored_pool_revision <= 0
        || [
            pool_digest.as_str(),
            provider_id.as_str(),
            resource_scope_digest.as_str(),
            region_or_data_zone.as_str(),
            created_at.as_str(),
        ]
        .into_iter()
        .any(|value| value.trim().is_empty() || value != value.trim())
    {
        bail!("容量池历史版本身份或投影字段审计失败");
    }
    let binding = ComputeCapacityPoolBinding {
        pool_id: stored_pool_id,
        capacity_epoch: stored_capacity_epoch,
        pool_revision: stored_pool_revision,
        pool_digest,
    };
    audit_legacy_compute_capacity_pool_digests(
        &binding,
        &provider_id,
        &resource_scope_digest,
        &resource_profile_digest,
        &region_or_data_zone,
        &meter_policies,
    )?;
    Ok(Some(AuditedComputeCapacityPoolVersion {
        binding,
        provider_id,
        resource_scope_digest,
        resource_profile_digest,
        region_or_data_zone,
        meter_policies,
        created_at,
    }))
}

fn audit_pool_version_payload(
    resource_profile_json: &str,
    meter_policies_json: &str,
) -> Result<(String, Vec<ComputeCapacityMeterPolicy>)> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ExactResourceProfile {
        digest: String,
        profile: serde_json::Value,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ExactMeterPolicy {
        meter: String,
        meter_mode: ComputeCapacityMeterMode,
        quantum_units: i64,
        policy_digest: String,
    }

    let profile: ExactResourceProfile =
        serde_json::from_str(resource_profile_json).context("容量池资源档案 JSON 无效")?;
    let profile_bytes = serde_json::to_vec(&profile.profile)?;
    if !profile.profile.is_object()
        || profile_bytes.len() > LEGACY_MAX_RESOURCE_PROFILE_BYTES
        || profile.digest.trim().is_empty()
        || profile.digest != profile.digest.trim()
    {
        bail!("容量池资源档案缺少摘要");
    }
    let recomputed_profile_digest = hex::encode(Sha256::digest(profile_bytes));
    if recomputed_profile_digest != profile.digest {
        bail!("容量池资源档案摘要审计失败");
    }
    let exact_meter_policies: Vec<ExactMeterPolicy> =
        serde_json::from_str(meter_policies_json).context("容量池计量策略 JSON 无效")?;
    let meter_policies = exact_meter_policies
        .into_iter()
        .map(|policy| ComputeCapacityMeterPolicy {
            meter: policy.meter,
            meter_mode: policy.meter_mode,
            quantum_units: policy.quantum_units,
            policy_digest: policy.policy_digest,
        })
        .collect::<Vec<_>>();
    let mut meters = BTreeSet::new();
    if meter_policies.is_empty()
        || meter_policies.len() > 64
        || meter_policies.iter().any(|policy| {
            policy.meter.trim().is_empty()
                || policy.meter != policy.meter.trim()
                || policy.policy_digest.trim().is_empty()
                || policy.policy_digest != policy.policy_digest.trim()
                || policy.quantum_units <= 0
                || !meters.insert(policy.meter.as_str())
        })
        || meter_policies
            .windows(2)
            .any(|pair| pair[0].meter >= pair[1].meter)
    {
        bail!("容量池历史版本计量策略审计失败");
    }
    Ok((profile.digest, meter_policies))
}

pub(in crate::store) fn audit_legacy_compute_capacity_pool_digests(
    binding: &ComputeCapacityPoolBinding,
    provider_id: &str,
    resource_scope_digest: &str,
    resource_profile_digest: &str,
    region_or_data_zone: &str,
    meter_policies: &[ComputeCapacityMeterPolicy],
) -> Result<()> {
    for policy in meter_policies {
        let meter_mode = match policy.meter_mode {
            ComputeCapacityMeterMode::Consumable => "consumable",
            ComputeCapacityMeterMode::Reusable => "reusable",
        };
        let expected_policy_digest = legacy_capacity_digest(&serde_json::json!({
            "meter": policy.meter.as_str(),
            "meter_mode": meter_mode,
            "quantum_units": policy.quantum_units,
        }))?;
        if expected_policy_digest != policy.policy_digest {
            bail!("容量池历史版本 meter policy 摘要审计失败");
        }
    }
    let expected_pool_digest = legacy_capacity_digest(&serde_json::json!({
        "schema": COMPUTE_CAPACITY_POOL_SCHEMA,
        "pool_id": binding.pool_id.as_str(),
        "capacity_epoch": binding.capacity_epoch,
        "pool_revision": binding.pool_revision,
        "provider_id": provider_id,
        "resource_scope_digest": resource_scope_digest,
        "resource_profile_digest": resource_profile_digest,
        "region_or_data_zone": region_or_data_zone,
        "meter_policies": meter_policies,
    }))?;
    if expected_pool_digest != binding.pool_digest {
        bail!("容量池历史版本 pool 摘要审计失败");
    }
    Ok(())
}

fn legacy_capacity_digest(value: &serde_json::Value) -> Result<String> {
    let bytes = serde_json::to_vec(value).context("容量池历史合同无法按 legacy 公式序列化")?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn parse_pool_status(value: &str) -> Result<ComputeCapacityPoolStatus> {
    match value {
        "registering" => Ok(ComputeCapacityPoolStatus::Registering),
        "active" => Ok(ComputeCapacityPoolStatus::Active),
        "draining" => Ok(ComputeCapacityPoolStatus::Draining),
        "retired" => Ok(ComputeCapacityPoolStatus::Retired),
        "quarantined" => Ok(ComputeCapacityPoolStatus::Quarantined),
        _ => bail!("容量池状态不受支持"),
    }
}
