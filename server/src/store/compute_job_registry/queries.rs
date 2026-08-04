use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};

use super::{current_registered_job_on, ComputeJobRegistrationReceipt};

pub(super) fn list_current_jobs_on(
    conn: &Connection,
    consumer_account_id: &str,
    project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<ComputeJobRegistrationReceipt>> {
    let job_ids = if let Some(project_id) = project_id {
        let mut stmt = conn.prepare(
            "SELECT job_id
               FROM compute_jobs
              WHERE consumer_account_id=?1 AND project_id=?2
              ORDER BY recorded_at DESC, job_id ASC
              LIMIT ?3",
        )?;
        stmt.query_map(
            params![consumer_account_id, project_id, limit as i64],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?
    } else {
        let mut stmt = conn.prepare(
            "SELECT job_id
               FROM compute_jobs
              WHERE consumer_account_id=?1
              ORDER BY recorded_at DESC, job_id ASC
              LIMIT ?2",
        )?;
        stmt.query_map(params![consumer_account_id, limit as i64], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };

    job_ids
        .into_iter()
        .map(|job_id| {
            current_registered_job_on(conn, &job_id)?
                .ok_or_else(|| anyhow!("算力 Job 当前投影缺失"))
        })
        .collect()
}
