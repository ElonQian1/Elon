use anyhow::{anyhow, bail, Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::task_settlement::{
    sui_preflight_job_model::SuiPreflightJob,
    sui_preflight_model::{CreateSuiPreflightReport, SuiPreflightAdapter, SuiPreflightReport},
};

use super::{
    now,
    task_sui_preflight_jobs::{job_from_row, JOB_SELECT},
    task_sui_preflight_reports::insert_or_get_report,
    Store,
};

impl Store {
    pub(crate) fn try_claim_task_sui_preflight_job(
        &self,
        adapter: &SuiPreflightAdapter,
        job_id: &str,
        lease_seconds: i64,
    ) -> Result<Option<(SuiPreflightJob, String)>> {
        let lease_token = new_lease_token();
        let timestamp = now();
        let lease_expires_at = (Utc::now() + Duration::seconds(lease_seconds)).to_rfc3339();
        let lease_deadline_at = (Utc::now() + Duration::hours(1)).to_rfc3339();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE task_sui_preflight_jobs AS job
                SET status='leased', adapter_id=?1, credential_version=?2,
                    attempt_no=attempt_no+1, lease_token_hash=?3,
                    lease_token_hint=?4, lease_started_at=?5,
                    lease_expires_at=?6, lease_deadline_at=?7,
                    last_error=NULL, updated_at=?5
              WHERE job.id=?8 AND job.project_id=?9 AND job.status='pending'
                AND EXISTS (
                  SELECT 1 FROM task_sui_preflight_adapters adapter
                   WHERE adapter.id=?1 AND adapter.project_id=job.project_id
                     AND adapter.status='active' AND adapter.credential_version=?2
                     AND julianday(adapter.expires_at) > julianday(?5)
                )",
            params![
                adapter.id,
                adapter.credential_version,
                lease_token_hash(&lease_token),
                token_hint(&lease_token),
                timestamp,
                lease_expires_at,
                lease_deadline_at,
                job_id.trim(),
                adapter.project_id,
            ],
        )?;
        if changed != 1 {
            tx.rollback()?;
            return Ok(None);
        }
        let job = tx.query_row(
            &format!("{JOB_SELECT} WHERE project_id=?1 AND id=?2"),
            params![adapter.project_id, job_id.trim()],
            job_from_row,
        )?;
        tx.commit()?;
        Ok(Some((job, lease_token)))
    }

    pub(crate) fn renew_task_sui_preflight_job(
        &self,
        adapter: &SuiPreflightAdapter,
        job_id: &str,
        lease_token: &str,
        extend_seconds: i64,
    ) -> Result<SuiPreflightJob> {
        let timestamp = now();
        let requested_expires_at = (Utc::now() + Duration::seconds(extend_seconds)).to_rfc3339();
        let changed = self.conn()?.execute(
            "UPDATE task_sui_preflight_jobs AS job
                SET lease_expires_at=CASE
                      WHEN julianday(?1) < julianday(job.lease_deadline_at) THEN ?1
                      ELSE job.lease_deadline_at
                    END,
                    updated_at=?2
              WHERE job.id=?3 AND job.project_id=?4 AND job.status='leased'
                AND job.adapter_id=?5 AND job.credential_version=?6
                AND job.lease_token_hash=?7
                AND julianday(job.lease_expires_at) > julianday(?2)
                AND julianday(job.lease_deadline_at) > julianday(?2)
                AND EXISTS (
                  SELECT 1 FROM task_sui_preflight_adapters adapter
                   WHERE adapter.id=job.adapter_id AND adapter.status='active'
                     AND adapter.credential_version=job.credential_version
                     AND julianday(adapter.expires_at) > julianday(?2)
                )",
            params![
                requested_expires_at,
                timestamp,
                job_id.trim(),
                adapter.project_id,
                adapter.id,
                adapter.credential_version,
                lease_token_hash(lease_token.trim()),
            ],
        )?;
        if changed != 1 {
            bail!("Sui 预检任务租约无效、已过期、达到最长期限或机器凭据已失效");
        }
        self.task_sui_preflight_job(&adapter.project_id, job_id)
    }

    pub(crate) fn release_task_sui_preflight_job(
        &self,
        adapter: &SuiPreflightAdapter,
        job_id: &str,
        lease_token: &str,
        reason: &str,
    ) -> Result<SuiPreflightJob> {
        let timestamp = now();
        let changed = self.conn()?.execute(
            "UPDATE task_sui_preflight_jobs AS job
                SET status='pending', adapter_id=NULL, credential_version=NULL,
                    lease_token_hash=NULL, lease_token_hint=NULL,
                    lease_started_at=NULL, lease_expires_at=NULL,
                    lease_deadline_at=NULL, last_error=?1, updated_at=?2
              WHERE job.id=?3 AND job.project_id=?4 AND job.status='leased'
                AND job.adapter_id=?5 AND job.credential_version=?6
                AND job.lease_token_hash=?7
                AND julianday(job.lease_expires_at) > julianday(?2)
                AND EXISTS (
                  SELECT 1 FROM task_sui_preflight_adapters adapter
                   WHERE adapter.id=job.adapter_id AND adapter.status='active'
                     AND adapter.credential_version=job.credential_version
                     AND julianday(adapter.expires_at) > julianday(?2)
                )",
            params![
                clean_reason(reason),
                timestamp,
                job_id.trim(),
                adapter.project_id,
                adapter.id,
                adapter.credential_version,
                lease_token_hash(lease_token.trim()),
            ],
        )?;
        if changed != 1 {
            bail!("Sui 预检任务租约无效、已完成、已过期或机器凭据已失效");
        }
        self.task_sui_preflight_job(&adapter.project_id, job_id)
    }

    pub(crate) fn complete_task_sui_preflight_job(
        &self,
        adapter: &SuiPreflightAdapter,
        job_id: &str,
        lease_token: &str,
        report_input: CreateSuiPreflightReport<'_>,
    ) -> Result<(SuiPreflightJob, SuiPreflightReport)> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let job = tx
            .query_row(
                &format!(
                    "{JOB_SELECT} WHERE project_id=?1 AND id=?2
                       AND adapter_id=?3 AND credential_version=?4
                       AND lease_token_hash=?5 AND status IN ('leased', 'completed')"
                ),
                params![
                    adapter.project_id,
                    job_id.trim(),
                    adapter.id,
                    adapter.credential_version,
                    lease_token_hash(lease_token.trim()),
                ],
                job_from_row,
            )
            .map_err(|error| anyhow!(error).context("Sui 预检任务租约无效或机器凭据已失效"))?;
        if job.status == "leased" {
            let active: i64 = tx.query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM task_sui_preflight_adapters
                    WHERE id=?1 AND project_id=?2 AND status='active'
                      AND credential_version=?3
                      AND julianday(expires_at) > julianday(?4)
                 )",
                params![
                    adapter.id,
                    adapter.project_id,
                    adapter.credential_version,
                    timestamp
                ],
                |row| row.get(0),
            )?;
            let lease_current: i64 = tx.query_row(
                "SELECT (julianday(?1) > julianday(?3)
                         AND julianday(?2) > julianday(?3))",
                params![job.lease_expires_at, job.lease_deadline_at, timestamp],
                |row| row.get(0),
            )?;
            if active != 1 || lease_current != 1 {
                bail!("Sui 预检任务租约已过期、达到最长处理期限或机器凭据已失效");
            }
        }
        ensure_report_matches_job(&job, adapter, report_input)?;
        let report = insert_or_get_report(&tx, report_input)?;
        if job.status == "completed" {
            if job.report_id.as_deref() != Some(&report.id) {
                bail!("已完成的 Sui 预检任务不能绑定不同报告");
            }
            tx.commit()?;
            return Ok((job, report));
        }
        let changed = tx.execute(
            "UPDATE task_sui_preflight_jobs
                SET status='completed', report_id=?1, completed_at=?2, updated_at=?2
              WHERE id=?3 AND status='leased' AND lease_token_hash=?4",
            params![
                report.id,
                timestamp,
                job.id,
                lease_token_hash(lease_token.trim())
            ],
        )?;
        if changed != 1 {
            bail!("Sui 预检任务完成发生并发冲突");
        }
        let completed = tx.query_row(
            &format!("{JOB_SELECT} WHERE id=?1"),
            params![job.id],
            job_from_row,
        )?;
        tx.commit()?;
        Ok((completed, report))
    }
}

fn ensure_report_matches_job(
    job: &SuiPreflightJob,
    adapter: &SuiPreflightAdapter,
    report: CreateSuiPreflightReport<'_>,
) -> Result<()> {
    if report.project_id.trim() != job.project_id
        || report.adapter_id.trim() != adapter.id
        || report.credential_version != adapter.credential_version
        || report.package_kind.trim() != job.package_kind
        || report.projection_package_id.trim() != job.projection_package_id
        || report.target_network.trim() != job.target_network
        || report.handoff_digest.trim() != job.handoff_digest
        || report.projection_digest.trim() != job.projection_digest
    {
        bail!("Sui 预检报告与租约任务绑定内容冲突");
    }
    Ok(())
}

fn new_lease_token() -> String {
    format!(
        "sui_preflight_lease_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn lease_token_hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn token_hint(value: &str) -> String {
    format!("...{}", &value[value.len().saturating_sub(6)..])
}

fn clean_reason(value: &str) -> String {
    value.trim().chars().take(500).collect()
}
