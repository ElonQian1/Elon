use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Row};

use crate::task_settlement::sui_preflight_job_model::{SuiPreflightJob, SUI_PREFLIGHT_JOB_SCHEMA};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_task_sui_preflight_job(
        &self,
        project_id: &str,
        package_kind: &str,
        projection_package_id: &str,
        target_network: &str,
        handoff_digest: &str,
        projection_digest: &str,
        created_by_user_id: &str,
    ) -> Result<SuiPreflightJob> {
        let timestamp = now();
        let id = new_id("sui_preflight_job");
        self.conn()?.execute(
            "INSERT OR IGNORE INTO task_sui_preflight_jobs (
               id, project_id, package_kind, projection_package_id,
               target_network, handoff_digest, projection_digest, status,
               adapter_id, credential_version, attempt_no, lease_token_hash,
               lease_token_hint, lease_started_at, lease_expires_at,
               lease_deadline_at, report_id, last_error, created_by_user_id,
               completed_at, canceled_at, created_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', NULL, NULL, 0,
               NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?8,
               NULL, NULL, ?9, ?9
             )",
            params![
                id,
                project_id.trim(),
                package_kind.trim(),
                projection_package_id.trim(),
                target_network.trim(),
                handoff_digest.trim(),
                projection_digest.trim(),
                created_by_user_id.trim(),
                timestamp,
            ],
        )?;
        let job = self
            .conn()?
            .query_row(
                &format!(
                    "{JOB_SELECT} WHERE project_id=?1 AND package_kind=?2
                       AND projection_package_id=?3 AND status IN ('pending', 'leased')"
                ),
                params![
                    project_id.trim(),
                    package_kind.trim(),
                    projection_package_id.trim(),
                ],
                job_from_row,
            )
            .map_err(|error| anyhow!(error).context("Sui 预检任务写入后无法读取"))?;
        if job.handoff_digest != handoff_digest.trim()
            || job.projection_digest != projection_digest.trim()
            || job.target_network != target_network.trim()
        {
            bail!("同一投影包的活动 Sui 预检任务内容发生冲突");
        }
        Ok(job)
    }

    pub(crate) fn list_task_sui_preflight_jobs(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<SuiPreflightJob>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{JOB_SELECT} WHERE project_id=?1 ORDER BY created_at DESC LIMIT ?2"
        ))?;
        let jobs = statement
            .query_map(
                params![project_id.trim(), limit.clamp(1, 500) as i64],
                job_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(jobs)
    }

    pub(crate) fn task_sui_preflight_job(
        &self,
        project_id: &str,
        job_id: &str,
    ) -> Result<SuiPreflightJob> {
        self.conn()?
            .query_row(
                &format!("{JOB_SELECT} WHERE project_id=?1 AND id=?2"),
                params![project_id.trim(), job_id.trim()],
                job_from_row,
            )
            .map_err(|error| anyhow!(error).context("Sui 预检任务不存在"))
    }

    pub(crate) fn cancel_task_sui_preflight_job(
        &self,
        project_id: &str,
        job_id: &str,
        reason: &str,
    ) -> Result<SuiPreflightJob> {
        let timestamp = now();
        let changed = self.conn()?.execute(
            "UPDATE task_sui_preflight_jobs
                SET status='canceled', last_error=?1, canceled_at=?2, updated_at=?2
              WHERE project_id=?3 AND id=?4 AND status IN ('pending', 'blocked')",
            params![reason.trim(), timestamp, project_id.trim(), job_id.trim()],
        )?;
        if changed != 1 {
            bail!("只有待领取或已阻断的 Sui 预检任务可以取消");
        }
        self.task_sui_preflight_job(project_id, job_id)
    }

    pub(crate) fn list_task_sui_preflight_candidate_ids(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        let timestamp = now();
        self.conn()?.execute(
            "UPDATE task_sui_preflight_jobs
                SET status='pending', adapter_id=NULL, credential_version=NULL,
                    lease_token_hash=NULL, lease_token_hint=NULL,
                    lease_started_at=NULL, lease_expires_at=NULL,
                    lease_deadline_at=NULL, last_error='lease_expired', updated_at=?1
              WHERE project_id=?2 AND status='leased'
                AND julianday(lease_expires_at) <= julianday(?1)",
            params![timestamp, project_id.trim()],
        )?;
        let conn = self.conn()?;
        let mut statement = conn.prepare(
            "SELECT id FROM task_sui_preflight_jobs
              WHERE project_id=?1 AND status='pending'
              ORDER BY created_at, id LIMIT ?2",
        )?;
        let candidate_ids = statement
            .query_map(
                params![project_id.trim(), limit.clamp(1, 100) as i64],
                |row| row.get(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(candidate_ids)
    }

    pub(crate) fn block_task_sui_preflight_job(
        &self,
        project_id: &str,
        job_id: &str,
        reason: &str,
    ) -> Result<SuiPreflightJob> {
        self.conn()?.execute(
            "UPDATE task_sui_preflight_jobs
                SET status='blocked', last_error=?1, updated_at=?2
              WHERE project_id=?3 AND id=?4 AND status='pending'",
            params![
                clean_reason(reason),
                now(),
                project_id.trim(),
                job_id.trim()
            ],
        )?;
        self.task_sui_preflight_job(project_id, job_id)
    }
}

pub(super) fn job_from_row(row: &Row<'_>) -> rusqlite::Result<SuiPreflightJob> {
    Ok(SuiPreflightJob {
        schema: SUI_PREFLIGHT_JOB_SCHEMA,
        id: row.get(0)?,
        project_id: row.get(1)?,
        package_kind: row.get(2)?,
        projection_package_id: row.get(3)?,
        target_network: row.get(4)?,
        handoff_digest: row.get(5)?,
        projection_digest: row.get(6)?,
        status: row.get(7)?,
        adapter_id: row.get(8)?,
        credential_version: row.get(9)?,
        attempt_no: row.get(10)?,
        lease_token_hint: row.get(11)?,
        lease_started_at: row.get(12)?,
        lease_expires_at: row.get(13)?,
        lease_deadline_at: row.get(14)?,
        report_id: row.get(15)?,
        last_error: row.get(16)?,
        created_by_user_id: row.get(17)?,
        completed_at: row.get(18)?,
        canceled_at: row.get(19)?,
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
    })
}

fn clean_reason(value: &str) -> String {
    value.trim().chars().take(500).collect()
}

pub(super) const JOB_SELECT: &str = "SELECT id, project_id, package_kind, projection_package_id,
            target_network, handoff_digest, projection_digest, status,
            adapter_id, credential_version, attempt_no, lease_token_hint,
            lease_started_at, lease_expires_at, lease_deadline_at,
            report_id, last_error, created_by_user_id, completed_at,
            canceled_at, created_at, updated_at
       FROM task_sui_preflight_jobs";
