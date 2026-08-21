use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::store::Store;

use super::{
    model::{ManagedRolloutPayload, ManagedRolloutPlan, ROLLOUT_STATUS_PLANNED},
    validation::payload_hash,
};

struct StoredManagedRolloutPlan {
    plan: ManagedRolloutPlan,
    source_configuration_revision: i64,
    source_version_id: String,
}

impl Store {
    pub(crate) fn create_managed_rollout_plan(
        &self,
        project_id: &str,
        actor_user_id: &str,
        payload: &ManagedRolloutPayload,
    ) -> Result<ManagedRolloutPlan> {
        let plan_sha256 = payload_hash(payload)?;
        let payload_json = serde_json::to_string(payload)?;
        let id = format!("erp_rollout_{}", uuid::Uuid::new_v4().simple());
        let created_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        self.conn()?.execute(
            "INSERT OR IGNORE INTO erp_managed_rollout_plans (
               id, project_id, instance_id, merchant_id,
               source_configuration_revision, source_version_id,
               plan_sha256, payload_json, status, created_by_user_id, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                project_id.trim(),
                payload.source.instance_id,
                payload.source.merchant_id,
                payload.source.configuration_revision,
                payload.source.blueprint_version_id,
                plan_sha256,
                payload_json,
                ROLLOUT_STATUS_PLANNED,
                actor_user_id.trim(),
                created_at,
            ],
        )?;
        self.managed_rollout_plan_by_hash(project_id, &payload.source.instance_id, &plan_sha256)
    }

    pub(crate) fn managed_rollout_plan(
        &self,
        project_id: &str,
        instance_id: &str,
        rollout_id: &str,
    ) -> Result<ManagedRolloutPlan> {
        self.conn()?
            .query_row(
                &format!("{ROLLOUT_SELECT} WHERE project_id=?1 AND instance_id=?2 AND id=?3"),
                params![project_id.trim(), instance_id.trim(), rollout_id.trim()],
                rollout_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("托管发布计划不存在"))
            .and_then(verify_stored_plan)
    }

    pub(crate) fn list_managed_rollout_plans(
        &self,
        project_id: &str,
        instance_id: &str,
        limit: usize,
    ) -> Result<Vec<ManagedRolloutPlan>> {
        let limit = limit.clamp(1, 100) as i64;
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{ROLLOUT_SELECT}
             WHERE project_id=?1 AND instance_id=?2
             ORDER BY created_at DESC, id DESC LIMIT ?3"
        ))?;
        let rows = statement
            .query_map(
                params![project_id.trim(), instance_id.trim(), limit],
                rollout_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().map(verify_stored_plan).collect()
    }

    fn managed_rollout_plan_by_hash(
        &self,
        project_id: &str,
        instance_id: &str,
        plan_sha256: &str,
    ) -> Result<ManagedRolloutPlan> {
        self.conn()?
            .query_row(
                &format!(
                    "{ROLLOUT_SELECT}
                     WHERE project_id=?1 AND instance_id=?2 AND plan_sha256=?3"
                ),
                params![project_id.trim(), instance_id.trim(), plan_sha256],
                rollout_from_row,
            )
            .map_err(|error| anyhow!(error).context("读取托管发布计划失败"))
            .and_then(verify_stored_plan)
    }
}

fn rollout_from_row(row: &Row<'_>) -> rusqlite::Result<StoredManagedRolloutPlan> {
    let payload_json: String = row.get(8)?;
    let payload = serde_json::from_str(&payload_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(StoredManagedRolloutPlan {
        plan: ManagedRolloutPlan {
            id: row.get(0)?,
            project_id: row.get(1)?,
            instance_id: row.get(2)?,
            merchant_id: row.get(3)?,
            plan_sha256: row.get(6)?,
            status: row.get(7)?,
            payload,
            created_by_user_id: row.get(9)?,
            created_at: row.get(10)?,
        },
        source_configuration_revision: row.get(4)?,
        source_version_id: row.get(5)?,
    })
}

fn verify_stored_plan(stored: StoredManagedRolloutPlan) -> Result<ManagedRolloutPlan> {
    let plan = stored.plan;
    if plan.status != ROLLOUT_STATUS_PLANNED
        || plan.instance_id != plan.payload.source.instance_id
        || plan.merchant_id != plan.payload.source.merchant_id
        || stored.source_configuration_revision != plan.payload.source.configuration_revision
        || stored.source_version_id != plan.payload.source.blueprint_version_id
        || plan.plan_sha256 != payload_hash(&plan.payload)?
    {
        bail!("托管发布计划完整性校验失败");
    }
    Ok(plan)
}

const ROLLOUT_SELECT: &str = "SELECT id, project_id, instance_id, merchant_id,
            source_configuration_revision, source_version_id, plan_sha256, status,
            payload_json, created_by_user_id, created_at
       FROM erp_managed_rollout_plans";
