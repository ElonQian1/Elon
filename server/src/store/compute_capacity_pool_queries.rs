use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::compute_federation::capacity::{
    ComputeCapacityMeterPolicy, ComputeCapacityPool, ComputeCapacityPoolBinding,
    ComputeCapacityPoolStatus, COMPUTE_CAPACITY_POOL_SCHEMA,
};

use super::Store;

impl Store {
    pub(crate) fn compute_capacity_pool(&self, pool_id: &str) -> Result<ComputeCapacityPool> {
        if pool_id.trim().is_empty() {
            bail!("容量池 ID 不能为空");
        }
        current_capacity_pool_on(&self.conn()?, pool_id.trim())?
            .ok_or_else(|| anyhow!("容量池不存在"))
    }

    pub(crate) fn compute_capacity_pool_if_exists(
        &self,
        pool_id: &str,
    ) -> Result<Option<ComputeCapacityPool>> {
        if pool_id.trim().is_empty() {
            bail!("容量池 ID 不能为空");
        }
        current_capacity_pool_on(&self.conn()?, pool_id.trim())
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
    let profile: serde_json::Value =
        serde_json::from_str(&resource_profile_json).context("容量池资源档案 JSON 无效")?;
    let resource_profile_digest = profile
        .get("digest")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("容量池资源档案缺少摘要"))?
        .to_string();
    let profile_value = profile
        .get("profile")
        .ok_or_else(|| anyhow!("容量池资源档案缺少内容"))?;
    let recomputed_profile_digest = hex::encode(Sha256::digest(serde_json::to_vec(profile_value)?));
    if recomputed_profile_digest != resource_profile_digest {
        bail!("容量池资源档案摘要审计失败");
    }
    let meter_policies: Vec<ComputeCapacityMeterPolicy> =
        serde_json::from_str(&meter_policies_json).context("容量池计量策略 JSON 无效")?;
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
